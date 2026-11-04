//! SimpleFIN Bridge protocol client and response parser.
//!
//! SimpleFIN provides standardized access to North American financial institutions.
//! Authentication utilizes a one-time setup token exchanged for an access URL containing
//! HTTP Basic Authentication credentials.

use crate::bank_sync::types::BankSyncAccount;
use crate::error::ImportError;
use crate::models::ImportedTransaction;
use actual_core::Cents;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use chrono::DateTime;
use serde::{Deserialize, Serialize};

/// Raw account structure returned by SimpleFIN Bridge API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleFinAccount {
    pub id: String,
    pub name: String,
    pub currency: String,
    pub balance: String,
    #[serde(rename = "available-balance")]
    pub available_balance: Option<String>,
    #[serde(rename = "balance-date")]
    pub balance_date: Option<i64>,
    #[serde(default)]
    pub transactions: Vec<SimpleFinTransaction>,
    pub org: Option<SimpleFinOrg>,
}

/// Organization / Financial Institution metadata from SimpleFIN.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleFinOrg {
    pub name: Option<String>,
    pub domain: Option<String>,
    #[serde(rename = "sfin-url")]
    pub sfin_url: Option<String>,
}

/// Raw transaction item returned by SimpleFIN Bridge API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleFinTransaction {
    pub id: String,
    pub posted: i64,
    pub amount: String,
    pub description: Option<String>,
    pub payee: Option<String>,
    pub memo: Option<String>,
    pub transacted_at: Option<i64>,
}

/// Top-level response container from SimpleFIN Bridge API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleFinResponse {
    #[serde(default)]
    pub errors: Vec<String>,
    #[serde(default)]
    pub accounts: Vec<SimpleFinAccount>,
}

/// SimpleFIN Bridge API client.
#[derive(Debug, Clone)]
pub struct SimpleFinClient {
    /// Base service endpoint without credentials (e.g., "https://beta-bridge.simplefin.org/simplefin").
    pub base_url: String,
    /// HTTP Basic Auth username.
    pub username: String,
    /// HTTP Basic Auth password.
    pub password: String,
}

