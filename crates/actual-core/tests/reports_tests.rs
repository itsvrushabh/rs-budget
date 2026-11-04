//! Integration tests for financial reports and analytics engine (Issue #37).

use std::collections::HashMap;

use actual_core::reports::{
    AccountType, CashFlowReport, NetWorthReport, ReportFilter, SpendingReport,
};
use actual_core::{Account, Category, CategoryGroup, Cents, Transaction};
use chrono::NaiveDate;

fn dollars(d: i64) -> Cents {
    Cents::from_cents(d * 100)
}

fn create_account(id: &str, name: &str) -> Account {
    Account::new(id, name)
}

fn create_tx(
    id: &str,
    account_id: &str,
    date: (i32, u32, u32),
    amount: Cents,
    category_id: Option<&str>,
    payee_id: Option<&str>,
) -> Transaction {
    let mut tx = Transaction::new(
        id,
        account_id,
        NaiveDate::from_ymd_opt(date.0, date.1, date.2).unwrap(),
        amount,
    );
    tx.category_id = category_id.map(String::from);
    tx.payee_id = payee_id.map(String::from);
    tx
}

// ─── Net Worth Tests ───────────────────────────────────────────────────────────

#[test]
fn test_net_worth_progression() {
    let checking = create_account("acc-checking", "Checking");
    let savings = create_account("acc-savings", "Savings");
    let card = create_account("acc-card", "Credit Card");
    let accounts = vec![checking, savings, card];

    let mut account_types = HashMap::new();
    account_types.insert("acc-checking".to_string(), AccountType::Asset);
    account_types.insert("acc-savings".to_string(), AccountType::Asset);
    account_types.insert("acc-card".to_string(), AccountType::Liability);

    let transactions = vec![
        // Jan 2024: $5,000 income to checking, -$500 credit card purchase
        create_tx(
            "t1",
            "acc-checking",
            (2024, 1, 5),
            dollars(5000),
            None,
            None,
        ),
        create_tx("t2", "acc-card", (2024, 1, 10), dollars(-500), None, None),
        // Feb 2024: $2,000 moved from checking to savings, -$200 on credit card
        create_tx(
            "t3",
            "acc-checking",
            (2024, 2, 1),
            dollars(-2000),
            None,
            None,
        ),
        create_tx("t4", "acc-savings", (2024, 2, 1), dollars(2000), None, None),
        create_tx("t5", "acc-card", (2024, 2, 15), dollars(-200), None, None),
        // Mar 2024: $3,000 new income, $700 card payment
        create_tx(
            "t6",
            "acc-checking",
            (2024, 3, 5),
            dollars(3000),
            None,
            None,
        ),
        create_tx(
            "t7",
            "acc-checking",
            (2024, 3, 20),
            dollars(-700),
            None,
            None,
        ),
        create_tx("t8", "acc-card", (2024, 3, 20), dollars(700), None, None),
    ];

    let filter = ReportFilter::new();
    let report = NetWorthReport::compute(&transactions, &accounts, &account_types, &filter);

    assert_eq!(report.months.len(), 3);

    // Jan 2024: Assets = 5000, Liabilities = -500, Net Worth = 4500
    let jan = &report.months[0];
    assert_eq!(jan.month, "2024-01");
    assert_eq!(jan.total_assets, dollars(5000));
    assert_eq!(jan.total_liabilities, dollars(-500));
    assert_eq!(jan.net_worth, dollars(4500));

    // Feb 2024: Assets = 3000 (checking) + 2000 (savings) = 5000, Liabilities = -700, Net Worth = 4300
    let feb = &report.months[1];
    assert_eq!(feb.month, "2024-02");
    assert_eq!(feb.total_assets, dollars(5000));
    assert_eq!(feb.total_liabilities, dollars(-700));
    assert_eq!(feb.net_worth, dollars(4300));

    // Mar 2024: Assets = (3000+3000-700) + 2000 = 7300, Liabilities = -700 + 700 = 0, Net Worth = 7300
    let mar = &report.months[2];
    assert_eq!(mar.month, "2024-03");
    assert_eq!(mar.total_assets, dollars(7300));
    assert_eq!(mar.total_liabilities, dollars(0));
    assert_eq!(mar.net_worth, dollars(7300));
}

