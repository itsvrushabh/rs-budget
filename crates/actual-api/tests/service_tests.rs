//! Integration tests for the `BudgetService` typed API and undo/redo stack.

use actual_api::commands::*;
use actual_api::service::BudgetService;
use actual_api::ApiError;
use actual_core::Cents;
use chrono::{Datelike, NaiveDate};

/// Helper to create cents from a dollar integer.
fn dollars(d: i64) -> Cents {
    Cents::from_cents(d * 100)
}

// ─── Account CRUD ──────────────────────────────────────────────────────────────

#[test]
fn test_create_and_read_account() {
    let mut svc = BudgetService::open_in_memory().unwrap();
    let id = svc
        .create_account(CreateAccount {
            id: "acc-1".into(),
            name: "Checking".into(),
            offbudget: false,
        })
        .unwrap();
    assert_eq!(id, "acc-1");
    let acc = svc.account("acc-1").unwrap();
    assert_eq!(acc.name, "Checking");
    assert!(!acc.offbudget);
    assert!(!acc.closed);
    assert_eq!(svc.accounts().len(), 1);
}

#[test]
fn test_rename_account() {
    let mut svc = BudgetService::open_in_memory().unwrap();
    svc.create_account(CreateAccount {
        id: "acc-1".into(),
        name: "Old Name".into(),
        offbudget: false,
    })
    .unwrap();
    svc.rename_account(RenameAccount {
        account_id: "acc-1".into(),
        new_name: "New Name".into(),
    })
    .unwrap();
    assert_eq!(svc.account("acc-1").unwrap().name, "New Name");
}

#[test]
fn test_close_reopen_account() {
    let mut svc = BudgetService::open_in_memory().unwrap();
    svc.create_account(CreateAccount {
        id: "acc-1".into(),
        name: "Savings".into(),
        offbudget: false,
    })
    .unwrap();
    svc.close_account(CloseAccount {
        account_id: "acc-1".into(),
    })
    .unwrap();
    assert!(svc.account("acc-1").unwrap().closed);
    svc.reopen_account(ReopenAccount {
        account_id: "acc-1".into(),
    })
    .unwrap();
    assert!(!svc.account("acc-1").unwrap().closed);
}

#[test]
fn test_account_not_found() {
    let mut svc = BudgetService::open_in_memory().unwrap();
    let err = svc
        .rename_account(RenameAccount {
            account_id: "no-such".into(),
            new_name: "x".into(),
        })
        .unwrap_err();
    assert!(matches!(err, ApiError::AccountNotFound(_)));
}

// ─── Payee CRUD ────────────────────────────────────────────────────────────────

#[test]
fn test_create_and_rename_payee() {
    let mut svc = BudgetService::open_in_memory().unwrap();
    let id = svc
        .create_payee(CreatePayee {
            id: "pay-1".into(),
            name: "Grocery Store".into(),
        })
        .unwrap();
    assert_eq!(id, "pay-1");
    svc.rename_payee(RenamePayee {
        payee_id: "pay-1".into(),
        new_name: "Super Market".into(),
    })
    .unwrap();
    assert_eq!(svc.payee("pay-1").unwrap().name, "Super Market");
}

#[test]
fn test_merge_payees() {
    let mut svc = BudgetService::open_in_memory().unwrap();
    svc.create_account(CreateAccount {
        id: "acc-1".into(),
        name: "Checking".into(),
        offbudget: false,
    })
    .unwrap();
    svc.create_payee(CreatePayee {
        id: "pay-a".into(),
        name: "Payee A".into(),
    })
    .unwrap();
    svc.create_payee(CreatePayee {
        id: "pay-b".into(),
        name: "Payee B".into(),
    })
    .unwrap();

    // Add a transaction referencing pay-a.
    svc.add_transaction(AddTransaction {
        id: "tx-1".into(),
        account_id: "acc-1".into(),
        date: NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
        amount: dollars(-50),
        payee_id: Some("pay-a".into()),
        category_id: None,
        notes: None,
        cleared: false,
    })
    .unwrap();

    svc.merge_payees(MergePayees {
        source_payee_id: "pay-a".into(),
        target_payee_id: "pay-b".into(),
    })
    .unwrap();

    // pay-a is tombstoned, tx-1 now references pay-b.
    assert!(svc.payee("pay-a").unwrap().tombstone);
    let txs = svc.transactions();
    assert_eq!(txs[0].payee_id, Some("pay-b".to_string()));
}

