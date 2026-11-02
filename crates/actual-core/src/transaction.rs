//! Transaction and Split domain entities.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::cents::Cents;
use crate::error::DomainError;

/// Financial transaction entity representing money movements, category assignments, and splits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transaction {
    /// Unique identifier (UUID).
    pub id: String,
    /// Foreign key referencing the `Account`.
    pub account_id: String,
    /// Calendar date of the transaction.
    pub date: NaiveDate,
    /// Monetary amount in integer cents (positive = income/inflow, negative = expense/outflow).
    pub amount: Cents,
    /// Optional foreign key to a `Payee`.
    pub payee_id: Option<String>,
    /// Optional foreign key to a `Category` envelope.
    pub category_id: Option<String>,
    /// User notes or memo.
    pub notes: Option<String>,
    /// External import identifier (e.g. from bank sync or OFX FITID).
    pub imported_id: Option<String>,
    /// Original raw payee description from bank import.
    pub imported_payee: Option<String>,
    /// Whether this transaction represents an account starting balance.
    pub starting_balance_flag: bool,
    /// If part of an account transfer, foreign key to the counterpart `Transaction`.
    pub transfer_id: Option<String>,
    /// Display sort order within the day.
    pub sort_order: Option<f64>,
    /// Whether this transaction has cleared the bank.
    pub cleared: bool,
    /// Whether this transaction has been reconciled.
    pub reconciled: bool,
    /// Soft deletion marker.
    pub tombstone: bool,
    /// Optional link to a recurring `Schedule`.
    pub schedule_id: Option<String>,

    // Split transaction support
    /// True if this transaction is a parent containing subtransactions.
    pub is_parent: bool,
    /// True if this transaction is a child split subtransaction.
    pub is_child: bool,
    /// If a child split, foreign key to the parent `Transaction`.
    pub parent_id: Option<String>,
    /// Subtransactions if `is_parent` is true.
    #[serde(default)]
    pub subtransactions: Vec<Transaction>,
}

impl Transaction {
    /// Creates a standard new transaction.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        account_id: impl Into<String>,
        date: NaiveDate,
        amount: Cents,
    ) -> Self {
        Self {
            id: id.into(),
            account_id: account_id.into(),
            date,
            amount,
            payee_id: None,
            category_id: None,
            notes: None,
            imported_id: None,
            imported_payee: None,
            starting_balance_flag: false,
            transfer_id: None,
            sort_order: None,
            cleared: false,
            reconciled: false,
            tombstone: false,
            schedule_id: None,
            is_parent: false,
            is_child: false,
            parent_id: None,
            subtransactions: Vec::new(),
        }
    }

    /// Creates a starting balance transaction.
    #[must_use]
    pub fn new_starting_balance(
        id: impl Into<String>,
        account_id: impl Into<String>,
        date: NaiveDate,
        amount: Cents,
    ) -> Self {
        let mut tx = Self::new(id, account_id, date, amount);
        tx.starting_balance_flag = true;
        tx.cleared = true;
        tx
    }

    /// Returns whether this transaction is a transfer between accounts.
    #[must_use]
    pub fn is_transfer(&self) -> bool {
        self.transfer_id.is_some()
    }

    /// Returns whether this transaction is a split parent containing child subtransactions.
    #[must_use]
    pub fn is_split_parent(&self) -> bool {
        self.is_parent || !self.subtransactions.is_empty()
    }

    /// Returns whether this transaction is a child split.
    #[must_use]
    pub fn is_split_child(&self) -> bool {
        self.is_child || self.parent_id.is_some()
    }

    /// Adds a child split subtransaction to this parent transaction.
    pub fn add_subtransaction(&mut self, mut sub: Transaction) {
        self.is_parent = true;
        sub.is_child = true;
        sub.parent_id = Some(self.id.clone());
        sub.account_id = self.account_id.clone();
        sub.date = self.date;
        self.subtransactions.push(sub);
    }

    /// Sums the amounts of all subtransactions.
    #[must_use]
    pub fn calculate_splits_sum(&self) -> Cents {
        self.subtransactions.iter().map(|s| s.amount).sum()
    }

    /// Validates that split subtransactions sum up exactly to this parent transaction's amount.
    ///
    /// # Errors
    /// Returns [`DomainError::SplitBalanceMismatch`] if the sum does not equal the parent amount.
    pub fn validate_split_balance(&self) -> Result<(), DomainError> {
        if !self.is_split_parent() || self.subtransactions.is_empty() {
            return Ok(());
        }

        let sum = self.calculate_splits_sum();
        if sum != self.amount {
            return Err(DomainError::SplitBalanceMismatch {
                parent: self.amount,
                children_sum: sum,
                difference: self.amount - sum,
            });
        }

        Ok(())
    }
}
