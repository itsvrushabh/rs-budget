use actual_core::account::{Account, AccountGroup};
use actual_core::category::{Category, CategoryGroup};
use actual_core::cents::{Cents, MAX_SAFE_CENTS, MIN_SAFE_CENTS};
use actual_core::error::DomainError;
use actual_core::payee::Payee;
use actual_core::transaction::Transaction;
use actual_core::validation::{
    validate_account, validate_category, validate_category_group, validate_payee, validate_split,
    validate_transfer_categorization, validate_transfer_pair,
};
use chrono::NaiveDate;

#[test]
fn test_cents_arithmetic_and_signs() {
    let a = Cents::from_cents(1050); // $10.50
    let b = Cents::from_cents(425); // $4.25

    assert_eq!(a + b, Cents::from_cents(1475));
    assert_eq!(a - b, Cents::from_cents(625));
    assert_eq!(b * 2, Cents::from_cents(850));
    assert_eq!(a / 2, Cents::from_cents(525));
    assert_eq!(-a, Cents::from_cents(-1050));

    assert!(a.is_positive());
    assert!(!a.is_negative());
    assert!(!a.is_zero());

    let zero = Cents::ZERO;
    assert!(zero.is_zero());
    assert!(!zero.is_positive());
    assert!(!zero.is_negative());

    let neg = Cents::from_cents(-500);
    assert!(neg.is_negative());
    assert_eq!(neg.abs(), Cents::from_cents(500));
}

#[test]
fn test_cents_checked_math_and_overflow_protection() {
    let max = Cents::MAX;
    assert_eq!(max.as_cents(), MAX_SAFE_CENTS);

    // Addition exceeding safe bounds fails
    assert_eq!(max.checked_add(Cents::from_cents(1)), None);

    let min = Cents::MIN;
    assert_eq!(min.as_cents(), MIN_SAFE_CENTS);
    assert_eq!(min.checked_sub(Cents::from_cents(1)), None);

    // Checked multiplication overflow
    assert_eq!(max.checked_mul(2), None);

    // Division by zero
    assert_eq!(Cents::from_cents(100).checked_div(0), None);
}

#[test]
fn test_cents_display_and_decimal_string() {
    assert_eq!(Cents::from_cents(0).to_string(), "0.00");
    assert_eq!(Cents::from_cents(5).to_string(), "0.05");
    assert_eq!(Cents::from_cents(50).to_string(), "0.50");
    assert_eq!(Cents::from_cents(1050).to_string(), "10.50");
    assert_eq!(Cents::from_cents(-1050).to_string(), "-10.50");
    assert_eq!(Cents::from_cents(-5).to_string(), "-0.05");
}

#[test]
fn test_cents_parsing_formats() {
    assert_eq!("10.50".parse::<Cents>().unwrap(), Cents::from_cents(1050));
    assert_eq!("$10.50".parse::<Cents>().unwrap(), Cents::from_cents(1050));
    assert_eq!("-10.50".parse::<Cents>().unwrap(), Cents::from_cents(-1050));
    assert_eq!(
        "-$10.50".parse::<Cents>().unwrap(),
        Cents::from_cents(-1050)
    );
    assert_eq!(
        "($10.50)".parse::<Cents>().unwrap(),
        Cents::from_cents(-1050)
    );
    assert_eq!(
        "$1,234.56".parse::<Cents>().unwrap(),
        Cents::from_cents(123456)
    );
    assert_eq!("1234".parse::<Cents>().unwrap(), Cents::from_cents(1234));
    assert_eq!("0.5".parse::<Cents>().unwrap(), Cents::from_cents(50));
    assert_eq!("0.05".parse::<Cents>().unwrap(), Cents::from_cents(5));

    // Invalid parses
    assert!("".parse::<Cents>().is_err());
    assert!("abc".parse::<Cents>().is_err());
    assert!("$".parse::<Cents>().is_err());
}

#[test]
fn test_account_creation_and_activity() {
    let mut acct = Account::new("acct-1", "Checking");
    assert!(acct.is_on_budget());
    assert!(acct.is_active());
    assert!(!acct.is_off_budget());

    acct.closed = true;
    assert!(!acct.is_active());
    assert!(!acct.is_on_budget());

    let tracking = Account::new_offbudget("acct-2", "Mortgage");
    assert!(tracking.is_off_budget());
    assert!(!tracking.is_on_budget());
    assert!(tracking.is_active());

    let group = AccountGroup::new("grp-1", "Bank Accounts");
    assert_eq!(group.name, "Bank Accounts");
}

#[test]
fn test_category_and_category_group() {
    let expense_group = CategoryGroup::new("grp-exp", "Living Expenses");
    assert!(!expense_group.is_income);
    assert!(expense_group.is_active());

    let income_group = CategoryGroup::new_income("grp-inc", "Income");
    assert!(income_group.is_income);

    let rent = Category::new("cat-rent", "Rent", "grp-exp");
    assert!(!rent.is_income);
    assert!(rent.is_active());

    let salary = Category::new_income("cat-sal", "Salary", "grp-inc");
    assert!(salary.is_income);
}

#[test]
fn test_payee_standard_and_transfer() {
    let merchant = Payee::new("payee-1", "Grocery Store");
    assert!(!merchant.is_transfer());
    assert_eq!(merchant.transfer_account_id(), None);
    assert!(merchant.is_active());

    let transfer = Payee::new_transfer("payee-2", "Transfer : Savings", "acct-savings");
    assert!(transfer.is_transfer());
    assert_eq!(transfer.transfer_account_id(), Some("acct-savings"));
}