// ─── Category & Category Group ─────────────────────────────────────────────────

#[test]
fn test_create_category_group_and_category() {
    let mut svc = BudgetService::open_in_memory().unwrap();
    svc.create_category_group(CreateCategoryGroup {
        id: "grp-1".into(),
        name: "Housing".into(),
        is_income: false,
    })
    .unwrap();
    svc.create_category(CreateCategory {
        id: "cat-1".into(),
        name: "Rent".into(),
        group_id: "grp-1".into(),
        is_income: false,
    })
    .unwrap();
    assert_eq!(svc.category("cat-1").unwrap().name, "Rent");
    assert_eq!(svc.category_groups().len(), 1);
    assert_eq!(svc.categories().len(), 1);
}

#[test]
fn test_category_requires_existing_group() {
    let mut svc = BudgetService::open_in_memory().unwrap();
    let err = svc
        .create_category(CreateCategory {
            id: "cat-1".into(),
            name: "Rent".into(),
            group_id: "no-group".into(),
            is_income: false,
        })
        .unwrap_err();
    assert!(matches!(err, ApiError::CategoryGroupNotFound(_)));
}

#[test]
fn test_rename_move_hide_delete_category() {
    let mut svc = BudgetService::open_in_memory().unwrap();
    svc.create_category_group(CreateCategoryGroup {
        id: "grp-1".into(),
        name: "Food".into(),
        is_income: false,
    })
    .unwrap();
    svc.create_category_group(CreateCategoryGroup {
        id: "grp-2".into(),
        name: "Transport".into(),
        is_income: false,
    })
    .unwrap();
    svc.create_category(CreateCategory {
        id: "cat-1".into(),
        name: "Groceries".into(),
        group_id: "grp-1".into(),
        is_income: false,
    })
    .unwrap();

    // Rename
    svc.rename_category(RenameCategory {
        category_id: "cat-1".into(),
        new_name: "Food Shopping".into(),
    })
    .unwrap();
    assert_eq!(svc.category("cat-1").unwrap().name, "Food Shopping");

    // Move to another group
    svc.move_category(MoveCategory {
        category_id: "cat-1".into(),
        new_group_id: "grp-2".into(),
    })
    .unwrap();
    assert_eq!(svc.category("cat-1").unwrap().group_id, "grp-2");

    // Hide
    svc.set_category_visibility(SetCategoryVisibility {
        category_id: "cat-1".into(),
        hidden: true,
    })
    .unwrap();
    assert!(svc.category("cat-1").unwrap().hidden);

    // Delete (tombstone)
    svc.delete_category(DeleteCategory {
        category_id: "cat-1".into(),
    })
    .unwrap();
    assert!(svc.category("cat-1").unwrap().tombstone);
}

// ─── Envelope Budget ───────────────────────────────────────────────────────────

