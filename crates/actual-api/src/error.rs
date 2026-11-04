//! Error types for the `actual-api` service layer.

use thiserror::Error;

/// Errors that can arise from budget service operations.
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("account not found: {0}")]
    AccountNotFound(String),

    #[error("payee not found: {0}")]
    PayeeNotFound(String),

    #[error("category not found: {0}")]
    CategoryNotFound(String),

    #[error("category group not found: {0}")]
    CategoryGroupNotFound(String),

    #[error("transaction not found: {0}")]
    TransactionNotFound(String),

    #[error("rule not found: {0}")]
    RuleNotFound(String),

    #[error("schedule not found: {0}")]
    ScheduleNotFound(String),

    #[error("split amounts must sum to parent amount (got {got}, expected {expected})")]
    SplitAmountMismatch { got: i64, expected: i64 },

    #[error("cannot split: transaction is already a child split")]
    CannotSplitChild,

    #[error("undo stack is empty — nothing to undo")]
    NothingToUndo,

    #[error("redo stack is empty — nothing to redo")]
    NothingToRedo,

    #[error("domain error: {0}")]
    Domain(#[from] actual_core::DomainError),

    #[error("storage error: {0}")]
    Storage(#[from] actual_storage::StorageError),
}
