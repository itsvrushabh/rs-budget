//! `actual-api` provides a typed application API and undo/redo state management
//! for rs-budget.
//!
//! # Overview
//!
//! The crate is organized into three public modules:
//!
//! - [`commands`] — typed command structs for every budget operation.
//! - [`service`] — [`BudgetService`], the central coordinator of domain logic
//!   and SQLite persistence.
//! - [`undo`] — [`UndoManager`] and [`UndoEntry`], the in-memory inverse-snapshot
//!   undo/redo stack.
//! - [`error`] — [`ApiError`] unifying domain, storage, and API-layer errors.

pub mod commands;
pub mod error;
pub mod service;
pub mod undo;

pub use error::ApiError;
pub use service::BudgetService;
pub use undo::{UndoEntry, UndoManager};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(10 / 2, 5);
    }
}