#[test]
fn test_set_budget_and_carryover() {
    let mut svc = BudgetService::open_in_memory().unwrap();
    svc.create_category_group(CreateCategoryGroup {
        id: "grp-1".into(),
        name: "Bills".into(),
        is_income: false,
    })
    .unwrap();
    svc.create_category(CreateCategory {
        id: "cat-1".into(),
        name: "Rent".into(),
        group_id: "grp-1".into(),
        is_income: false,
    })
    .unwrap();

    svc.set_budget(SetBudget {
        month: "2024-06".into(),
        category_id: "cat-1".into(),
        amount: dollars(1500),
    })
    .unwrap();
    assert_eq!(
        svc.budget().get_category_budget("2024-06", "cat-1"),
        dollars(1500)
    );

    svc.set_carryover(SetCarryover {
        month: "2024-06".into(),
        category_id: "cat-1".into(),
        carry: true,
    })
    .unwrap();
    assert!(svc.budget().get_category_carryover("2024-06", "cat-1"));
}

#[test]
fn test_budget_requires_existing_category() {
    let mut svc = BudgetService::open_in_memory().unwrap();
    let err = svc
        .set_budget(SetBudget {
            month: "2024-06".into(),
            category_id: "no-cat".into(),
            amount: dollars(100),
        })
        .unwrap_err();
    assert!(matches!(err, ApiError::CategoryNotFound(_)));
}

// ─── Transaction CRUD ──────────────────────────────────────────────────────────

#[test]
fn test_add_and_read_transaction() {
    let mut svc = BudgetService::open_in_memory().unwrap();
    svc.create_account(CreateAccount {
        id: "acc-1".into(),
        name: "Checking".into(),
        offbudget: false,
    })
    .unwrap();
    let tx_id = svc
        .add_transaction(AddTransaction {
            id: "tx-1".into(),
            account_id: "acc-1".into(),
            date: NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            amount: dollars(-42),
            payee_id: None,
            category_id: None,
            notes: Some("Coffee".into()),
            cleared: true,
        })
        .unwrap();
    assert_eq!(tx_id, "tx-1");
    let txs = svc.transactions();
    assert_eq!(txs.len(), 1);
    assert_eq!(txs[0].amount, dollars(-42));
    assert!(txs[0].cleared);
}

#[test]
fn test_update_transaction() {
    let mut svc = BudgetService::open_in_memory().unwrap();
    svc.create_account(CreateAccount {
        id: "acc-1".into(),
        name: "Checking".into(),
        offbudget: false,
    })
    .unwrap();
    svc.add_transaction(AddTransaction {
        id: "tx-1".into(),
        account_id: "acc-1".into(),
        date: NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
        amount: dollars(-42),
        payee_id: None,
        category_id: None,
        notes: None,
        cleared: false,
    })
    .unwrap();
    svc.update_transaction(UpdateTransaction {
        transaction_id: "tx-1".into(),
        date: None,
        amount: Some(dollars(-100)),
        payee_id: None,
        category_id: None,
        notes: Some(Some("Updated".into())),
        cleared: Some(true),
    })
    .unwrap();
    let txs = svc.transactions();
    assert_eq!(txs[0].amount, dollars(-100));
    assert_eq!(txs[0].notes, Some("Updated".to_string()));
    assert!(txs[0].cleared);
}

#[test]
fn test_delete_transaction() {
    let mut svc = BudgetService::open_in_memory().unwrap();
    svc.create_account(CreateAccount {
        id: "acc-1".into(),
        name: "Checking".into(),
        offbudget: false,
    })
    .unwrap();
    svc.add_transaction(AddTransaction {
        id: "tx-1".into(),
        account_id: "acc-1".into(),
        date: NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
        amount: dollars(-42),
        payee_id: None,
        category_id: None,
        notes: None,
        cleared: false,
    })
    .unwrap();
    svc.delete_transaction(DeleteTransaction {
        transaction_id: "tx-1".into(),
    })
    .unwrap();
    // Tombstoned: not returned by transactions() accessor.
    assert_eq!(svc.transactions().len(), 0);
}

#[test]
fn test_transaction_requires_existing_account() {
    let mut svc = BudgetService::open_in_memory().unwrap();
    let err = svc
        .add_transaction(AddTransaction {
            id: "tx-1".into(),
            account_id: "no-acct".into(),
            date: NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
            amount: dollars(-10),
            payee_id: None,
            category_id: None,
            notes: None,
            cleared: false,
        })
        .unwrap_err();
    assert!(matches!(err, ApiError::AccountNotFound(_)));
}

