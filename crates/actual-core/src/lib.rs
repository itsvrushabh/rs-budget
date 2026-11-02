//! `actual-core` defines domain models, monetary precision,
//! envelope budgeting calculations, and deterministic transaction rules.

pub mod account;
pub mod budget;
pub mod category;
pub mod cents;
pub mod error;
pub mod models;
pub mod payee;
pub mod spreadsheet;
pub mod transaction;
pub mod validation;

pub use account::{Account, AccountGroup};
pub use budget::EnvelopeBudget;
pub use category::{Category, CategoryGroup};
pub use cents::{Cents, MAX_SAFE_CENTS, MIN_SAFE_CENTS};
pub use error::DomainError;
pub use payee::Payee;
pub use spreadsheet::{CellValue, DependencyGraph, Spreadsheet};
pub use transaction::Transaction;
pub use validation::{
    validate_account, validate_category, validate_category_group, validate_payee, validate_split,
    validate_transfer_categorization, validate_transfer_pair,
};
