//! Typed command messages for all budget operations.
//!
//! Each variant carries every field required to perform and later undo the operation.

use actual_core::{Cents, Transaction};
use chrono::NaiveDate;

// ─── Account commands ──────────────────────────────────────────────────────────

/// Create a new financial account.
#[derive(Debug, Clone)]
pub struct CreateAccount {
    /// Caller-supplied UUID; if empty one will be generated.
    pub id: String,
    pub name: String,
    pub offbudget: bool,
}

/// Rename an existing account.
#[derive(Debug, Clone)]
pub struct RenameAccount {
    pub account_id: String,
    pub new_name: String,
}

/// Mark an account as closed (soft-delete).
#[derive(Debug, Clone)]
pub struct CloseAccount {
    pub account_id: String,
}

/// Reopen a previously closed account.
#[derive(Debug, Clone)]
pub struct ReopenAccount {
    pub account_id: String,
}

// ─── Payee commands ────────────────────────────────────────────────────────────

/// Create a new payee.
#[derive(Debug, Clone)]
pub struct CreatePayee {
    pub id: String,
    pub name: String,
}

/// Rename an existing payee.
#[derive(Debug, Clone)]
pub struct RenamePayee {
    pub payee_id: String,
    pub new_name: String,
}

/// Merge one payee into another (re-assign transactions, then tombstone source).
#[derive(Debug, Clone)]
pub struct MergePayees {
    pub source_payee_id: String,
    pub target_payee_id: String,
}

// ─── Category commands ─────────────────────────────────────────────────────────

/// Create a new category group.
#[derive(Debug, Clone)]
pub struct CreateCategoryGroup {
    pub id: String,
    pub name: String,
    pub is_income: bool,
}

/// Create a new budget envelope category.
#[derive(Debug, Clone)]
pub struct CreateCategory {
    pub id: String,
    pub name: String,
    pub group_id: String,
    pub is_income: bool,
}

/// Rename a category.
#[derive(Debug, Clone)]
pub struct RenameCategory {
    pub category_id: String,
    pub new_name: String,
}

/// Move a category to a different group.
#[derive(Debug, Clone)]
pub struct MoveCategory {
    pub category_id: String,
    pub new_group_id: String,
}

/// Hide or unhide a category.
#[derive(Debug, Clone)]
pub struct SetCategoryVisibility {
    pub category_id: String,
    pub hidden: bool,
}

/// Soft-delete a category (tombstone).
#[derive(Debug, Clone)]
pub struct DeleteCategory {
    pub category_id: String,
}

// ─── Envelope budget commands ──────────────────────────────────────────────────

/// Set the budgeted amount for a category envelope in a given month.
#[derive(Debug, Clone)]
pub struct SetBudget {
    /// Year-month in `"YYYY-MM"` format.
    pub month: String,
    pub category_id: String,
    pub amount: Cents,
}

/// Toggle carry-over behaviour for an overspent category.
#[derive(Debug, Clone)]
pub struct SetCarryover {
    pub month: String,
    pub category_id: String,
    pub carry: bool,
}

// ─── Transaction commands ──────────────────────────────────────────────────────

/// Add a single (non-split) transaction.
#[derive(Debug, Clone)]
pub struct AddTransaction {
    pub id: String,
    pub account_id: String,
    pub date: NaiveDate,
    pub amount: Cents,
    pub payee_id: Option<String>,
    pub category_id: Option<String>,
    pub notes: Option<String>,
    pub cleared: bool,
}

/// Update mutable fields of an existing transaction.
#[derive(Debug, Clone)]
pub struct UpdateTransaction {
    pub transaction_id: String,
    pub date: Option<NaiveDate>,
    pub amount: Option<Cents>,
    pub payee_id: Option<Option<String>>,
    pub category_id: Option<Option<String>>,
    pub notes: Option<Option<String>>,
    pub cleared: Option<bool>,
}

/// Soft-delete a transaction (tombstone).
#[derive(Debug, Clone)]
pub struct DeleteTransaction {
    pub transaction_id: String,
}

/// Convert a simple transaction into a parent-split and add child sub-transactions.
#[derive(Debug, Clone)]
pub struct SplitTransaction {
    pub parent_transaction_id: String,
    /// Each child carries (category_id, amount, notes).
    pub splits: Vec<(Option<String>, Cents, Option<String>)>,
}

/// Undo a prior `SplitTransaction` — collapses children back into the parent.
#[derive(Debug, Clone)]
pub struct UnsplitTransaction {
    pub parent_transaction_id: String,
}

// ─── Rules commands ────────────────────────────────────────────────────────────

/// Add a new transaction rule.
#[derive(Debug, Clone)]
pub struct CreateRule {
    pub id: String,
    pub rule: actual_core::Rule,
}

/// Replace an existing rule entirely.
#[derive(Debug, Clone)]
pub struct UpdateRule {
    pub rule_id: String,
    pub updated: actual_core::Rule,
}

/// Delete a rule.
#[derive(Debug, Clone)]
pub struct DeleteRule {
    pub rule_id: String,
}

/// Apply all rules to a batch of transaction IDs.
#[derive(Debug, Clone)]
pub struct ApplyRules {
    pub transaction_ids: Vec<String>,
}

// ─── Schedule commands ─────────────────────────────────────────────────────────

/// Register a new recurring transaction schedule.
#[derive(Debug, Clone)]
pub struct CreateSchedule {
    pub id: String,
    pub account_id: String,
    pub amount: Cents,
    pub config: actual_core::ScheduleConfig,
}

/// Update a schedule configuration.
#[derive(Debug, Clone)]
pub struct UpdateSchedule {
    pub schedule_id: String,
    pub config: actual_core::ScheduleConfig,
}

/// Delete a schedule and detach its transactions.
#[derive(Debug, Clone)]
pub struct DeleteSchedule {
    pub schedule_id: String,
}

/// Generate upcoming template transactions from a schedule within a date range.
#[derive(Debug, Clone)]
pub struct GetUpcomingTransactions {
    pub schedule_id: String,
    pub from_date: NaiveDate,
    pub until_date: NaiveDate,
}

/// Result of [`GetUpcomingTransactions`].
#[derive(Debug, Clone)]
pub struct UpcomingTransactions {
    pub schedule_id: String,
    pub transactions: Vec<Transaction>,
}
