//! Error types for the actual-gui graphical interface crate.

use thiserror::Error;

/// Errors arising during GUI operations, table manipulation, or service bridging.
#[derive(Debug, Error)]
pub enum GuiError {
    /// Cell index out of bounds.
    #[error("Cell index out of bounds: row {0}, col {1}")]
    OutOfBounds(usize, usize),

    /// Cell validation failure (e.g. invalid currency string or date).
    #[error("Validation failed for cell [{row}, {col}]: {reason}")]
    ValidationFailed {
        row: usize,
        col: usize,
        reason: String,
    },

    /// The requested column is not editable.
    #[error("Column '{0}' is read-only")]
    ReadOnlyColumn(String),

    /// Backend service error.
    #[error("Service bridge error: {0}")]
    Service(String),

    /// Internal error.
    #[error("GUI internal error: {0}")]
    Internal(String),
}
