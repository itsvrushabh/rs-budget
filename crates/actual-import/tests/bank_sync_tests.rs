//! Comprehensive unit and integration test suite for automated bank synchronization
//! (SimpleFIN Bridge, GoCardless / Nordigen, credential encryption, and delta deduplication).

use actual_core::{Cents, Transaction};
use actual_crypto::Key;
use actual_import::bank_sync::{
    BankProviderType, BankSyncManager, GoCardlessClient, SimpleFinClient,
};
use actual_import::{ImportError, PayeeCleaner};
use chrono::NaiveDate;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

#[test]
fn test_simplefin_access_url_parsing() {
    let url = "https://demo_user:secret_pass_123@beta-bridge.simplefin.org/simplefin";
    let (base_url, user, pass) = SimpleFinClient::parse_access_url(url).expect("valid access url");

    assert_eq!(base_url, "https://beta-bridge.simplefin.org/simplefin");
    assert_eq!(user, "demo_user");
    assert_eq!(pass, "secret_pass_123");

    // Trailing slash handling
    let url_slash = "https://user:pass@bridge.example.com/api/v1/";
    let (base_url_s, _, _) =
        SimpleFinClient::parse_access_url(url_slash).expect("valid access url");
    assert_eq!(base_url_s, "https://bridge.example.com/api/v1");

    // Invalid scheme
    assert!(matches!(
        SimpleFinClient::parse_access_url("ftp://user:pass@example.com"),
        Err(ImportError::BankSync { .. })
    ));

    // Missing auth delimiter
    assert!(matches!(
        SimpleFinClient::parse_access_url("https://example.com/simplefin"),
        Err(ImportError::BankSync { .. })
    ));
}

#[test]
fn test_simplefin_setup_token_invalid_base64() {
    let invalid_token = "!!!not-valid-base64???";
    let err = SimpleFinClient::claim_setup_token(invalid_token).unwrap_err();
    assert!(matches!(err, ImportError::BankSync { .. }));
}

#[test]
fn test_simplefin_parse_accounts_and_transactions() {
    let json_data = r#"{
        "errors": [],
        "accounts": [
            {
                "id": "act_chase_checking_1",
                "name": "Premier Checking",
                "currency": "USD",
                "balance": "4520.50",
                "available-balance": "4400.00",
                "balance-date": 1715731200,
                "org": {
                    "name": "JPMorgan Chase Bank",
                    "domain": "chase.com"
                },
                "transactions": [
                    {
                        "id": "sfin_tx_101",
                        "posted": 1715731200,
                        "amount": "-45.50",
                        "description": "WHOLEFDS SOMA 1024",
                        "payee": "Whole Foods Market",
                        "memo": "Groceries"
                    },
                    {
                        "id": "sfin_tx_102",
                        "posted": 1715644800,
                        "amount": "2500.00",
                        "description": "DIRECT DEP ACME CORP",
                        "payee": null,
                        "memo": "Payroll"
                    }
                ]
            }
        ]
    }"#;

    let parsed = SimpleFinClient::parse_accounts_response(json_data).expect("valid simplefin json");
    assert_eq!(parsed.len(), 1);

    let (acc, txs) = &parsed[0];
    assert_eq!(acc.external_id, "act_chase_checking_1");
    assert_eq!(acc.name, "Premier Checking");
    assert_eq!(acc.currency, "USD");
    assert_eq!(acc.current_balance, Some(Cents::from_cents(452050)));
    assert_eq!(acc.available_balance, Some(Cents::from_cents(440000)));
    assert_eq!(acc.institution_name.as_deref(), Some("JPMorgan Chase Bank"));

    assert_eq!(txs.len(), 2);
    // Outflow tx
    assert_eq!(txs[0].amount, Cents::from_cents(-4550));
    assert_eq!(txs[0].payee.as_deref(), Some("Whole Foods Market"));
    assert_eq!(txs[0].imported_id.as_deref(), Some("sfin_tx_101"));
    assert!(txs[0].cleared);

    // Inflow tx with null payee fallback to description
    assert_eq!(txs[1].amount, Cents::from_cents(250000));
    assert_eq!(txs[1].payee.as_deref(), Some("DIRECT DEP ACME CORP"));
    assert_eq!(txs[1].imported_id.as_deref(), Some("sfin_tx_102"));
}