#[test]
fn test_transactions_for_account_sorted_desc() {
    let mut svc = BudgetService::open_in_memory().unwrap();
    svc.create_account(CreateAccount {
        id: "acc-1".into(),
        name: "Checking".into(),
        offbudget: false,
    })
    .unwrap();
    for day in [1, 15, 28] {
        svc.add_transaction(AddTransaction {
            id: format!("tx-{day}"),
            account_id: "acc-1".into(),
            date: NaiveDate::from_ymd_opt(2024, 6, day).unwrap(),
            amount: dollars(-10),
            payee_id: None,
            category_id: None,
            notes: None,
            cleared: false,
        })
        .unwrap();
    }
    let txs = svc.transactions_for_account("acc-1");
    assert_eq!(txs.len(), 3);
    assert_eq!(txs[0].date.day(), 28); // most recent first
    assert_eq!(txs[2].date.day(), 1);
}

// ─── Split transactions ────────────────────────────────────────────────────────

#[test]
fn test_split_transaction() {
    let mut svc = BudgetService::open_in_memory().unwrap();
    svc.create_account(CreateAccount {
        id: "acc-1".into(),
        name: "Checking".into(),
        offbudget: false,
    })
    .unwrap();
    svc.add_transaction(AddTransaction {
        id: "tx-parent".into(),
        account_id: "acc-1".into(),
        date: NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
        amount: dollars(-100),
        payee_id: None,
        category_id: None,
        notes: None,
        cleared: false,
    })
    .unwrap();

    let child_ids = svc
        .split_transaction(SplitTransaction {
            parent_transaction_id: "tx-parent".into(),
            splits: vec![
                (Some("cat-food".into()), dollars(-60), Some("Food".into())),
                (Some("cat-gas".into()), dollars(-40), Some("Gas".into())),
            ],
        })
        .unwrap();
    assert_eq!(child_ids.len(), 2);
    // All transactions: 1 parent + 2 children = 3.
    assert_eq!(svc.transactions().len(), 3);
}

#[test]
fn test_split_amount_mismatch() {
    let mut svc = BudgetService::open_in_memory().unwrap();
    svc.create_account(CreateAccount {
        id: "acc-1".into(),
        name: "Checking".into(),
        offbudget: false,
    })
    .unwrap();
    svc.add_transaction(AddTransaction {
        id: "tx-parent".into(),
        account_id: "acc-1".into(),
        date: NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
        amount: dollars(-100),
        payee_id: None,
        category_id: None,
        notes: None,
        cleared: false,
    })
    .unwrap();

    let err = svc
        .split_transaction(SplitTransaction {
            parent_transaction_id: "tx-parent".into(),
            splits: vec![
                (None, dollars(-60), None),
                (None, dollars(-30), None), // sums to -90, not -100
            ],
        })
        .unwrap_err();
    assert!(matches!(err, ApiError::SplitAmountMismatch { .. }));
}

// ─── Undo / Redo ───────────────────────────────────────────────────────────────

#[test]
fn test_undo_create_account() {
    let mut svc = BudgetService::open_in_memory().unwrap();
    svc.create_account(CreateAccount {
        id: "acc-1".into(),
        name: "Checking".into(),
        offbudget: false,
    })
    .unwrap();
    assert_eq!(svc.undo_depth(), 1);
    assert_eq!(svc.accounts().len(), 1);

    svc.undo().unwrap();
    // Account is tombstoned after undo — filtered out of accounts().
    assert_eq!(svc.accounts().len(), 0);
    assert_eq!(svc.undo_depth(), 0);
    assert_eq!(svc.redo_depth(), 1);
}

