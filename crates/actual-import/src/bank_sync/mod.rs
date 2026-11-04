//! Automated direct bank synchronization providers and sync coordination engine.
//!
//! Provides protocol adapters for:
//! - **SimpleFIN Bridge**: Access token exchange and financial data feeds.
//! - **GoCardless (Nordigen)**: European Open Banking PSD2 directory, requisitions, and accounts.
//!
//! Includes secure AES-256-GCM encrypted credential persistence and deduplicated delta sync.

pub mod gocardless;
pub mod manager;
pub mod simplefin;
pub mod types;

pub use gocardless::{GoCardlessClient, Institution, RequisitionResponse};
pub use manager::{BankSyncConfig, BankSyncManager};
pub use simplefin::{SimpleFinAccount, SimpleFinClient, SimpleFinTransaction};
pub use types::{
    BankProviderType, BankSyncAccount, BankSyncResult, LinkedAccount, ProviderCredentials,
};
