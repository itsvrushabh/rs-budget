//! Domain error types for core budgeting entities and monetary calculations.

use thiserror::Error;

use crate::cents::Cents;

/// Errors arising from domain entity invariants, monetary math, spreadsheet DAG, and transfer validations.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum DomainError {
    /// Monetary parsing error from string representation.
    #[error("failed to parse monetary amount '{0}'")]
    CentsParse(String),

    /// Monetary calculation overflow or unsafe number.
    #[error("monetary calculation overflow or exceeds safe limits")]
    CentsOverflow,

    /// Split transaction amounts do not sum up to the parent transaction amount.
    #[error("split transaction mismatch: parent {parent}, subtransactions sum {children_sum}, difference {difference}")]
    SplitBalanceMismatch {
        parent: Cents,
        children_sum: Cents,
        difference: Cents,
    },

    /// Transfer transaction amounts are not balanced inverses.
    #[error(
        "transfer amounts unbalanced: transfer amount {transfer}, counterpart amount {counterpart}"
    )]
    TransferBalanceMismatch { transfer: Cents, counterpart: Cents },

    /// Invalid transfer configuration or self-transfer.
    #[error("invalid transfer: {0}")]
    InvalidTransfer(String),

    /// Entity validation failure or missing mandatory fields.
    #[error("entity validation failure: {0}")]
    Validation(String),

    /// A dependency cycle was detected in the spreadsheet calculation graph.
    #[error("dependency cycle detected in calculation graph: {0}")]
    CycleDetected(String),

    /// Invalid month format string (expected YYYY-MM).
    #[error("invalid month format: '{0}', expected YYYY-MM")]
    InvalidMonthFormat(String),

    /// Referenced cell was not found in the spreadsheet.
    #[error("cell not found in spreadsheet: '{0}'")]
    CellNotFound(String),
}
