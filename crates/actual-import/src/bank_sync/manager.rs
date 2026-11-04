//! Bank synchronization manager coordinating credential storage, account linking,
//! incremental delta retrieval, payee cleaning, and deduplication.

use crate::bank_sync::gocardless::GoCardlessClient;
use crate::bank_sync::simplefin::SimpleFinClient;
use crate::bank_sync::types::{
    BankProviderType, BankSyncResult, LinkedAccount, ProviderCredentials,
};
use crate::cleaner::PayeeCleaner;
use crate::dedup::Deduplicator;
use crate::error::ImportError;
use crate::models::ImportedTransaction;
use actual_core::Transaction;
use actual_crypto::{cipher, Key};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// Serializable configuration container containing linked accounts and provider credentials.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BankSyncConfig {
    /// Configured linked accounts.
    pub accounts: Vec<LinkedAccount>,
    /// SimpleFIN credentials if configured.
    pub simplefin: Option<ProviderCredentials>,
    /// GoCardless credentials if configured.
    pub gocardless: Option<ProviderCredentials>,
}

/// Orchestrates bank account synchronization, credential encryption, and transaction pipeline.
#[derive(Debug, Clone, Default)]
pub struct BankSyncManager {
    config: BankSyncConfig,
    /// Custom SimpleFIN base URL override (optional, for testing).
    simplefin_base_url_override: Option<String>,
    /// Custom GoCardless base URL override (optional, for testing).
    gocardless_base_url_override: Option<String>,
}

impl BankSyncManager {
    /// Creates a new, empty bank synchronization manager.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets an optional base URL override for SimpleFIN (useful for unit testing).
    pub fn set_simplefin_base_url_override(&mut self, url: impl Into<String>) {
        self.simplefin_base_url_override = Some(url.into());
    }

    /// Sets an optional base URL override for GoCardless (useful for unit testing).
    pub fn set_gocardless_base_url_override(&mut self, url: impl Into<String>) {
        self.gocardless_base_url_override = Some(url.into());
    }

    /// Configures SimpleFIN provider credentials via access URL.
    pub fn set_simplefin_access_url(&mut self, access_url: impl Into<String>) {
        self.config.simplefin = Some(ProviderCredentials::SimpleFin {
            access_url: access_url.into(),
        });
    }

    /// Configures GoCardless credentials via Secret ID and Secret Key.
    pub fn set_gocardless_credentials(
        &mut self,
        secret_id: impl Into<String>,
        secret_key: impl Into<String>,
    ) {
        self.config.gocardless = Some(ProviderCredentials::GoCardless {
            secret_id: secret_id.into(),
            secret_key: secret_key.into(),
            access_token: None,
            refresh_token: None,
            token_expires_at: None,
        });
    }

    /// Links an internal account to an external bank provider account.
    pub fn link_account(
        &mut self,
        actual_account_id: impl Into<String>,
        external_account_id: impl Into<String>,
        provider: BankProviderType,
        external_name: Option<String>,
        currency: Option<String>,
    ) {
        let actual_id = actual_account_id.into();
        let external_id = external_account_id.into();

        // Update existing link if present
        if let Some(existing) = self
            .config
            .accounts
            .iter_mut()
            .find(|a| a.actual_account_id == actual_id)
        {
            existing.external_account_id = external_id;
            existing.provider = provider;
            existing.external_name = external_name;
            existing.currency = currency;
            return;
        }

        self.config.accounts.push(LinkedAccount {
            actual_account_id: actual_id,
            external_account_id: external_id,
            provider,
            external_name,
            last_sync: None,
            currency,
        });
    }

    /// Unlinks an account by internal account ID.
    pub fn unlink_account(&mut self, actual_account_id: &str) -> bool {
        let initial_len = self.config.accounts.len();
        self.config
            .accounts
            .retain(|a| a.actual_account_id != actual_account_id);
        self.config.accounts.len() < initial_len
    }

    /// Returns a slice of all active linked accounts.
    #[must_use]
    pub fn linked_accounts(&self) -> &[LinkedAccount] {
        &self.config.accounts
    }

    /// Looks up a linked account by its internal account UUID.
    #[must_use]
    pub fn get_linked_account(&self, actual_account_id: &str) -> Option<&LinkedAccount> {
        self.config
            .accounts
            .iter()
            .find(|a| a.actual_account_id == actual_account_id)
    }

    /// Encrypts the manager's configuration (accounts and credentials) into an AES-256-GCM ciphertext payload.
    pub fn export_encrypted(&self, key: &Key) -> Result<String, ImportError> {
        let json_bytes = serde_json::to_vec(&self.config)?;
        let encrypted_payload = cipher::encrypt(key, &json_bytes)?;
        let serialized = serde_json::to_string(&encrypted_payload)?;
        Ok(serialized)
    }

    /// Decrypts and restores the configuration from an AES-256-GCM ciphertext payload.
    pub fn import_encrypted(&mut self, encrypted_json: &str, key: &Key) -> Result<(), ImportError> {
        let encrypted_payload: actual_crypto::EncryptedData = serde_json::from_str(encrypted_json)?;
        let decrypted_bytes = cipher::decrypt(key, &encrypted_payload)?;
        let loaded_config: BankSyncConfig = serde_json::from_slice(&decrypted_bytes)?;
        self.config = loaded_config;
        Ok(())
    }