#[test]
fn test_undo_redo_rename_account() {
    let mut svc = BudgetService::open_in_memory().unwrap();
    svc.create_account(CreateAccount {
        id: "acc-1".into(),
        name: "Old".into(),
        offbudget: false,
    })
    .unwrap();
    svc.rename_account(RenameAccount {
        account_id: "acc-1".into(),
        new_name: "New".into(),
    })
    .unwrap();
    assert_eq!(svc.account("acc-1").unwrap().name, "New");

    svc.undo().unwrap(); // undo rename
    assert_eq!(svc.account("acc-1").unwrap().name, "Old");

    svc.redo().unwrap(); // redo rename
    assert_eq!(svc.account("acc-1").unwrap().name, "New");
}

#[test]
fn test_undo_set_budget() {
    let mut svc = BudgetService::open_in_memory().unwrap();
    svc.create_category_group(CreateCategoryGroup {
        id: "grp-1".into(),
        name: "Bills".into(),
        is_income: false,
    })
    .unwrap();
    svc.create_category(CreateCategory {
        id: "cat-1".into(),
        name: "Rent".into(),
        group_id: "grp-1".into(),
        is_income: false,
    })
    .unwrap();

    svc.set_budget(SetBudget {
        month: "2024-06".into(),
        category_id: "cat-1".into(),
        amount: dollars(1000),
    })
    .unwrap();
    assert_eq!(
        svc.budget().get_category_budget("2024-06", "cat-1"),
        dollars(1000)
    );

    svc.set_budget(SetBudget {
        month: "2024-06".into(),
        category_id: "cat-1".into(),
        amount: dollars(1500),
    })
    .unwrap();
    assert_eq!(
        svc.budget().get_category_budget("2024-06", "cat-1"),
        dollars(1500)
    );

    svc.undo().unwrap(); // undo second set_budget → back to 1000
    assert_eq!(
        svc.budget().get_category_budget("2024-06", "cat-1"),
        dollars(1000)
    );

    svc.undo().unwrap(); // undo first set_budget → back to 0
    assert_eq!(
        svc.budget().get_category_budget("2024-06", "cat-1"),
        dollars(0)
    );
}

#[test]
fn test_undo_delete_transaction() {
    let mut svc = BudgetService::open_in_memory().unwrap();
    svc.create_account(CreateAccount {
        id: "acc-1".into(),
        name: "Checking".into(),
        offbudget: false,
    })
    .unwrap();
    svc.add_transaction(AddTransaction {
        id: "tx-1".into(),
        account_id: "acc-1".into(),
        date: NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
        amount: dollars(-42),
        payee_id: None,
        category_id: None,
        notes: None,
        cleared: false,
    })
    .unwrap();
    svc.delete_transaction(DeleteTransaction {
        transaction_id: "tx-1".into(),
    })
    .unwrap();
    assert_eq!(svc.transactions().len(), 0);

    svc.undo().unwrap(); // undo delete → transaction restored
    assert_eq!(svc.transactions().len(), 1);
    assert_eq!(svc.transactions()[0].amount, dollars(-42));
}

#[test]
fn test_undo_update_transaction() {
    let mut svc = BudgetService::open_in_memory().unwrap();
    svc.create_account(CreateAccount {
        id: "acc-1".into(),
        name: "Checking".into(),
        offbudget: false,
    })
    .unwrap();
    svc.add_transaction(AddTransaction {
        id: "tx-1".into(),
        account_id: "acc-1".into(),
        date: NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
        amount: dollars(-42),
        payee_id: None,
        category_id: None,
        notes: Some("original".into()),
        cleared: false,
    })
    .unwrap();
    svc.update_transaction(UpdateTransaction {
        transaction_id: "tx-1".into(),
        date: None,
        amount: Some(dollars(-99)),
        payee_id: None,
        category_id: None,
        notes: Some(Some("changed".into())),
        cleared: None,
    })
    .unwrap();
    assert_eq!(svc.transactions()[0].amount, dollars(-99));

    svc.undo().unwrap();
    assert_eq!(svc.transactions()[0].amount, dollars(-42));
    assert_eq!(svc.transactions()[0].notes, Some("original".to_string()));
}