#[test]
fn test_split_transaction_balancing_and_invariants() {
    let date = NaiveDate::from_ymd_opt(2026, 11, 2).unwrap();
    let mut parent = Transaction::new("tx-parent", "acct-1", date, Cents::from_cents(-10000)); // -$100.00

    let split1 = Transaction::new("tx-split-1", "acct-1", date, Cents::from_cents(-6000)); // -$60.00
    let split2 = Transaction::new("tx-split-2", "acct-1", date, Cents::from_cents(-4000)); // -$40.00

    parent.add_subtransaction(split1);
    parent.add_subtransaction(split2);

    assert!(parent.is_split_parent());
    assert_eq!(parent.calculate_splits_sum(), Cents::from_cents(-10000));
    assert!(validate_split(&parent).is_ok());

    // Mismatched split: child sum -$90.00 vs parent -$100.00
    let mut bad_parent = Transaction::new("tx-bad", "acct-1", date, Cents::from_cents(-10000));
    let bad_split = Transaction::new("tx-bad-1", "acct-1", date, Cents::from_cents(-9000));
    bad_parent.add_subtransaction(bad_split);

    let err = validate_split(&bad_parent).unwrap_err();
    assert!(matches!(err, DomainError::SplitBalanceMismatch { .. }));
}

#[test]
fn test_transfer_pair_balancing_and_invariants() {
    let date = NaiveDate::from_ymd_opt(2026, 11, 2).unwrap();

    let mut tx1 = Transaction::new("tx-1", "acct-checking", date, Cents::from_cents(-5000));
    let mut tx2 = Transaction::new("tx-2", "acct-savings", date, Cents::from_cents(5000));

    tx1.transfer_id = Some("tx-2".to_string());
    tx2.transfer_id = Some("tx-1".to_string());

    assert!(validate_transfer_pair(&tx1, &tx2).is_ok());

    // Unbalanced transfer amounts
    let mut unbalanced_tx2 =
        Transaction::new("tx-2", "acct-savings", date, Cents::from_cents(4000));
    unbalanced_tx2.transfer_id = Some("tx-1".to_string());
    assert!(matches!(
        validate_transfer_pair(&tx1, &unbalanced_tx2),
        Err(DomainError::TransferBalanceMismatch { .. })
    ));

    // Self-transfer to identical account
    let mut self_tx2 = Transaction::new("tx-2", "acct-checking", date, Cents::from_cents(5000));
    self_tx2.transfer_id = Some("tx-1".to_string());
    assert!(matches!(
        validate_transfer_pair(&tx1, &self_tx2),
        Err(DomainError::InvalidTransfer(_))
    ));
}

#[test]
fn test_transfer_categorization_rules() {
    let date = NaiveDate::from_ymd_opt(2026, 11, 2).unwrap();
    let on_budget_1 = Account::new("acct-1", "Checking");
    let on_budget_2 = Account::new("acct-2", "Savings");
    let tracking = Account::new_offbudget("acct-3", "Investment");

    // Case 1: Transfer between two on-budget accounts must NOT have a category
    let mut tx_internal = Transaction::new("tx-int", "acct-1", date, Cents::from_cents(-5000));
    assert!(validate_transfer_categorization(&tx_internal, &on_budget_1, &on_budget_2).is_ok());

    tx_internal.category_id = Some("cat-any".to_string());
    assert!(validate_transfer_categorization(&tx_internal, &on_budget_1, &on_budget_2).is_err());

    // Case 2: Transfer between on-budget and off-budget MUST have a category
    let mut tx_to_tracking = Transaction::new("tx-track", "acct-1", date, Cents::from_cents(-5000));
    assert!(validate_transfer_categorization(&tx_to_tracking, &on_budget_1, &tracking).is_err());

    tx_to_tracking.category_id = Some("cat-investment".to_string());
    assert!(validate_transfer_categorization(&tx_to_tracking, &on_budget_1, &tracking).is_ok());
}

#[test]
fn test_entity_validation_mandatory_fields() {
    let empty_acct = Account::new("", "   ");
    assert!(validate_account(&empty_acct).is_err());

    let empty_cat = Category::new("cat-1", "   ", "grp-1");
    assert!(validate_category(&empty_cat).is_err());

    let empty_grp = CategoryGroup::new("grp-1", "  ");
    assert!(validate_category_group(&empty_grp).is_err());

    let empty_payee = Payee::new("  ", "Valid Name");
    assert!(validate_payee(&empty_payee).is_err());
}

#[test]
fn test_json_serde_roundtrips() {
    let acct = Account::new("acct-1", "Checking");
    let json = serde_json::to_string(&acct).unwrap();
    let deserialized: Account = serde_json::from_str(&json).unwrap();
    assert_eq!(acct, deserialized);

    let date = NaiveDate::from_ymd_opt(2026, 11, 2).unwrap();
    let mut tx = Transaction::new("tx-1", "acct-1", date, Cents::from_cents(4250));
    tx.payee_id = Some("payee-1".to_string());
    tx.category_id = Some("cat-1".to_string());

    let tx_json = serde_json::to_string(&tx).unwrap();
    assert!(tx_json.contains("\"amount\":4250"));

    let deserialized_tx: Transaction = serde_json::from_str(&tx_json).unwrap();
    assert_eq!(tx, deserialized_tx);
}