#[test]
fn test_simplefin_api_error_response() {
    let json_err = r#"{
        "errors": ["Claim expired", "Authentication failure"],
        "accounts": []
    }"#;

    let res = SimpleFinClient::parse_accounts_response(json_err);
    match res {
        Err(ImportError::BankSync { provider, message }) => {
            assert_eq!(provider, "simplefin");
            assert!(message.contains("Claim expired"));
        }
        _ => panic!("Expected BankSync error with error messages"),
    }
}

#[test]
fn test_gocardless_parse_institutions() {
    let json_data = r#"[
        {
            "id": "SANDBOXFINANCE_SAPI0000",
            "name": "Sandbox Finance UK",
            "bic": "SANDGB2L",
            "transaction_total_days": "90",
            "countries": ["GB"],
            "logo": "https://cdn.nordigen.com/logos/sandbox.png"
        },
        {
            "id": "REVOLUT_REVOGB21",
            "name": "Revolut",
            "bic": "REVOGB21",
            "transaction_total_days": "730",
            "countries": ["GB", "DE", "FR"]
        }
    ]"#;

    let institutions =
        GoCardlessClient::parse_institutions_response(json_data).expect("valid institutions json");
    assert_eq!(institutions.len(), 2);
    assert_eq!(institutions[0].id, "SANDBOXFINANCE_SAPI0000");
    assert_eq!(institutions[0].countries, vec!["GB"]);
    assert_eq!(institutions[1].name, "Revolut");
}

#[test]
fn test_gocardless_parse_account_details_and_balances() {
    let meta_json = r#"{
        "id": "gc_acc_9876",
        "created": "2024-01-01T00:00:00Z",
        "name": "Personal Main Euro",
        "currency": "EUR",
        "iban": "DE89370400440532013000",
        "institution_id": "DEUTSCHE_BANK_DEUTDEDD"
    }"#;

    let bal_json = r#"{
        "balances": [
            {
                "balanceAmount": {
                    "amount": "3140.25",
                    "currency": "EUR"
                },
                "balanceType": "closingBooked"
            },
            {
                "balanceAmount": {
                    "amount": "2900.00",
                    "currency": "EUR"
                },
                "balanceType": "interimAvailable"
            }
        ]
    }"#;

    let acc =
        GoCardlessClient::parse_account_details_and_balances("gc_acc_9876", meta_json, bal_json)
            .expect("valid account details and balances");

    assert_eq!(acc.external_id, "gc_acc_9876");
    assert_eq!(acc.name, "Personal Main Euro");
    assert_eq!(acc.currency, "EUR");
    assert_eq!(acc.current_balance, Some(Cents::from_cents(314025)));
    assert_eq!(acc.available_balance, Some(Cents::from_cents(290000)));
    assert_eq!(
        acc.institution_name.as_deref(),
        Some("DEUTSCHE_BANK_DEUTDEDD")
    );
    assert_eq!(
        acc.account_number_masked.as_deref(),
        Some("DE89370400440532013000")
    );
}

