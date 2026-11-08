//! Shared data structures and configuration models for automated bank synchronization.

use crate::models::ImportedTransaction;
use actual_core::Cents;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

/// Supported automated bank synchronization providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BankProviderType {
    /// SimpleFIN Bridge protocol (North America).
    SimpleFin,
    /// GoCardless / Nordigen Open Banking API (Europe & UK).
    GoCardless,
    /// Plaid API integration (US, Canada, UK, Europe).
    Plaid,
    /// Teller API integration (US Real-Time Banking).
    Teller,
}

impl std::fmt::Display for BankProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SimpleFin => write!(f, "SimpleFIN"),
            Self::GoCardless => write!(f, "GoCardless"),
            Self::Plaid => write!(f, "Plaid"),
            Self::Teller => write!(f, "Teller"),
        }
    }
}

/// Normalized bank account representation retrieved from external provider.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BankSyncAccount {
    /// Provider's unique external account identifier.
    pub external_id: String,
    /// Friendly account display name.
    pub name: String,
    /// Currency code (e.g., "USD", "EUR", "GBP").
    pub currency: String,
    /// Current account balance.
    pub current_balance: Option<Cents>,
    /// Available balance if reported.
    pub available_balance: Option<Cents>,
    /// Institution or bank organization name.
    pub institution_name: Option<String>,
    /// Masked account number or IBAN if available.
    pub account_number_masked: Option<String>,
}

/// Mapping between an rs-budget internal `Account` and an external bank account.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LinkedAccount {
    /// Internal rs-budget account UUID.
    pub actual_account_id: String,
    /// External bank account identifier from provider.
    pub external_account_id: String,
    /// Provider providing the data feed.
    pub provider: BankProviderType,
    /// Optional custom display name of the external account.
    pub external_name: Option<String>,
    /// Timestamp of the last successful synchronization.
    pub last_sync: Option<DateTime<Utc>>,
    /// Currency code reported by external provider.
    pub currency: Option<String>,
}

/// Result of an account or bulk bank synchronization operation.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BankSyncResult {
    /// Transactions successfully imported and cleared of duplicates.
    pub imported_transactions: Vec<ImportedTransaction>,
    /// Number of transactions ignored because they were exact or proximity duplicates.
    pub duplicate_count: usize,
    /// Latest transaction date observed during sync.
    pub latest_transaction_date: Option<NaiveDate>,
    /// Any warnings or informational notes during sync.
    pub warnings: Vec<String>,
}

/// Provider credentials configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "provider", rename_all = "lowercase")]
pub enum ProviderCredentials {
    /// SimpleFIN credentials.
    SimpleFin {
        /// Access URL obtained via setup token claim.
        access_url: String,
    },
    /// GoCardless credentials and active OAuth tokens.
    GoCardless {
        /// Secret ID from GoCardless developer portal.
        secret_id: String,
        /// Secret Key from GoCardless developer portal.
        secret_key: String,
        /// Active access token (if already exchanged).
        access_token: Option<String>,
        /// Refresh token (if already exchanged).
        refresh_token: Option<String>,
        /// Unix timestamp when access token expires.
        token_expires_at: Option<i64>,
    },
}
