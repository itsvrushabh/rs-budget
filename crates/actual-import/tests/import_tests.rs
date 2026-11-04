//! Integration tests for financial statement format parsers (Issue #14).

use actual_core::{Cents, Transaction};
use actual_import::{
    CsvConfig, CsvParser, Deduplicator, ImportedTransaction, OfxParser, PayeeCleaner, QifParser,
};
use chrono::NaiveDate;

fn dollars(d: i64) -> Cents {
    Cents::from_cents(d * 100)
}

fn cents(c: i64) -> Cents {
    Cents::from_cents(c)
}

// ─── QIF Parsing ───────────────────────────────────────────────────────────────

#[test]
fn test_parse_qif_fixture() {
    let content = std::fs::read_to_string("../../fixtures/imports/sample.qif")
        .or_else(|_| std::fs::read_to_string("fixtures/imports/sample.qif"))
        .expect("read sample.qif fixture");

    let parser = QifParser::new();
    let txs = parser.parse(&content).expect("parse QIF");

    assert_eq!(txs.len(), 3);

    // 1. Trader Joe's: raw "POS DEBIT TRADER JOE'S #1234 NY" -> cleaned "TRADER JOE'S"
    let t1 = &txs[0];
    assert_eq!(t1.date, NaiveDate::from_ymd_opt(2024, 5, 15).unwrap());
    assert_eq!(t1.amount, cents(-4250));
    assert_eq!(t1.payee.as_deref(), Some("TRADER JOE'S"));
    assert_eq!(
        t1.imported_payee.as_deref(),
        Some("POS DEBIT TRADER JOE'S #1234 NY")
    );
    assert_eq!(t1.notes.as_deref(), Some("Tasty snacks"));
    assert!(t1.cleared);

    // 2. Paycheck: "DIRECT DEP ACME CORP" -> "ACME CORP"
    let t2 = &txs[1];
    assert_eq!(t2.date, NaiveDate::from_ymd_opt(2024, 5, 16).unwrap());
    assert_eq!(t2.amount, dollars(2500));
    assert_eq!(t2.payee.as_deref(), Some("ACME CORP"));
    assert_eq!(t2.notes.as_deref(), Some("Paycheck"));

    // 3. Split transaction: Target -$100 split into -$60 and -$40
    let t3 = &txs[2];
    assert_eq!(t3.date, NaiveDate::from_ymd_opt(2024, 5, 18).unwrap());
    assert_eq!(t3.amount, dollars(-100));
    assert_eq!(t3.payee.as_deref(), Some("TARGET"));
    assert_eq!(t3.subtransactions.len(), 2);
    assert_eq!(t3.subtransactions[0].amount, dollars(-60));
    assert_eq!(
        t3.subtransactions[0].category.as_deref(),
        Some("Food:Groceries")
    );
    assert_eq!(
        t3.subtransactions[0].notes.as_deref(),
        Some("Food supplies")
    );
    assert_eq!(t3.subtransactions[1].amount, dollars(-40));
    assert_eq!(t3.subtransactions[1].category.as_deref(), Some("Household"));
    assert_eq!(
        t3.subtransactions[1].notes.as_deref(),
        Some("Cleaning supplies")
    );
}

// ─── OFX SGML & XML Parsing ───────────────────────────────────────────────────

#[test]
fn test_parse_ofx_sgml_fixture() {
    let content = std::fs::read_to_string("../../fixtures/imports/sample.ofx")
        .or_else(|_| std::fs::read_to_string("fixtures/imports/sample.ofx"))
        .expect("read sample.ofx fixture");

    let parser = OfxParser::new();
    let txs = parser.parse(&content).expect("parse OFX");

    assert_eq!(txs.len(), 3);

    // 1. Coffee shop: "SQ *COFFEE SHOP" -> "COFFEE SHOP"
    let t1 = &txs[0];
    assert_eq!(t1.date, NaiveDate::from_ymd_opt(2024, 5, 10).unwrap());
    assert_eq!(t1.amount, cents(-3575));
    assert_eq!(t1.imported_id.as_deref(), Some("FITID-20240510-001"));
    assert_eq!(t1.payee.as_deref(), Some("COFFEE SHOP"));
    assert_eq!(t1.notes.as_deref(), Some("Morning Latte"));

    // 2. Direct dep salary
    let t2 = &txs[1];
    assert_eq!(t2.date, NaiveDate::from_ymd_opt(2024, 5, 15).unwrap());
    assert_eq!(t2.amount, dollars(1500));
    assert_eq!(t2.imported_id.as_deref(), Some("FITID-20240515-002"));
    assert_eq!(t2.payee.as_deref(), Some("EMPLOYER"));

    // 3. Netflix: "WWW.NETFLIX.COM CA" -> "NETFLIX.COM"
    let t3 = &txs[2];
    assert_eq!(t3.date, NaiveDate::from_ymd_opt(2024, 5, 18).unwrap());
    assert_eq!(t3.amount, cents(-1250));
    assert_eq!(t3.imported_id.as_deref(), Some("FITID-20240518-003"));
    assert_eq!(t3.payee.as_deref(), Some("NETFLIX.COM"));
}