#[test]
fn test_nothing_to_undo() {
    let mut svc = BudgetService::open_in_memory().unwrap();
    let err = svc.undo().unwrap_err();
    assert!(matches!(err, ApiError::NothingToUndo));
}

#[test]
fn test_nothing_to_redo() {
    let mut svc = BudgetService::open_in_memory().unwrap();
    let err = svc.redo().unwrap_err();
    assert!(matches!(err, ApiError::NothingToRedo));
}

#[test]
fn test_new_operation_clears_redo() {
    let mut svc = BudgetService::open_in_memory().unwrap();
    svc.create_account(CreateAccount {
        id: "acc-1".into(),
        name: "Checking".into(),
        offbudget: false,
    })
    .unwrap();
    svc.undo().unwrap();
    assert_eq!(svc.redo_depth(), 1);

    // A new operation should clear the redo stack.
    svc.create_account(CreateAccount {
        id: "acc-2".into(),
        name: "Savings".into(),
        offbudget: false,
    })
    .unwrap();
    assert_eq!(svc.redo_depth(), 0);
}

// ─── Undo merge payees ─────────────────────────────────────────────────────────

#[test]
fn test_undo_merge_payees() {
    let mut svc = BudgetService::open_in_memory().unwrap();
    svc.create_account(CreateAccount {
        id: "acc-1".into(),
        name: "Checking".into(),
        offbudget: false,
    })
    .unwrap();
    svc.create_payee(CreatePayee {
        id: "pay-a".into(),
        name: "Payee A".into(),
    })
    .unwrap();
    svc.create_payee(CreatePayee {
        id: "pay-b".into(),
        name: "Payee B".into(),
    })
    .unwrap();
    svc.add_transaction(AddTransaction {
        id: "tx-1".into(),
        account_id: "acc-1".into(),
        date: NaiveDate::from_ymd_opt(2024, 6, 15).unwrap(),
        amount: dollars(-50),
        payee_id: Some("pay-a".into()),
        category_id: None,
        notes: None,
        cleared: false,
    })
    .unwrap();

    svc.merge_payees(MergePayees {
        source_payee_id: "pay-a".into(),
        target_payee_id: "pay-b".into(),
    })
    .unwrap();

    // Undo should restore pay-a and re-point transactions.
    svc.undo().unwrap();
    assert!(!svc.payee("pay-a").unwrap().tombstone);
    // Payee A should now be visible in payees().
    assert_eq!(svc.payees().len(), 2);
}

// ─── Undo close / reopen symmetry ─────────────────────────────────────────────

#[test]
fn test_undo_close_account() {
    let mut svc = BudgetService::open_in_memory().unwrap();
    svc.create_account(CreateAccount {
        id: "acc-1".into(),
        name: "Checking".into(),
        offbudget: false,
    })
    .unwrap();
    svc.close_account(CloseAccount {
        account_id: "acc-1".into(),
    })
    .unwrap();
    assert!(svc.account("acc-1").unwrap().closed);

    svc.undo().unwrap();
    assert!(!svc.account("acc-1").unwrap().closed);

    svc.redo().unwrap();
    assert!(svc.account("acc-1").unwrap().closed);
}

// ─── Undo category operations ─────────────────────────────────────────────────

