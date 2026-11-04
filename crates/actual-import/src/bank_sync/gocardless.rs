//! GoCardless (Nordigen) Open Banking API client and response parser.
//!
//! GoCardless provides European and UK PSD2 / Open Banking data aggregation.
//! Authentication utilizes developer Secret ID & Secret Key exchanged for JWT tokens.
//! Account access flows through End-User Agreements and Requisitions.

use crate::bank_sync::types::BankSyncAccount;
use crate::error::ImportError;
use crate::models::ImportedTransaction;
use actual_core::Cents;
use chrono::{NaiveDate, Utc};
use serde::{Deserialize, Serialize};

/// Financial institution metadata from GoCardless directory.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Institution {
    pub id: String,
    pub name: String,
    pub bic: Option<String>,
    pub transaction_total_days: Option<String>,
    #[serde(default)]
    pub countries: Vec<String>,
    pub logo: Option<String>,
}

/// Token exchange response from `/token/new/` or `/token/refresh/`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenResponse {
    pub access: String,
    pub access_expires: Option<i64>,
    pub refresh: Option<String>,
    pub refresh_expires: Option<i64>,
}

/// End-user agreement response from `/agreements/enduser/`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgreementResponse {
    pub id: String,
    pub created: Option<String>,
    pub max_historical_days: Option<i64>,
    pub access_valid_for_days: Option<i64>,
    pub institution_id: Option<String>,
}

/// Requisition response from `/requisitions/` creation and status check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequisitionResponse {
    pub id: String,
    pub status: String,
    pub link: Option<String>,
    #[serde(default)]
    pub accounts: Vec<String>,
    pub reference: Option<String>,
}

/// Raw balance entry from GoCardless `/balances/`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoCardlessBalanceItem {
    #[serde(rename = "balanceAmount")]
    pub balance_amount: GoCardlessAmount,
    #[serde(rename = "balanceType")]
    pub balance_type: Option<String>,
}

/// Amount and currency descriptor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoCardlessAmount {
    pub amount: String,
    pub currency: String,
}

/// Balances container response from `/accounts/{id}/balances/`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoCardlessBalancesResponse {
    #[serde(default)]
    pub balances: Vec<GoCardlessBalanceItem>,
}

/// Account metadata from `/accounts/{id}/`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoCardlessAccountMetadata {
    pub id: String,
    pub created: Option<String>,
    pub name: Option<String>,
    pub currency: Option<String>,
    pub iban: Option<String>,
    #[serde(rename = "institution_id")]
    pub institution_id: Option<String>,
}

/// Single transaction item from GoCardless booked or pending lists.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoCardlessTransactionItem {
    #[serde(rename = "transactionId")]
    pub transaction_id: Option<String>,
    #[serde(rename = "internalTransactionId")]
    pub internal_transaction_id: Option<String>,
    #[serde(rename = "bookingDate")]
    pub booking_date: Option<String>,
    #[serde(rename = "valueDate")]
    pub value_date: Option<String>,
    #[serde(rename = "transactionAmount")]
    pub transaction_amount: GoCardlessAmount,
    #[serde(rename = "creditorName")]
    pub creditor_name: Option<String>,
    #[serde(rename = "debtorName")]
    pub debtor_name: Option<String>,
    #[serde(rename = "remittanceInformationUnstructured")]
    pub remittance_info: Option<String>,
    #[serde(rename = "proprietaryBankTransactionCode")]
    pub bank_code: Option<String>,
}

/// Transactions bucket containing booked and pending lists.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoCardlessTransactionsBucket {
    #[serde(default)]
    pub booked: Vec<GoCardlessTransactionItem>,
    #[serde(default)]
    pub pending: Vec<GoCardlessTransactionItem>,
}

/// Transactions response container from `/accounts/{id}/transactions/`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoCardlessTransactionsResponse {
    pub transactions: GoCardlessTransactionsBucket,
}