#[test]
fn test_parse_ofx_xml_fixture() {
    let content = std::fs::read_to_string("../../fixtures/imports/sample_xml.ofx")
        .or_else(|_| std::fs::read_to_string("fixtures/imports/sample_xml.ofx"))
        .expect("read sample_xml.ofx fixture");

    let parser = OfxParser::new();
    let txs = parser.parse(&content).expect("parse XML OFX");

    assert_eq!(txs.len(), 1);
    let t = &txs[0];
    assert_eq!(t.date, NaiveDate::from_ymd_opt(2024, 5, 12).unwrap());
    assert_eq!(t.amount, cents(-8520));
    assert_eq!(t.imported_id.as_deref(), Some("XML-FITID-999"));
    assert_eq!(t.payee.as_deref(), Some("WHOLEFDS SOHO"));
}

// ─── CSV Parsing ───────────────────────────────────────────────────────────────

#[test]
fn test_parse_csv_signed_fixture() {
    let content = std::fs::read_to_string("../../fixtures/imports/sample_signed.csv")
        .or_else(|_| std::fs::read_to_string("fixtures/imports/sample_signed.csv"))
        .expect("read sample_signed.csv fixture");

    let config = CsvConfig {
        date_column: "Date".to_string(),
        payee_column: Some("Payee".to_string()),
        notes_column: Some("Memo".to_string()),
        amount_column: Some("Amount".to_string()),
        id_column: Some("TransactionID".to_string()),
        category_column: Some("Category".to_string()),
        ..Default::default()
    };

    let parser = CsvParser::new();
    let txs = parser.parse(&content, &config).expect("parse CSV");

    assert_eq!(txs.len(), 3);

    // 1. Uber Eats: "POS PURCHASE UBER EATS #882" -> "UBER EATS"
    let t1 = &txs[0];
    assert_eq!(t1.date, NaiveDate::from_ymd_opt(2024, 5, 1).unwrap());
    assert_eq!(t1.amount, cents(-2450));
    assert_eq!(t1.payee.as_deref(), Some("UBER EATS"));
    assert_eq!(t1.category.as_deref(), Some("Dining"));
    assert_eq!(t1.imported_id.as_deref(), Some("CSV-TX-001"));

    // 2. Paypal: "PAYPAL *GITHUB" -> "GITHUB"
    let t2 = &txs[1];
    assert_eq!(t2.date, NaiveDate::from_ymd_opt(2024, 5, 5).unwrap());
    assert_eq!(t2.amount, dollars(-10));
    assert_eq!(t2.payee.as_deref(), Some("GITHUB"));

    // 3. Consulting income: +$3,200
    let t3 = &txs[2];
    assert_eq!(t3.date, NaiveDate::from_ymd_opt(2024, 5, 15).unwrap());
    assert_eq!(t3.amount, dollars(3200));
}

#[test]
fn test_parse_csv_dual_columns_and_custom_delimiter() {
    let content = std::fs::read_to_string("../../fixtures/imports/sample_dual.csv")
        .or_else(|_| std::fs::read_to_string("fixtures/imports/sample_dual.csv"))
        .expect("read sample_dual.csv fixture");

    let config = CsvConfig {
        delimiter: b';',
        has_headers: true,
        date_column: "Transaction Date".to_string(),
        date_format: Some("%m/%d/%Y".to_string()),
        payee_column: Some("Description".to_string()),
        amount_column: None,
        outflow_column: Some("Outflow".to_string()),
        inflow_column: Some("Inflow".to_string()),
        ..Default::default()
    };

    let parser = CsvParser::new();
    let txs = parser
        .parse(&content, &config)
        .expect("parse dual column CSV");

    assert_eq!(txs.len(), 3);

    // 1. Outflow: CVS PHARMACY $15.75 -> -$15.75
    let t1 = &txs[0];
    assert_eq!(t1.date, NaiveDate::from_ymd_opt(2024, 5, 2).unwrap());
    assert_eq!(t1.amount, cents(-1575));
    assert_eq!(t1.payee.as_deref(), Some("CVS PHARMACY"));

    // 2. Inflow: Freelance +$750.00
    let t2 = &txs[1];
    assert_eq!(t2.date, NaiveDate::from_ymd_opt(2024, 5, 10).unwrap());
    assert_eq!(t2.amount, dollars(750));

    // 3. Outflow: Shell Oil $45.00 -> -$45.00
    let t3 = &txs[2];
    assert_eq!(t3.date, NaiveDate::from_ymd_opt(2024, 5, 14).unwrap());
    assert_eq!(t3.amount, dollars(-45));
}

