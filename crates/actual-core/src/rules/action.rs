//! Rule action types and transaction transformation execution.

use serde::{Deserialize, Serialize};

use crate::cents::Cents;
use crate::transaction::Transaction;

/// Fields on a transaction that can be modified by rule actions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionField {
    Category,
    Payee,
    Notes,
    Cleared,
    Account,
    Split,
}

/// Operation to perform when executing a rule action.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ActionOp {
    Set,
    PrependNotes,
    AppendNotes,
    LinkSchedule,
    SetSplit,
}

/// Dynamic value applied by a rule action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ActionValue {
    Text(String),
    Cents(Cents),
    Bool(bool),
    Splits(Vec<SplitAllocation>),
}

/// Allocation definition for auto-splitting transactions via rules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SplitAllocation {
    pub amount: Cents,
    pub category_id: Option<String>,
    pub payee_id: Option<String>,
    pub notes: Option<String>,
}

/// An action performed when a rule's conditions match a transaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Action {
    pub field: ActionField,
    pub op: ActionOp,
    pub value: ActionValue,
}

impl Action {
    /// Creates a new action.
    #[must_use]
    pub fn new(field: ActionField, op: ActionOp, value: ActionValue) -> Self {
        Self { field, op, value }
    }

    /// Applies this action to modify the given transaction in-place.
    pub fn apply(&self, tx: &mut Transaction) {
        match (&self.field, &self.op) {
            (ActionField::Category, ActionOp::Set) => {
                if let ActionValue::Text(cat_id) = &self.value {
                    tx.category_id = Some(cat_id.clone());
                }
            }
            (ActionField::Payee, ActionOp::Set) => {
                if let ActionValue::Text(payee_id) = &self.value {
                    tx.payee_id = Some(payee_id.clone());
                }
            }
            (ActionField::Account, ActionOp::Set) => {
                if let ActionValue::Text(account_id) = &self.value {
                    tx.account_id = account_id.clone();
                }
            }
            (ActionField::Cleared, ActionOp::Set) => {
                if let ActionValue::Bool(b) = self.value {
                    tx.cleared = b;
                }
            }
            (ActionField::Notes, ActionOp::Set) => {
                if let ActionValue::Text(notes) = &self.value {
                    tx.notes = Some(notes.clone());
                }
            }
            (ActionField::Notes, ActionOp::PrependNotes) => {
                if let ActionValue::Text(prefix) = &self.value {
                    let new_notes = match &tx.notes {
                        Some(existing) if !existing.is_empty() => format!("{prefix} {existing}"),
                        _ => prefix.clone(),
                    };
                    tx.notes = Some(new_notes);
                }
            }
            (ActionField::Notes, ActionOp::AppendNotes) => {
                if let ActionValue::Text(suffix) = &self.value {
                    let new_notes = match &tx.notes {
                        Some(existing) if !existing.is_empty() => format!("{existing} {suffix}"),
                        _ => suffix.clone(),
                    };
                    tx.notes = Some(new_notes);
                }
            }
            (ActionField::Split, ActionOp::SetSplit) => {
                if let ActionValue::Splits(allocations) = &self.value {
                    tx.is_parent = true;
                    tx.subtransactions.clear();

                    for (idx, alloc) in allocations.iter().enumerate() {
                        let mut sub = Transaction::new(
                            format!("{}_split_{idx}", tx.id),
                            tx.account_id.clone(),
                            tx.date,
                            alloc.amount,
                        );
                        sub.is_child = true;
                        sub.parent_id = Some(tx.id.clone());
                        sub.category_id = alloc.category_id.clone();
                        sub.payee_id = alloc.payee_id.clone();
                        sub.notes = alloc.notes.clone();
                        tx.subtransactions.push(sub);
                    }
                }
            }
            _ => {}
        }
    }
}