#[test]
fn test_undo_delete_category() {
    let mut svc = BudgetService::open_in_memory().unwrap();
    svc.create_category_group(CreateCategoryGroup {
        id: "grp-1".into(),
        name: "Bills".into(),
        is_income: false,
    })
    .unwrap();
    svc.create_category(CreateCategory {
        id: "cat-1".into(),
        name: "Rent".into(),
        group_id: "grp-1".into(),
        is_income: false,
    })
    .unwrap();
    svc.delete_category(DeleteCategory {
        category_id: "cat-1".into(),
    })
    .unwrap();
    // Tombstoned — still visible via category() but filtered from categories().
    assert!(svc.category("cat-1").unwrap().tombstone);
    assert_eq!(svc.categories().len(), 0);

    svc.undo().unwrap(); // restore
    assert!(!svc.category("cat-1").unwrap().tombstone);
    assert_eq!(svc.categories().len(), 1);
}

// ─── Undo carryover ───────────────────────────────────────────────────────────

#[test]
fn test_undo_carryover() {
    let mut svc = BudgetService::open_in_memory().unwrap();
    svc.create_category_group(CreateCategoryGroup {
        id: "grp-1".into(),
        name: "Bills".into(),
        is_income: false,
    })
    .unwrap();
    svc.create_category(CreateCategory {
        id: "cat-1".into(),
        name: "Rent".into(),
        group_id: "grp-1".into(),
        is_income: false,
    })
    .unwrap();
    svc.set_carryover(SetCarryover {
        month: "2024-06".into(),
        category_id: "cat-1".into(),
        carry: true,
    })
    .unwrap();
    assert!(svc.budget().get_category_carryover("2024-06", "cat-1"));

    svc.undo().unwrap();
    assert!(!svc.budget().get_category_carryover("2024-06", "cat-1"));
}

// ─── Service Report Queries ───────────────────────────────────────────────────

#[test]
fn test_budget_service_reports() {
    let mut svc = BudgetService::open_in_memory().unwrap();

    svc.create_account(CreateAccount {
        id: "acc-1".into(),
        name: "Checking".into(),
        offbudget: false,
    })
    .unwrap();

    svc.create_category_group(CreateCategoryGroup {
        id: "grp-1".into(),
        name: "General".into(),
        is_income: false,
    })
    .unwrap();

    svc.create_category(CreateCategory {
        id: "cat-1".into(),
        name: "Groceries".into(),
        group_id: "grp-1".into(),
        is_income: false,
    })
    .unwrap();

    // Income: $3000
    svc.add_transaction(AddTransaction {
        id: "tx-inc".into(),
        account_id: "acc-1".into(),
        date: NaiveDate::from_ymd_opt(2024, 7, 1).unwrap(),
        amount: dollars(3000),
        payee_id: None,
        category_id: None,
        notes: None,
        cleared: true,
    })
    .unwrap();

    // Expense: -$500 on Groceries
    svc.add_transaction(AddTransaction {
        id: "tx-exp".into(),
        account_id: "acc-1".into(),
        date: NaiveDate::from_ymd_opt(2024, 7, 10).unwrap(),
        amount: dollars(-500),
        payee_id: None,
        category_id: Some("cat-1".into()),
        notes: None,
        cleared: true,
    })
    .unwrap();

    let filter = actual_core::ReportFilter::new();

    // Cash flow
    let cf = svc.cash_flow_report(&filter);
    assert_eq!(cf.total_income, dollars(3000));
    assert_eq!(cf.total_expenses, dollars(500));
    assert_eq!(cf.total_net, dollars(2500));

    // Spending
    let sp = svc.spending_report(&filter);
    assert_eq!(sp.total_spending, dollars(500));
    assert_eq!(sp.groups.len(), 1);
    assert_eq!(sp.groups[0].group_id, "grp-1");
    assert_eq!(sp.groups[0].amount, dollars(500));

    // Net worth
    let mut acct_types = std::collections::HashMap::new();
    acct_types.insert("acc-1".into(), actual_core::AccountType::Asset);
    let nw = svc.net_worth_report(&acct_types, &filter);
    assert_eq!(nw.months.len(), 1);
    assert_eq!(nw.months[0].net_worth, dollars(2500));
}
