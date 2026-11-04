//! Models representing imported transactions and splits.

use actual_core::{Cents, Transaction};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// An individual split line item within an imported transaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportedSplit {
    /// Monetary amount for this split.
    pub amount: Cents,
    /// Category name or identifier.
    pub category: Option<String>,
    /// Memo or notes for this split.
    pub notes: Option<String>,
}

/// A parsed transaction extracted from a financial statement (QIF, OFX, CSV).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportedTransaction {
    /// Date of the transaction.
    pub date: NaiveDate,
    /// Monetary amount in integer cents. Positive = inflow/income, Negative = outflow/expense.
    pub amount: Cents,
    /// Payee name (may be normalized/cleaned).
    pub payee: Option<String>,
    /// Original raw payee name before cleaning.
    pub imported_payee: Option<String>,
    /// Memo or description notes.
    pub notes: Option<String>,
    /// External statement identifier (e.g., OFX FITID or bank reference number).
    pub imported_id: Option<String>,
    /// Category name or identifier if present in statement.
    pub category: Option<String>,
    /// Whether the transaction was cleared in the source statement.
    pub cleared: bool,
    /// Sub-transactions if the imported item is a split.
    pub subtransactions: Vec<ImportedSplit>,
}

impl ImportedTransaction {
    /// Creates a new `ImportedTransaction`.
    #[must_use]
    pub fn new(date: NaiveDate, amount: Cents) -> Self {
        Self {
            date,
            amount,
            payee: None,
            imported_payee: None,
            notes: None,
            imported_id: None,
            category: None,
            cleared: true,
            subtransactions: Vec::new(),
        }
    }

    /// Converts this imported item into a domain `Transaction` associated with `account_id`.
    #[must_use]
    pub fn into_domain_transaction(self, account_id: &str) -> Transaction {
        let parent_id = Uuid::new_v4().to_string();
        let mut tx = Transaction::new(&parent_id, account_id, self.date, self.amount);
        tx.notes = self.notes;
        tx.imported_id = self.imported_id;
        tx.imported_payee = self.imported_payee.or_else(|| self.payee.clone());
        tx.cleared = self.cleared;

        if !self.subtransactions.is_empty() {
            tx.is_parent = true;
            for split in self.subtransactions {
                let child_id = Uuid::new_v4().to_string();
                let mut child = Transaction::new(&child_id, account_id, self.date, split.amount);
                child.is_child = true;
                child.parent_id = Some(parent_id.clone());
                child.notes = split.notes;
                tx.subtransactions.push(child);
            }
        }

        tx
    }
}
