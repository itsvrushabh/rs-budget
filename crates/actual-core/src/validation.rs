//! Domain invariant validation rules for transactions, transfers, accounts, and categories.

use crate::account::Account;
use crate::category::{Category, CategoryGroup};
use crate::error::DomainError;
use crate::payee::Payee;
use crate::transaction::Transaction;

/// Validates split transaction balance: the sum of child subtransaction amounts
/// must equal the parent transaction amount.
pub fn validate_split(tx: &Transaction) -> Result<(), DomainError> {
    tx.validate_split_balance()
}

/// Validates that a pair of transactions constitutes a valid, balanced transfer between accounts.
///
/// Invariants verified:
/// 1. The two transactions must reference different accounts.
/// 2. The amounts must be exact inverses (`tx1.amount == -tx2.amount`).
/// 3. Cross-linking: `tx1.transfer_id == Some(tx2.id)` and `tx2.transfer_id == Some(tx1.id)`.
pub fn validate_transfer_pair(tx1: &Transaction, tx2: &Transaction) -> Result<(), DomainError> {
    if tx1.account_id == tx2.account_id {
        return Err(DomainError::InvalidTransfer(
            "transfer source and destination accounts cannot be identical".to_string(),
        ));
    }

    if tx1.amount != -tx2.amount {
        return Err(DomainError::TransferBalanceMismatch {
            transfer: tx1.amount,
            counterpart: tx2.amount,
        });
    }

    if tx1.transfer_id.as_deref() != Some(&tx2.id) || tx2.transfer_id.as_deref() != Some(&tx1.id) {
        return Err(DomainError::InvalidTransfer(
            "transfer pair transaction IDs are not reciprocal".to_string(),
        ));
    }

    Ok(())
}

/// Validates categorization rules for transfers based on account budget participation.
///
/// In upstream Actual Budget:
/// - Transfers between two on-budget accounts do not affect envelopes (no category permitted).
/// - Transfers between an on-budget and an off-budget (tracking) account represent money entering
///   or leaving the envelope budget pool, and therefore require a valid category assignment.
pub fn validate_transfer_categorization(
    tx: &Transaction,
    source_acct: &Account,
    dest_acct: &Account,
) -> Result<(), DomainError> {
    let source_on_budget = source_acct.is_on_budget();
    let dest_on_budget = dest_acct.is_on_budget();

    if source_on_budget && dest_on_budget {
        // Both on-budget: internal transfer does not affect budget envelopes
        if tx.category_id.is_some() {
            return Err(DomainError::Validation(
                "transfers between on-budget accounts must not have a category assigned"
                    .to_string(),
            ));
        }
    } else if (source_on_budget && !dest_on_budget) || (!source_on_budget && dest_on_budget) {
        // Cross-boundary transfer: must be categorized to maintain envelope balance
        if tx.category_id.is_none() {
            return Err(DomainError::Validation(
                "transfers between on-budget and off-budget accounts require a category"
                    .to_string(),
            ));
        }
    }

    Ok(())
}

/// Validates basic invariants for an `Account`.
pub fn validate_account(acct: &Account) -> Result<(), DomainError> {
    if acct.id.trim().is_empty() {
        return Err(DomainError::Validation(
            "account ID cannot be empty".to_string(),
        ));
    }
    if acct.name.trim().is_empty() {
        return Err(DomainError::Validation(
            "account name cannot be empty".to_string(),
        ));
    }
    Ok(())
}

/// Validates basic invariants for a `CategoryGroup`.
pub fn validate_category_group(group: &CategoryGroup) -> Result<(), DomainError> {
    if group.id.trim().is_empty() {
        return Err(DomainError::Validation(
            "category group ID cannot be empty".to_string(),
        ));
    }
    if group.name.trim().is_empty() {
        return Err(DomainError::Validation(
            "category group name cannot be empty".to_string(),
        ));
    }
    Ok(())
}

/// Validates basic invariants for a `Category`.
pub fn validate_category(cat: &Category) -> Result<(), DomainError> {
    if cat.id.trim().is_empty() {
        return Err(DomainError::Validation(
            "category ID cannot be empty".to_string(),
        ));
    }
    if cat.name.trim().is_empty() {
        return Err(DomainError::Validation(
            "category name cannot be empty".to_string(),
        ));
    }
    if cat.group_id.trim().is_empty() {
        return Err(DomainError::Validation(
            "category group_id cannot be empty".to_string(),
        ));
    }
    Ok(())
}

/// Validates basic invariants for a `Payee`.
pub fn validate_payee(payee: &Payee) -> Result<(), DomainError> {
    if payee.id.trim().is_empty() {
        return Err(DomainError::Validation(
            "payee ID cannot be empty".to_string(),
        ));
    }
    if payee.name.trim().is_empty() {
        return Err(DomainError::Validation(
            "payee name cannot be empty".to_string(),
        ));
    }
    Ok(())
}