#[test]
fn test_gocardless_parse_transactions() {
    let tx_json = r#"{
        "transactions": {
            "booked": [
                {
                    "transactionId": "gc_tx_001",
                    "bookingDate": "2024-05-10",
                    "valueDate": "2024-05-10",
                    "transactionAmount": {
                        "amount": "-32.80",
                        "currency": "EUR"
                    },
                    "creditorName": "REWE Supermarkt",
                    "remittanceInformationUnstructured": "KARTENZAHLUNG REWE"
                }
            ],
            "pending": [
                {
                    "transactionId": "gc_tx_pending_002",
                    "bookingDate": "2024-05-12",
                    "transactionAmount": {
                        "amount": "-15.00",
                        "currency": "EUR"
                    },
                    "creditorName": "CAFE BERLIN",
                    "remittanceInformationUnstructured": "PENDING RESERVATION"
                }
            ]
        }
    }"#;

    let txs = GoCardlessClient::parse_transactions_response(tx_json).expect("valid gc txs");
    assert_eq!(txs.len(), 2);

    // Booked transaction
    assert_eq!(txs[0].date, NaiveDate::from_ymd_opt(2024, 5, 10).unwrap());
    assert_eq!(txs[0].amount, Cents::from_cents(-3280));
    assert_eq!(txs[0].payee.as_deref(), Some("REWE Supermarkt"));
    assert_eq!(txs[0].imported_id.as_deref(), Some("gc_tx_001"));
    assert!(txs[0].cleared);

    // Pending transaction
    assert_eq!(txs[1].date, NaiveDate::from_ymd_opt(2024, 5, 12).unwrap());
    assert_eq!(txs[1].amount, Cents::from_cents(-1500));
    assert_eq!(txs[1].payee.as_deref(), Some("CAFE BERLIN"));
    assert_eq!(txs[1].imported_id.as_deref(), Some("gc_tx_pending_002"));
    assert!(!txs[1].cleared);
}

#[test]
fn test_bank_sync_manager_account_linking_and_unlinking() {
    let mut mgr = BankSyncManager::new();
    assert_eq!(mgr.linked_accounts().len(), 0);

    mgr.link_account(
        "act-local-checking",
        "ext-chase-123",
        BankProviderType::SimpleFin,
        Some("Chase Checking".into()),
        Some("USD".into()),
    );

    assert_eq!(mgr.linked_accounts().len(), 1);
    let linked = mgr
        .get_linked_account("act-local-checking")
        .expect("linked account");
    assert_eq!(linked.external_account_id, "ext-chase-123");
    assert_eq!(linked.provider, BankProviderType::SimpleFin);

    // Re-link updates existing entry
    mgr.link_account(
        "act-local-checking",
        "ext-chase-updated-456",
        BankProviderType::SimpleFin,
        Some("Updated Name".into()),
        Some("USD".into()),
    );
    assert_eq!(mgr.linked_accounts().len(), 1);
    assert_eq!(
        mgr.get_linked_account("act-local-checking")
            .unwrap()
            .external_account_id,
        "ext-chase-updated-456"
    );

    // Unlink
    let unlinked = mgr.unlink_account("act-local-checking");
    assert!(unlinked);
    assert_eq!(mgr.linked_accounts().len(), 0);
}

#[test]
fn test_bank_sync_credential_encryption_roundtrip() {
    let mut mgr = BankSyncManager::new();
    mgr.set_simplefin_access_url("https://user1:pass1@beta-bridge.simplefin.org/simplefin");
    mgr.set_gocardless_credentials("sec-id-xyz", "sec-key-abc");
    mgr.link_account(
        "acc-1",
        "ext-1",
        BankProviderType::SimpleFin,
        Some("My Bank".into()),
        Some("USD".into()),
    );

    let salt = "budget-salt-12345";
    let key = Key::from_password(None, "user-super-secret-password-123", salt).expect("derive key");

    // Export encrypted
    let encrypted_payload = mgr.export_encrypted(&key).expect("encrypt bank config");
    assert!(!encrypted_payload.contains("user-super-secret-password"));
    assert!(!encrypted_payload.contains("sec-key-abc"));

    // Restore into fresh manager
    let mut restored_mgr = BankSyncManager::new();
    restored_mgr
        .import_encrypted(&encrypted_payload, &key)
        .expect("decrypt bank config");

    assert_eq!(restored_mgr.linked_accounts().len(), 1);
    assert_eq!(restored_mgr.linked_accounts()[0].actual_account_id, "acc-1");

    // Decrypting with wrong key must fail
    let wrong_key = Key::from_password(None, "wrong-password", salt).expect("derive wrong key");
    assert!(restored_mgr
        .import_encrypted(&encrypted_payload, &wrong_key)
        .is_err());
}

