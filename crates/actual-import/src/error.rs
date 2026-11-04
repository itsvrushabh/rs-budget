//! Error types for statement parsing and transaction import.

use thiserror::Error;

/// Errors occurring during financial statement parsing and importing.
#[derive(Debug, Error)]
pub enum ImportError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("CSV parsing error: {0}")]
    Csv(#[from] csv::Error),

    #[error("Invalid date format '{0}' with expected layouts: {1}")]
    InvalidDate(String, String),

    #[error("Invalid amount format '{0}': {1}")]
    InvalidAmount(String, String),

    #[error("Missing required column/field: {0}")]
    MissingField(String),

    #[error("Malformed file structure: {0}")]
    MalformedFile(String),

    #[error("Domain error: {0}")]
    Domain(#[from] actual_core::DomainError),
}
