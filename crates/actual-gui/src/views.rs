//! High-level UI view components: Monthly Envelope Budget, Transaction Register, and Summary Bar.

use crate::table::{CellValue, ColumnAlign, EditableTable, TableCell, TableColumn};
use actual_core::cents::Cents;
use serde::{Deserialize, Serialize};

/// Summary metrics displayed in the top header banner of the budget interface.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SummaryBar {
    /// Month being inspected (e.g. "2024-05").
    pub month: String,
    /// Total funds available to budget ("To Budget").
    pub to_budget: Cents,
    /// Total monthly income.
    pub total_income: Cents,
    /// Total funds budgeted across all categories.
    pub total_budgeted: Cents,
    /// Total actual spending activity for the month.
    pub total_activity: Cents,
    /// Net available balance rolled over.
    pub net_balance: Cents,
}

impl SummaryBar {
    pub fn new(month: impl Into<String>) -> Self {
        Self {
            month: month.into(),
            ..Default::default()
        }
    }
}

/// Monthly Envelope Budget spreadsheet view component.
#[derive(Debug, Clone)]
pub struct BudgetView {
    /// Active month string (e.g. "2024-05").
    pub current_month: String,
    /// Summary bar.
    pub summary: SummaryBar,
    /// Editable financial table grid.
    pub table: EditableTable,
}

impl BudgetView {
    /// Creates a new BudgetView with standard column schema.
    pub fn new(month: impl Into<String>, viewport_height: u32) -> Self {
        let m = month.into();
        let columns = vec![
            TableColumn::new("name", "Category", 220),
            TableColumn::new("budgeted", "Budgeted", 120)
                .with_align(ColumnAlign::Right)
                .editable(true),
            TableColumn::new("activity", "Activity", 120).with_align(ColumnAlign::Right),
            TableColumn::new("balance", "Balance", 120).with_align(ColumnAlign::Right),
            TableColumn::new("carryover", "Carryover", 80)
                .with_align(ColumnAlign::Center)
                .editable(true),
        ];

        Self {
            current_month: m.clone(),
            summary: SummaryBar::new(m),
            table: EditableTable::new(columns, 32, viewport_height),
        }
    }

    /// Adds a category row to the budget table.
    pub fn add_category_row(
        &mut self,
        category_id: &str,
        name: &str,
        budgeted: Cents,
        activity: Cents,
        balance: Cents,
        carryover: bool,
    ) {
        let cells = vec![
            TableCell::new(CellValue::Text(name.to_string())),
            TableCell::new(CellValue::Currency(budgeted)),
            TableCell::new(CellValue::Currency(activity)),
            TableCell::new(CellValue::Currency(balance)),
            TableCell::new(CellValue::Boolean(carryover)),
        ];

        self.table.add_row(category_id, cells);
    }
}

/// Account Transaction Register view component.
#[derive(Debug, Clone)]
pub struct TransactionView {
    /// Current account filter (None for "All Accounts").
    pub account_id: Option<String>,
    /// Search / filter query string.
    pub filter_query: String,
    /// Editable transaction table grid.
    pub table: EditableTable,
}

impl TransactionView {
    /// Creates a new TransactionView with standard ledger columns.
    pub fn new(account_id: Option<String>, viewport_height: u32) -> Self {
        let columns = vec![
            TableColumn::new("date", "Date", 110).editable(true),
            TableColumn::new("account", "Account", 140),
            TableColumn::new("payee", "Payee", 180).editable(true),
            TableColumn::new("category", "Category", 180).editable(true),
            TableColumn::new("notes", "Notes", 220).editable(true),
            TableColumn::new("amount", "Amount", 110)
                .with_align(ColumnAlign::Right)
                .editable(true),
            TableColumn::new("cleared", "Clr", 60)
                .with_align(ColumnAlign::Center)
                .editable(true),
        ];

        Self {
            account_id,
            filter_query: String::new(),
            table: EditableTable::new(columns, 28, viewport_height),
        }
    }

    /// Adds a transaction row to the ledger table.
    #[allow(clippy::too_many_arguments)]
    pub fn add_transaction_row(
        &mut self,
        tx_id: &str,
        date: &str,
        account_name: &str,
        payee_name: &str,
        category_name: &str,
        notes: &str,
        amount: Cents,
        cleared: bool,
    ) {
        let cells = vec![
            TableCell::new(CellValue::Date(date.to_string())),
            TableCell::new(CellValue::Text(account_name.to_string())),
            TableCell::new(CellValue::Text(payee_name.to_string())),
            TableCell::new(CellValue::Text(category_name.to_string())),
            TableCell::new(CellValue::Text(notes.to_string())),
            TableCell::new(CellValue::Currency(amount)),
            TableCell::new(CellValue::Boolean(cleared)),
        ];

        self.table.add_row(tx_id, cells);
    }
}