// ─── Deduplication Tests ───────────────────────────────────────────────────────

#[test]
fn test_strict_and_proximity_deduplication() {
    let existing = vec![
        // Existing transaction with known imported_id
        {
            let mut tx = Transaction::new(
                "ex-1",
                "acc-1",
                NaiveDate::from_ymd_opt(2024, 5, 10).unwrap(),
                cents(-3575),
            );
            tx.imported_id = Some("FITID-20240510-001".to_string());
            tx
        },
        // Existing manual transaction without imported_id on May 15 for -$50.00
        Transaction::new(
            "ex-2",
            "acc-1",
            NaiveDate::from_ymd_opt(2024, 5, 15).unwrap(),
            dollars(-50),
        ),
    ];

    let incoming = vec![
        // 1. Strict duplicate: same FITID
        {
            let mut it = ImportedTransaction::new(
                NaiveDate::from_ymd_opt(2024, 5, 10).unwrap(),
                cents(-3575),
            );
            it.imported_id = Some("FITID-20240510-001".to_string());
            it
        },
        // 2. Proximity duplicate: May 17 (-$50.00) matches ex-2 on May 15 (2 days diff <= 3 days window)
        ImportedTransaction::new(NaiveDate::from_ymd_opt(2024, 5, 17).unwrap(), dollars(-50)),
        // 3. Unique transaction: May 20 for -$25.00
        ImportedTransaction::new(NaiveDate::from_ymd_opt(2024, 5, 20).unwrap(), dollars(-25)),
    ];

    let dedup = Deduplicator::new(3); // +/- 3 days window
    let result = dedup.deduplicate(incoming, &existing);

    // 2 duplicates, 1 unique
    assert_eq!(result.duplicates.len(), 2);
    assert_eq!(result.duplicates[0].1, "ex-1"); // matched strict ID
    assert_eq!(result.duplicates[1].1, "ex-2"); // matched proximity

    assert_eq!(result.unique.len(), 1);
    assert_eq!(result.unique[0].amount, dollars(-25));
}

// ─── Payee Cleaner Unit Tests ──────────────────────────────────────────────────

#[test]
fn test_payee_cleaner_patterns() {
    let cleaner = PayeeCleaner::new();

    let cases: [(&str, Option<&str>); 8] = [
        ("POS DEBIT STARBUCKS #08492 CA", Some("STARBUCKS")),
        (
            "ACH WITHDRAWAL ELECTRIC UTILITY CO",
            Some("ELECTRIC UTILITY CO"),
        ),
        ("CHECK #1024", None),
        ("TST* JOE'S PIZZA", Some("JOE'S PIZZA")),
        ("AMZN Mktp US* 987654321", Some("987654321")),
        ("WWW.BESTBUY.COM #999", Some("BESTBUY.COM")),
        ("SHELL OIL 800-555-0199", Some("SHELL OIL")),
        ("  WHOLE FOODS   MARKET   ", Some("WHOLE FOODS MARKET")),
    ];

    for (raw, expected) in cases {
        let cleaned = cleaner.clean(raw);
        assert_eq!(cleaned.as_deref(), expected, "failed for raw: '{raw}'");
    }
}

// ─── Model Conversion ──────────────────────────────────────────────────────────

#[test]
fn test_into_domain_transaction_conversion() {
    let mut item =
        ImportedTransaction::new(NaiveDate::from_ymd_opt(2024, 5, 18).unwrap(), dollars(-100));
    item.payee = Some("TARGET".to_string());
    item.imported_id = Some("TARGET-REF-1".to_string());
    item.notes = Some("Household & Groceries".to_string());
    item.subtransactions.push(actual_import::ImportedSplit {
        amount: dollars(-60),
        category: Some("Groceries".to_string()),
        notes: Some("Food".to_string()),
    });
    item.subtransactions.push(actual_import::ImportedSplit {
        amount: dollars(-40),
        category: Some("Household".to_string()),
        notes: Some("Cleaning".to_string()),
    });

    let domain_tx = item.into_domain_transaction("acc-test");

    assert_eq!(domain_tx.account_id, "acc-test");
    assert_eq!(domain_tx.amount, dollars(-100));
    assert!(domain_tx.is_parent);
    assert_eq!(domain_tx.subtransactions.len(), 2);
    assert_eq!(domain_tx.subtransactions[0].amount, dollars(-60));
    assert_eq!(domain_tx.subtransactions[1].amount, dollars(-40));
    assert_eq!(
        domain_tx.subtransactions[0].parent_id,
        Some(domain_tx.id.clone())
    );
}