#[test]
fn test_net_worth_with_date_filter() {
    let checking = create_account("acc-1", "Checking");
    let accounts = vec![checking];
    let mut account_types = HashMap::new();
    account_types.insert("acc-1".to_string(), AccountType::Asset);

    let transactions = vec![
        create_tx("t1", "acc-1", (2023, 12, 1), dollars(1000), None, None),
        create_tx("t2", "acc-1", (2024, 1, 15), dollars(500), None, None),
        create_tx("t3", "acc-1", (2024, 2, 15), dollars(300), None, None),
    ];

    // Filter only Jan 2024 to Feb 2024; running balance must include Dec 2023!
    let filter = ReportFilter::new().with_date_range(
        NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
        NaiveDate::from_ymd_opt(2024, 2, 28).unwrap(),
    );

    let report = NetWorthReport::compute(&transactions, &accounts, &account_types, &filter);
    assert_eq!(report.months.len(), 2);
    // End of Jan: 1000 (prior) + 500 = 1500
    assert_eq!(report.months[0].net_worth, dollars(1500));
    // End of Feb: 1500 + 300 = 1800
    assert_eq!(report.months[1].net_worth, dollars(1800));
}

// ─── Cash Flow Tests ──────────────────────────────────────────────────────────

#[test]
fn test_cash_flow_analytics() {
    let mut transfer_tx = create_tx("t_tr", "acc-1", (2024, 1, 15), dollars(-500), None, None);
    transfer_tx.transfer_id = Some("transfer-pair-1".to_string());

    let transactions = vec![
        // Jan: Income $4,000, Expense $2,500, Transfer $500 (should be excluded)
        create_tx("t1", "acc-1", (2024, 1, 1), dollars(4000), None, None),
        create_tx("t2", "acc-1", (2024, 1, 10), dollars(-1500), None, None),
        create_tx("t3", "acc-1", (2024, 1, 20), dollars(-1000), None, None),
        transfer_tx,
        // Feb: Income $4,000, Expense $1,000
        create_tx("t4", "acc-1", (2024, 2, 1), dollars(4000), None, None),
        create_tx("t5", "acc-1", (2024, 2, 10), dollars(-1000), None, None),
    ];

    let filter = ReportFilter::new();
    let report = CashFlowReport::compute(&transactions, &filter);

    assert_eq!(report.months.len(), 2);

    // Jan
    let jan = &report.months[0];
    assert_eq!(jan.month, "2024-01");
    assert_eq!(jan.income, dollars(4000));
    assert_eq!(jan.expenses, dollars(2500));
    assert_eq!(jan.net, dollars(1500));
    // savings rate: 1500 / 4000 = 37.5%
    assert!((jan.savings_rate - 37.5).abs() < 1e-4);

    // Feb
    let feb = &report.months[1];
    assert_eq!(feb.month, "2024-02");
    assert_eq!(feb.income, dollars(4000));
    assert_eq!(feb.expenses, dollars(1000));
    assert_eq!(feb.net, dollars(3000));
    // savings rate: 3000 / 4000 = 75.0%
    assert!((feb.savings_rate - 75.0).abs() < 1e-4);

    // Totals
    assert_eq!(report.total_income, dollars(8000));
    assert_eq!(report.total_expenses, dollars(3500));
    assert_eq!(report.total_net, dollars(4500));
    // overall savings rate: 4500 / 8000 = 56.25%
    assert!((report.overall_savings_rate - 56.25).abs() < 1e-4);
}

// ─── Spending Breakdown Tests ─────────────────────────────────────────────────