/// GoCardless (Nordigen) API client.
#[derive(Debug, Clone)]
pub struct GoCardlessClient {
    /// Base API URL (defaults to `https://bankaccountdata.gocardless.com/api/v2`).
    pub base_url: String,
    /// GoCardless Developer Secret ID.
    pub secret_id: String,
    /// GoCardless Developer Secret Key.
    pub secret_key: String,
    /// Cached access JWT token.
    pub access_token: Option<String>,
    /// Cached refresh JWT token.
    pub refresh_token: Option<String>,
    /// Expiration timestamp in seconds.
    pub token_expires_at: Option<i64>,
}

impl GoCardlessClient {
    pub const DEFAULT_BASE_URL: &'static str = "https://bankaccountdata.gocardless.com/api/v2";

    /// Constructs a new client with default base URL and API keys.
    #[must_use]
    pub fn new(secret_id: impl Into<String>, secret_key: impl Into<String>) -> Self {
        Self {
            base_url: Self::DEFAULT_BASE_URL.to_string(),
            secret_id: secret_id.into(),
            secret_key: secret_key.into(),
            access_token: None,
            refresh_token: None,
            token_expires_at: None,
        }
    }

    /// Sets a custom base URL (useful for mock testing or staging environments).
    #[must_use]
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into().trim_end_matches('/').to_string();
        self
    }

    /// Obtains a new JWT access and refresh token pair using secret ID and key.
    pub fn authenticate(&mut self) -> Result<(), ImportError> {
        let endpoint = format!("{}/token/new/", self.base_url);
        let payload = serde_json::json!({
            "secret_id": self.secret_id,
            "secret_key": self.secret_key,
        });

        let resp = ureq::post(&endpoint)
            .send_json(payload)
            .map_err(|e| ImportError::Network(format!("GoCardless auth failed: {e}")))?;

        let token_resp: TokenResponse = resp.into_json()?;
        let now_sec = Utc::now().timestamp();
        self.access_token = Some(token_resp.access);
        self.refresh_token = token_resp.refresh;
        self.token_expires_at = token_resp.access_expires.map(|exp| now_sec + exp);

        Ok(())
    }

    /// Refreshes the active access token using the stored refresh token.
    pub fn refresh_access_token(&mut self) -> Result<(), ImportError> {
        let refresh = self
            .refresh_token
            .as_ref()
            .ok_or_else(|| ImportError::BankSync {
                provider: "gocardless".into(),
                message: "No refresh token available to renew session".into(),
            })?;

        let endpoint = format!("{}/token/refresh/", self.base_url);
        let payload = serde_json::json!({
            "refresh": refresh,
        });

        let resp = ureq::post(&endpoint)
            .send_json(payload)
            .map_err(|e| ImportError::Network(format!("GoCardless token refresh failed: {e}")))?;

        let token_resp: TokenResponse = resp.into_json()?;
        let now_sec = Utc::now().timestamp();
        self.access_token = Some(token_resp.access);
        self.token_expires_at = token_resp.access_expires.map(|exp| now_sec + exp);

        Ok(())
    }

    /// Ensures an active access token is present, refreshing if expired.
    pub fn ensure_token(&mut self) -> Result<&str, ImportError> {
        let now_sec = Utc::now().timestamp();
        let needs_refresh = match self.token_expires_at {
            Some(exp) => now_sec >= exp - 60, // Refresh 60s early
            None => self.access_token.is_none(),
        };

        if needs_refresh {
            if self.refresh_token.is_some() {
                if self.refresh_access_token().is_err() {
                    self.authenticate()?;
                }
            } else {
                self.authenticate()?;
            }
        }

        self.access_token
            .as_deref()
            .ok_or_else(|| ImportError::BankSync {
                provider: "gocardless".into(),
                message: "Failed to acquire valid access token".into(),
            })
    }

    /// Queries the directory of supported institutions for a 2-letter ISO country code.
    pub fn list_institutions(
        &mut self,
        country_code: &str,
    ) -> Result<Vec<Institution>, ImportError> {
        let token = self.ensure_token()?.to_string();
        let endpoint = format!("{}/institutions/", self.base_url);
        let resp = ureq::get(&endpoint)
            .set("Authorization", &format!("Bearer {token}"))
            .query("country", country_code)
            .call()
            .map_err(|e| ImportError::Network(format!("Failed to list institutions: {e}")))?;

        let json_str = resp.into_string()?;
        Self::parse_institutions_response(&json_str)
    }

    /// Creates an end-user agreement and requisition, returning the authorization link.
    pub fn create_requisition_flow(
        &mut self,
        institution_id: &str,
        redirect_url: &str,
        reference: &str,
    ) -> Result<RequisitionResponse, ImportError> {
        let token = self.ensure_token()?.to_string();

        // 1. Create End-User Agreement
        let agreement_endpoint = format!("{}/agreements/enduser/", self.base_url);
        let agreement_payload = serde_json::json!({
            "institution_id": institution_id,
            "max_historical_days": 90,
            "access_valid_for_days": 90
        });

        let agreement_resp = ureq::post(&agreement_endpoint)
            .set("Authorization", &format!("Bearer {token}"))
            .send_json(agreement_payload)
            .map_err(|e| ImportError::Network(format!("Failed to create agreement: {e}")))?;

        let agreement: AgreementResponse = agreement_resp.into_json()?;

        // 2. Create Requisition
        let req_endpoint = format!("{}/requisitions/", self.base_url);
        let req_payload = serde_json::json!({
            "redirect": redirect_url,
            "institution_id": institution_id,
            "agreement": agreement.id,
            "reference": reference,
        });

        let req_resp = ureq::post(&req_endpoint)
            .set("Authorization", &format!("Bearer {token}"))
            .send_json(req_payload)
            .map_err(|e| ImportError::Network(format!("Failed to create requisition: {e}")))?;

        let requisition: RequisitionResponse = req_resp.into_json()?;
        Ok(requisition)
    }

    /// Fetches the latest requisition status and linked account IDs.
    pub fn get_requisition(
        &mut self,
        requisition_id: &str,
    ) -> Result<RequisitionResponse, ImportError> {
        let token = self.ensure_token()?.to_string();
        let endpoint = format!("{}/requisitions/{requisition_id}/", self.base_url);
        let resp = ureq::get(&endpoint)
            .set("Authorization", &format!("Bearer {token}"))
            .call()
            .map_err(|e| ImportError::Network(format!("Failed to get requisition: {e}")))?;

        let requisition: RequisitionResponse = resp.into_json()?;
        Ok(requisition)
    }

    /// Fetches metadata and current balances for an authorized external account ID.
    pub fn fetch_account_details(
        &mut self,
        account_id: &str,
    ) -> Result<BankSyncAccount, ImportError> {
        let token = self.ensure_token()?.to_string();

        // Metadata
        let meta_endpoint = format!("{}/accounts/{account_id}/", self.base_url);
        let meta_resp = ureq::get(&meta_endpoint)
            .set("Authorization", &format!("Bearer {token}"))
            .call()
            .map_err(|e| ImportError::Network(format!("Failed to fetch account metadata: {e}")))?;
        let meta_json = meta_resp.into_string()?;

        // Balances
        let bal_endpoint = format!("{}/accounts/{account_id}/balances/", self.base_url);
        let bal_resp = ureq::get(&bal_endpoint)
            .set("Authorization", &format!("Bearer {token}"))
            .call()
            .map_err(|e| ImportError::Network(format!("Failed to fetch account balances: {e}")))?;
        let bal_json = bal_resp.into_string()?;

        Self::parse_account_details_and_balances(account_id, &meta_json, &bal_json)
    }

    /// Fetches booked and pending transactions for an authorized account.
    pub fn fetch_transactions(
        &mut self,
        account_id: &str,
        date_from: Option<NaiveDate>,
    ) -> Result<Vec<ImportedTransaction>, ImportError> {
        let token = self.ensure_token()?.to_string();
        let endpoint = format!("{}/accounts/{account_id}/transactions/", self.base_url);
        let mut req = ureq::get(&endpoint).set("Authorization", &format!("Bearer {token}"));

        if let Some(d) = date_from {
            req = req.query("date_from", &d.format("%Y-%m-%d").to_string());
        }

        let resp = req.call().map_err(|e| {
            ImportError::Network(format!("Failed to fetch account transactions: {e}"))
        })?;

        let json_str = resp.into_string()?;
        Self::parse_transactions_response(&json_str)
    }

    /// Parses directory list of institutions JSON.
    pub fn parse_institutions_response(json_str: &str) -> Result<Vec<Institution>, ImportError> {
        let institutions: Vec<Institution> = serde_json::from_str(json_str)?;
        Ok(institutions)
    }

    /// Parses account details and balances JSON into a unified `BankSyncAccount`.
    pub fn parse_account_details_and_balances(
        account_id: &str,
        meta_json: &str,
        bal_json: &str,
    ) -> Result<BankSyncAccount, ImportError> {
        let meta: GoCardlessAccountMetadata = serde_json::from_str(meta_json)?;
        let balances: GoCardlessBalancesResponse = serde_json::from_str(bal_json)?;

        let currency = meta.currency.unwrap_or_else(|| "EUR".to_string());
        let name = meta
            .name
            .unwrap_or_else(|| format!("Account {}", meta.iban.as_deref().unwrap_or(account_id)));

        let mut current_balance = None;
        let mut available_balance = None;

        for item in balances.balances {
            let cents = item.balance_amount.amount.parse::<Cents>().ok();
            match item.balance_type.as_deref() {
                Some("interimAvailable") | Some("available") => available_balance = cents,
                Some("closingBooked") | Some("expected") | Some("interimBooked") => {
                    if current_balance.is_none() {
                        current_balance = cents;
                    }
                }
                _ => {
                    if current_balance.is_none() {
                        current_balance = cents;
                    }
                }
            }
        }

        Ok(BankSyncAccount {
            external_id: account_id.to_string(),
            name,
            currency,
            current_balance,
            available_balance,
            institution_name: meta.institution_id,
            account_number_masked: meta.iban,
        })
    }

    /// Parses GoCardless transactions JSON into normalized `ImportedTransaction`s.
    pub fn parse_transactions_response(
        json_str: &str,
    ) -> Result<Vec<ImportedTransaction>, ImportError> {
        let resp: GoCardlessTransactionsResponse = serde_json::from_str(json_str)?;
        let mut results = Vec::new();

        let process_item = |item: GoCardlessTransactionItem,
                            is_cleared: bool|
         -> Result<ImportedTransaction, ImportError> {
            let amount = item
                .transaction_amount
                .amount
                .parse::<Cents>()
                .map_err(|_| {
                    ImportError::InvalidAmount(
                        item.transaction_amount.amount.clone(),
                        "GoCardless amount parse error".into(),
                    )
                })?;

            let date_str = item
                .booking_date
                .as_deref()
                .or(item.value_date.as_deref())
                .unwrap_or("1970-01-01");

            let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                .unwrap_or_else(|_| NaiveDate::from_ymd_opt(1970, 1, 1).unwrap());

            let payee_name = item
                .creditor_name
                .or(item.debtor_name)
                .or_else(|| item.remittance_info.clone());

            let external_id = item
                .transaction_id
                .or(item.internal_transaction_id)
                .unwrap_or_else(|| {
                    format!(
                        "{}_{}_{}",
                        date.format("%Y%m%d"),
                        amount.as_cents(),
                        item.remittance_info.as_deref().unwrap_or("tx")
                    )
                });

            let mut tx = ImportedTransaction::new(date, amount);
            tx.payee = payee_name.clone();
            tx.imported_payee = payee_name;
            tx.notes = item.remittance_info;
            tx.imported_id = Some(external_id);
            tx.cleared = is_cleared;

            Ok(tx)
        };

        for item in resp.transactions.booked {
            results.push(process_item(item, true)?);
        }

        for item in resp.transactions.pending {
            results.push(process_item(item, false)?);
        }

        Ok(results)
    }
}