    /// Performs incremental delta sync for a specific linked account.
    ///
    /// Fetches transactions from the external bank provider since the last sync date (or 90 days ago if initial sync),
    /// applies payee normalization (if cleaner provided), and runs transactions through the deduplication engine
    /// against existing account transactions.
    pub fn sync_account(
        &mut self,
        actual_account_id: &str,
        existing_txs: &[Transaction],
        cleaner: Option<&PayeeCleaner>,
    ) -> Result<BankSyncResult, ImportError> {
        let (external_id, provider, last_sync) = {
            let account = self.get_linked_account(actual_account_id).ok_or_else(|| {
                ImportError::BankSync {
                    provider: "manager".into(),
                    message: format!(
                        "Account '{actual_account_id}' is not linked to any bank feed"
                    ),
                }
            })?;
            (
                account.external_account_id.clone(),
                account.provider,
                account.last_sync,
            )
        };

        // Calculate delta start date
        let start_time: DateTime<Utc> =
            last_sync.unwrap_or_else(|| Utc::now() - Duration::days(90));
        let start_date = start_time.date_naive();
        let start_sec = start_time.timestamp();

        let raw_transactions = match provider {
            BankProviderType::SimpleFin => {
                let access_url = match &self.config.simplefin {
                    Some(ProviderCredentials::SimpleFin { access_url }) => access_url,
                    _ => {
                        return Err(ImportError::BankSync {
                            provider: "simplefin".into(),
                            message: "SimpleFIN credentials not configured".into(),
                        });
                    }
                };

                let mut client = SimpleFinClient::from_access_url(access_url)?;
                if let Some(base) = &self.simplefin_base_url_override {
                    client.base_url = base.clone();
                }

                let accounts = client.fetch_accounts_and_transactions(Some(start_sec), None)?;
                let (_, txs) = accounts
                    .into_iter()
                    .find(|(acc, _)| acc.external_id == external_id)
                    .ok_or_else(|| ImportError::BankSync {
                        provider: "simplefin".into(),
                        message: format!(
                            "External account '{external_id}' not found in SimpleFIN feed"
                        ),
                    })?;
                txs
            }
            BankProviderType::GoCardless => {
                let (secret_id, secret_key) = match &self.config.gocardless {
                    Some(ProviderCredentials::GoCardless {
                        secret_id,
                        secret_key,
                        ..
                    }) => (secret_id.clone(), secret_key.clone()),
                    _ => {
                        return Err(ImportError::BankSync {
                            provider: "gocardless".into(),
                            message: "GoCardless credentials not configured".into(),
                        });
                    }
                };

                let mut client = GoCardlessClient::new(secret_id, secret_key);
                if let Some(base) = &self.gocardless_base_url_override {
                    client = client.with_base_url(base.clone());
                }

                // If cached tokens exist, copy them
                if let Some(ProviderCredentials::GoCardless {
                    access_token,
                    refresh_token,
                    token_expires_at,
                    ..
                }) = &self.config.gocardless
                {
                    client.access_token = access_token.clone();
                    client.refresh_token = refresh_token.clone();
                    client.token_expires_at = *token_expires_at;
                }

                let txs = client.fetch_transactions(&external_id, Some(start_date))?;

                // Persist any updated tokens back to config
                if let Some(ProviderCredentials::GoCardless {
                    access_token,
                    refresh_token,
                    token_expires_at,
                    ..
                }) = &mut self.config.gocardless
                {
                    *access_token = client.access_token;
                    *refresh_token = client.refresh_token;
                    *token_expires_at = client.token_expires_at;
                }

                txs
            }
        };

        // Process raw transactions: clean payees and deduplicate
        let processed_result =
            Self::process_and_deduplicate(raw_transactions, existing_txs, cleaner);

        // Update last_sync timestamp on linked account
        if let Some(account) = self
            .config
            .accounts
            .iter_mut()
            .find(|a| a.actual_account_id == actual_account_id)
        {
            account.last_sync = Some(Utc::now());
        }

        Ok(processed_result)
    }

    /// Helper to clean payees and deduplicate imported transactions against existing transactions.
    pub fn process_and_deduplicate(
        mut imported: Vec<ImportedTransaction>,
        existing_txs: &[Transaction],
        cleaner: Option<&PayeeCleaner>,
    ) -> BankSyncResult {
        if let Some(cleaner) = cleaner {
            for tx in &mut imported {
                if let Some(raw_payee) = &tx.imported_payee {
                    let cleaned = cleaner.clean(raw_payee);
                    tx.payee = cleaned;
                }
            }
        }

        let dedup = Deduplicator::new(3);
        let dedup_res = dedup.deduplicate(imported, existing_txs);
        let latest_date = dedup_res.unique.iter().map(|t| t.date).max();

        BankSyncResult {
            imported_transactions: dedup_res.unique,
            duplicate_count: dedup_res.duplicates.len(),
            latest_transaction_date: latest_date,
            warnings: Vec::new(),
        }
    }
}