#[test]
fn test_spending_breakdown_by_category_and_group() {
    let grp_living = CategoryGroup::new("grp-living", "Living Expenses");
    let grp_leisure = CategoryGroup::new("grp-leisure", "Leisure");
    let groups = vec![grp_living, grp_leisure];

    let cat_rent = Category::new("cat-rent", "Rent", "grp-living");
    let cat_groceries = Category::new("cat-groceries", "Groceries", "grp-living");
    let cat_games = Category::new("cat-games", "Games", "grp-leisure");
    let categories = vec![cat_rent, cat_groceries, cat_games];

    let transactions = vec![
        create_tx(
            "t1",
            "acc-1",
            (2024, 1, 1),
            dollars(-1000),
            Some("cat-rent"),
            None,
        ),
        create_tx(
            "t2",
            "acc-1",
            (2024, 1, 5),
            dollars(-600),
            Some("cat-groceries"),
            None,
        ),
        create_tx(
            "t3",
            "acc-1",
            (2024, 1, 12),
            dollars(-400),
            Some("cat-games"),
            None,
        ),
        // Income transaction should be ignored for spending
        create_tx("t4", "acc-1", (2024, 1, 15), dollars(5000), None, None),
    ];

    let filter = ReportFilter::new();
    let report = SpendingReport::compute(&transactions, &categories, &groups, &filter);

    assert_eq!(report.total_spending, dollars(2000));
    assert_eq!(report.groups.len(), 2);

    // Group 1: Living Expenses ($1,600 = 80%)
    let living = &report.groups[0];
    assert_eq!(living.group_id, "grp-living");
    assert_eq!(living.amount, dollars(1600));
    assert!((living.percentage - 80.0).abs() < 1e-4);
    assert_eq!(living.categories.len(), 2);
    // Categories within Living Expenses sorted descending: Rent ($1000), then Groceries ($600)
    assert_eq!(living.categories[0].category_id, "cat-rent");
    assert_eq!(living.categories[0].amount, dollars(1000));
    assert!((living.categories[0].percentage - 50.0).abs() < 1e-4);
    assert_eq!(living.categories[1].category_id, "cat-groceries");
    assert_eq!(living.categories[1].amount, dollars(600));
    assert!((living.categories[1].percentage - 30.0).abs() < 1e-4);

    // Group 2: Leisure ($400 = 20%)
    let leisure = &report.groups[1];
    assert_eq!(leisure.group_id, "grp-leisure");
    assert_eq!(leisure.amount, dollars(400));
    assert!((leisure.percentage - 20.0).abs() < 1e-4);
}

// ─── Filter Tests ─────────────────────────────────────────────────────────────

#[test]
fn test_filter_matching() {
    let mut tx = create_tx(
        "t1",
        "acc-1",
        (2024, 5, 10),
        dollars(-50),
        Some("cat-1"),
        Some("pay-1"),
    );

    let filter_all = ReportFilter::new();
    assert!(filter_all.matches(&tx));

    // Account filter
    let f_acc_match = ReportFilter::new().with_accounts(vec!["acc-1".into()]);
    let f_acc_miss = ReportFilter::new().with_accounts(vec!["acc-2".into()]);
    assert!(f_acc_match.matches(&tx));
    assert!(!f_acc_miss.matches(&tx));

    // Category filter
    let f_cat_match = ReportFilter::new().with_categories(vec!["cat-1".into()]);
    let f_cat_miss = ReportFilter::new().with_categories(vec!["cat-2".into()]);
    assert!(f_cat_match.matches(&tx));
    assert!(!f_cat_miss.matches(&tx));

    // Payee filter
    let f_pay_match = ReportFilter::new().with_payees(vec!["pay-1".into()]);
    let f_pay_miss = ReportFilter::new().with_payees(vec!["pay-2".into()]);
    assert!(f_pay_match.matches(&tx));
    assert!(!f_pay_miss.matches(&tx));

    // Tombstone check
    tx.tombstone = true;
    assert!(!filter_all.matches(&tx));
}