impl SimpleFinClient {
    /// Constructs a client directly from username, password, and base endpoint URL.
    #[must_use]
    pub fn new(
        base_url: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            username: username.into(),
            password: password.into(),
        }
    }

    /// Creates a client by parsing a full SimpleFIN access URL (`https://<user>:<pass>@<host>/<path>`).
    pub fn from_access_url(access_url: &str) -> Result<Self, ImportError> {
        let (base_url, username, password) = Self::parse_access_url(access_url)?;
        Ok(Self::new(base_url, username, password))
    }

    /// Exchanges a base64-encoded SimpleFIN setup token for an access URL.
    pub fn claim_setup_token(token: &str) -> Result<String, ImportError> {
        let clean_token = token.trim();
        let decoded_bytes =
            BASE64_STANDARD
                .decode(clean_token)
                .map_err(|e| ImportError::BankSync {
                    provider: "simplefin".into(),
                    message: format!("Failed to decode base64 setup token: {e}"),
                })?;

        let claim_url = String::from_utf8(decoded_bytes).map_err(|e| ImportError::BankSync {
            provider: "simplefin".into(),
            message: format!("Invalid UTF-8 in decoded setup token: {e}"),
        })?;

        let claim_url = claim_url.trim();
        let response = ureq::post(claim_url)
            .set("Content-Length", "0")
            .call()
            .map_err(|e| ImportError::Network(format!("SimpleFIN claim request failed: {e}")))?;

        let access_url = response
            .into_string()
            .map_err(|e| ImportError::Network(format!("Failed to read claim response: {e}")))?;

        Ok(access_url.trim().to_string())
    }

    /// Parses a SimpleFIN access URL into `(base_url, username, password)`.
    pub fn parse_access_url(access_url: &str) -> Result<(String, String, String), ImportError> {
        let url_str = access_url.trim();
        let stripped = if let Some(rest) = url_str.strip_prefix("https://") {
            rest
        } else if let Some(rest) = url_str.strip_prefix("http://") {
            rest
        } else {
            return Err(ImportError::BankSync {
                provider: "simplefin".into(),
                message: "Access URL missing http:// or https:// scheme".into(),
            });
        };

        let (auth_part, host_and_path) =
            stripped
                .split_once('@')
                .ok_or_else(|| ImportError::BankSync {
                    provider: "simplefin".into(),
                    message: "Access URL missing credentials (@ delimiter)".into(),
                })?;

        let (username, password) =
            auth_part
                .split_once(':')
                .ok_or_else(|| ImportError::BankSync {
                    provider: "simplefin".into(),
                    message: "Access URL credentials missing username:password separator".into(),
                })?;

        let scheme = if url_str.starts_with("https://") {
            "https://"
        } else {
            "http://"
        };
        let base_url = format!("{scheme}{host_and_path}")
            .trim_end_matches('/')
            .to_string();

        Ok((base_url, username.to_string(), password.to_string()))
    }

    /// Fetches all accounts and their recent transactions from SimpleFIN Bridge API.
    pub fn fetch_accounts_and_transactions(
        &self,
        start_date: Option<i64>,
        end_date: Option<i64>,
    ) -> Result<Vec<(BankSyncAccount, Vec<ImportedTransaction>)>, ImportError> {
        let endpoint = format!("{}/accounts", self.base_url);
        let mut req = ureq::get(&endpoint).set("Authorization", &self.basic_auth_header());

        if let Some(s) = start_date {
            req = req.query("start-date", &s.to_string());
        }
        if let Some(e) = end_date {
            req = req.query("end-date", &e.to_string());
        }

        let resp = req
            .call()
            .map_err(|e| ImportError::Network(format!("SimpleFIN API request failed: {e}")))?;

        let json_body = resp.into_string().map_err(|e| {
            ImportError::Network(format!("Failed to read SimpleFIN response body: {e}"))
        })?;

        Self::parse_accounts_response(&json_body)
    }

    /// Parses raw JSON response from SimpleFIN into normalized models.
    pub fn parse_accounts_response(
        json_str: &str,
    ) -> Result<Vec<(BankSyncAccount, Vec<ImportedTransaction>)>, ImportError> {
        let resp: SimpleFinResponse = serde_json::from_str(json_str)?;

        if !resp.errors.is_empty() {
            return Err(ImportError::BankSync {
                provider: "simplefin".into(),
                message: resp.errors.join("; "),
            });
        }

        let mut results = Vec::new();

        for acc in resp.accounts {
            let current_balance = acc.balance.parse::<Cents>().ok();
            let available_balance = acc
                .available_balance
                .as_deref()
                .and_then(|b| b.parse::<Cents>().ok());

            let institution_name = acc.org.as_ref().and_then(|o| o.name.clone());

            let bank_acc = BankSyncAccount {
                external_id: acc.id.clone(),
                name: acc.name,
                currency: acc.currency,
                current_balance,
                available_balance,
                institution_name,
                account_number_masked: None,
            };

            let mut txs = Vec::new();
            for raw_tx in acc.transactions {
                let amount = raw_tx.amount.parse::<Cents>().map_err(|_| {
                    ImportError::InvalidAmount(
                        raw_tx.amount.clone(),
                        "SimpleFIN amount parse error".into(),
                    )
                })?;

                let timestamp = if raw_tx.posted > 0 {
                    raw_tx.posted
                } else {
                    raw_tx.transacted_at.unwrap_or_default()
                };

                let date = DateTime::from_timestamp(timestamp, 0)
                    .map(|dt| dt.date_naive())
                    .unwrap_or_default();

                let payee_candidate = raw_tx.payee.clone().or_else(|| raw_tx.description.clone());
                let mut imported = ImportedTransaction::new(date, amount);
                imported.payee = payee_candidate.clone();
                imported.imported_payee = payee_candidate;
                imported.notes = raw_tx.memo.clone().or_else(|| raw_tx.description.clone());
                imported.imported_id = Some(raw_tx.id);
                imported.cleared = true;

                txs.push(imported);
            }

            results.push((bank_acc, txs));
        }

        Ok(results)
    }

    fn basic_auth_header(&self) -> String {
        let credentials = format!("{}:{}", self.username, self.password);
        let encoded = BASE64_STANDARD.encode(credentials.as_bytes());
        format!("Basic {encoded}")
    }
}
