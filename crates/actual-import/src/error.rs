//! Error types for statement parsing, transaction import, and bank synchronization.

use thiserror::Error;

/// Errors occurring during financial statement parsing, importing, and bank sync.
#[derive(Debug, Error)]
pub enum ImportError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("CSV parsing error: {0}")]
    Csv(#[from] csv::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),

    #[error("HTTP / Network error: {0}")]
    Network(String),

    #[error("Bank synchronization error ({provider}): {message}")]
    BankSync { provider: String, message: String },

    #[error("Cryptography error: {0}")]
    Crypto(#[from] actual_crypto::CryptoError),

    #[error("Invalid date format '{0}' with expected layouts: {1}")]
    InvalidDate(String, String),

    #[error("Invalid amount format '{0}': {1}")]
    InvalidAmount(String, String),

    #[error("Missing required column/field: {0}")]
    MissingField(String),

    #[error("Malformed file or payload structure: {0}")]
    MalformedFile(String),

    #[error("Domain error: {0}")]
    Domain(#[from] actual_core::DomainError),
}