#[test]
fn test_bank_sync_delta_deduplication_and_payee_cleaning() {
    let cleaner = PayeeCleaner::new();

    // Create existing transactions in account
    let mut existing_tx = Transaction::new(
        "tx-exist-1",
        "acc-main",
        NaiveDate::from_ymd_opt(2024, 5, 10).unwrap(),
        Cents::from_cents(-4550),
    );
    existing_tx.imported_id = Some("ext-tx-100".into());

    let existing_tx_approx = Transaction::new(
        "tx-exist-2",
        "acc-main",
        NaiveDate::from_ymd_opt(2024, 5, 15).unwrap(),
        Cents::from_cents(-2500),
    );

    let existing = vec![existing_tx, existing_tx_approx];

    // Incoming batch with:
    // 1. Exact match by imported_id ("ext-tx-100") -> duplicate
    // 2. Proximity match (-2500 on 2024-05-16, 1 day diff) -> duplicate
    // 3. New unique transaction with dirty payee needing cleaning
    let mut incoming_exact = actual_import::ImportedTransaction::new(
        NaiveDate::from_ymd_opt(2024, 5, 10).unwrap(),
        Cents::from_cents(-4550),
    );
    incoming_exact.imported_id = Some("ext-tx-100".into());

    let incoming_prox = actual_import::ImportedTransaction::new(
        NaiveDate::from_ymd_opt(2024, 5, 16).unwrap(),
        Cents::from_cents(-2500),
    );

    let mut incoming_new = actual_import::ImportedTransaction::new(
        NaiveDate::from_ymd_opt(2024, 5, 18).unwrap(),
        Cents::from_cents(-1299),
    );
    incoming_new.imported_payee = Some("POS DEBIT STARBUCKS #08492 CA".into());
    incoming_new.imported_id = Some("ext-tx-300".into());

    let incoming = vec![incoming_exact, incoming_prox, incoming_new];

    let result = BankSyncManager::process_and_deduplicate(incoming, &existing, Some(&cleaner));

    assert_eq!(result.duplicate_count, 2);
    assert_eq!(result.imported_transactions.len(), 1);

    let unique_tx = &result.imported_transactions[0];
    assert_eq!(unique_tx.imported_id.as_deref(), Some("ext-tx-300"));
    assert_eq!(unique_tx.amount, Cents::from_cents(-1299));
    // Cleaned payee should have bank prefixes, suffixes, and state abbreviation stripped
    assert_eq!(unique_tx.payee.as_deref(), Some("STARBUCKS"));
    assert_eq!(
        result.latest_transaction_date,
        Some(NaiveDate::from_ymd_opt(2024, 5, 18).unwrap())
    );
}

#[test]
fn test_end_to_end_mock_http_simplefin_sync() {
    // Bind mock HTTP server on local random free port
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local tcp");
    let addr = listener.local_addr().expect("local addr");
    let port = addr.port();

    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept connection");
        let mut buffer = [0u8; 2048];
        let bytes_read = stream.read(&mut buffer).expect("read request");
        let req_str = String::from_utf8_lossy(&buffer[..bytes_read]);

        assert!(req_str.starts_with("GET /accounts"));
        // Verify Basic Auth header was delivered
        assert!(req_str.contains("Authorization: Basic "));

        let body = r#"{
            "errors": [],
            "accounts": [
                {
                    "id": "act-live-001",
                    "name": "Live Checking",
                    "currency": "USD",
                    "balance": "950.00",
                    "transactions": [
                        {
                            "id": "mock_tx_777",
                            "posted": 1715731200,
                            "amount": "-50.00",
                            "description": "TARGET STORE #001"
                        }
                    ]
                }
            ]
        }"#;

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .expect("write response");
    });

    let mock_base = format!("http://127.0.0.1:{port}");
    let client = SimpleFinClient::new(&mock_base, "testuser", "testpass");
    let results = client
        .fetch_accounts_and_transactions(None, None)
        .expect("successful mock sync");

    server_handle.join().expect("server thread finished");

    assert_eq!(results.len(), 1);
    let (acc, txs) = &results[0];
    assert_eq!(acc.external_id, "act-live-001");
    assert_eq!(acc.current_balance, Some(Cents::from_cents(95000)));
    assert_eq!(txs.len(), 1);
    assert_eq!(txs[0].imported_id.as_deref(), Some("mock_tx_777"));
    assert_eq!(txs[0].amount, Cents::from_cents(-5000));
}
