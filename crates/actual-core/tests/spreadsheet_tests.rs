use std::slice::from_ref;

use chrono::NaiveDate;

use actual_core::budget::{next_month, parse_year_month, prev_month, EnvelopeBudget};
use actual_core::category::{Category, CategoryGroup};
use actual_core::cents::Cents;
use actual_core::error::DomainError;
use actual_core::spreadsheet::{CellValue, DependencyGraph, Spreadsheet};
use actual_core::transaction::Transaction;

fn dollars(d: i64) -> Cents {
    Cents::from_cents(d * 100)
}

#[test]
fn test_dependency_graph_dag_and_cycle_detection() {
    let mut graph = DependencyGraph::new();

    // Valid DAG: A -> B -> C
    assert!(graph.add_dependency("A", "B").is_ok());
    assert!(graph.add_dependency("B", "C").is_ok());

    // Cycle A -> B -> C -> A
    let cycle_res = graph.add_dependency("C", "A");
    assert!(matches!(cycle_res, Err(DomainError::CycleDetected(_))));

    // Self dependency A -> A
    let self_res = graph.add_dependency("A", "A");
    assert!(matches!(self_res, Err(DomainError::CycleDetected(_))));

    // Diamond DAG: A -> B, A -> C, B -> D, C -> D
    assert!(graph.add_dependency("A", "D_left").is_ok());
    assert!(graph.add_dependency("A", "D_right").is_ok());
    assert!(graph.add_dependency("D_left", "D_sink").is_ok());
    assert!(graph.add_dependency("D_right", "D_sink").is_ok());

    // Topological sort of dependents starting at "A"
    let dependents = graph.topological_sort_dependents(&["A"]).unwrap();
    let pos_d_left = dependents.iter().position(|x| x == "D_left").unwrap();
    let pos_d_right = dependents.iter().position(|x| x == "D_right").unwrap();
    let pos_d_sink = dependents.iter().position(|x| x == "D_sink").unwrap();

    assert!(pos_d_left < pos_d_sink);
    assert!(pos_d_right < pos_d_sink);
}

#[test]
fn test_spreadsheet_engine_reactive_evaluation() {
    let mut sheet = Spreadsheet::new();

    sheet.set_static("base_income", dollars(5000));
    sheet.set_static("tax_rate_pct", 20i64);

    // tax = base_income * tax_rate / 100
    sheet
        .set_dynamic("tax", &["base_income", "tax_rate_pct"], |args| {
            let income = args[0].as_cents();
            let rate = args[1].as_int();
            let tax_cents = (income.as_cents() * rate) / 100;
            CellValue::Cents(Cents::from_cents(tax_cents))
        })
        .unwrap();

    // net_income = base_income - tax
    sheet
        .set_dynamic("net_income", &["base_income", "tax"], |args| {
            let income = args[0].as_cents();
            let tax = args[1].as_cents();
            CellValue::Cents(income - tax)
        })
        .unwrap();

    assert_eq!(sheet.get_cents("tax"), dollars(1000));
    assert_eq!(sheet.get_cents("net_income"), dollars(4000));

    // Update base_income and verify topological cascade
    sheet.set_static("base_income", dollars(6000));
    assert_eq!(sheet.get_cents("tax"), dollars(1200));
    assert_eq!(sheet.get_cents("net_income"), dollars(4800));
}

#[test]
fn test_spreadsheet_transactions_batching() {
    let mut sheet = Spreadsheet::new();

    sheet.set_static("x", 10i64);
    sheet
        .set_dynamic("doubled", &["x"], |args| {
            CellValue::Int(args[0].as_int() * 2)
        })
        .unwrap();

    assert_eq!(sheet.get_int("doubled"), 20);

    let res = sheet.transaction(|s| {
        s.set_static("x", 50i64);
        // Inside transaction, doubled is not recomputed yet
        s.set_static("x", 100i64);
        Ok(())
    });

    assert!(res.is_ok());
    assert_eq!(sheet.get_int("doubled"), 200);
}

#[test]
fn test_month_date_math_utilities() {
    assert_eq!(parse_year_month("2026-11").unwrap(), (2026, 11));
    assert_eq!(prev_month("2026-11").unwrap(), "2026-10");
    assert_eq!(next_month("2026-11").unwrap(), "2026-12");

    // Year rollovers
    assert_eq!(prev_month("2026-01").unwrap(), "2025-12");
    assert_eq!(next_month("2026-12").unwrap(), "2027-01");

    // Invalid format errors
    assert!(matches!(
        parse_year_month("2026/11"),
        Err(DomainError::InvalidMonthFormat(_))
    ));
    assert!(matches!(
        parse_year_month("2026-13"),
        Err(DomainError::InvalidMonthFormat(_))
    ));
    assert!(matches!(
        parse_year_month("2026-00"),
        Err(DomainError::InvalidMonthFormat(_))
    ));
    assert!(matches!(
        parse_year_month("invalid"),
        Err(DomainError::InvalidMonthFormat(_))
    ));
}

#[test]
fn test_envelope_budget_single_month_allocations() {
    let mut budget = EnvelopeBudget::new();

    let food_group = CategoryGroup::new("grp_food", "Food & Living");
    let groceries = Category::new("cat_groc", "Groceries", "grp_food");
    let dining = Category::new("cat_dine", "Dining Out", "grp_food");

    budget
        .init_month("2026-11", &[groceries, dining], from_ref(&food_group))
        .unwrap();

    // Income of $4,000
    budget.set_uncategorized_income("2026-11", dollars(4000));
    assert_eq!(budget.get_available_funds("2026-11"), dollars(4000));
    assert_eq!(budget.get_to_budget("2026-11"), dollars(4000));

    // Budget $600 for Groceries and $200 for Dining Out
    budget.set_budget("2026-11", "cat_groc", dollars(600));
    budget.set_budget("2026-11", "cat_dine", dollars(200));

    assert_eq!(budget.get_total_budgeted("2026-11"), dollars(800));
    assert_eq!(budget.get_group_budget("2026-11", "grp_food"), dollars(800));
    assert_eq!(budget.get_to_budget("2026-11"), dollars(3200));

    // Spend $450 in Groceries (-$450)
    budget.set_spent("2026-11", "cat_groc", dollars(-450));
    assert_eq!(
        budget.get_category_leftover("2026-11", "cat_groc"),
        dollars(150)
    );
    assert_eq!(budget.get_group_spent("2026-11", "grp_food"), dollars(-450));
    assert_eq!(
        budget.get_group_leftover("2026-11", "grp_food"),
        dollars(350)
    );
}

#[test]
fn test_envelope_budget_positive_rollover() {
    let mut budget = EnvelopeBudget::new();

    let housing = CategoryGroup::new("grp_house", "Housing");
    let utilities = Category::new("cat_util", "Utilities", "grp_house");

    // Month 1: 2026-01
    budget
        .init_month("2026-01", from_ref(&utilities), from_ref(&housing))
        .unwrap();
    budget.set_uncategorized_income("2026-01", dollars(2000));
    budget.set_budget("2026-01", "cat_util", dollars(300));
    budget.set_spent("2026-01", "cat_util", dollars(-200));

    // Month 1 leftover for utilities = $100.
    assert_eq!(
        budget.get_category_leftover("2026-01", "cat_util"),
        dollars(100)
    );
    // Month 1 To Budget = $2000 - $300 = $1700.
    assert_eq!(budget.get_to_budget("2026-01"), dollars(1700));

    // Month 2: 2026-02
    budget
        .init_month("2026-02", from_ref(&utilities), from_ref(&housing))
        .unwrap();

    // From last month brings over the $1700 unbudgeted funds
    assert_eq!(budget.get_from_last_month("2026-02"), dollars(1700));
    assert_eq!(budget.get_available_funds("2026-02"), dollars(1700));

    // Utilities category envelope starts with $100 carried over from January
    assert_eq!(
        budget.get_category_leftover("2026-02", "cat_util"),
        dollars(100)
    );

    // Budget another $250 in February -> leftover becomes $100 + $250 = $350
    budget.set_budget("2026-02", "cat_util", dollars(250));
    assert_eq!(
        budget.get_category_leftover("2026-02", "cat_util"),
        dollars(350)
    );
    assert_eq!(budget.get_to_budget("2026-02"), dollars(1450));
}

#[test]
fn test_envelope_budget_overspending_without_carryover() {
    let mut budget = EnvelopeBudget::new();

    let food_group = CategoryGroup::new("grp_food", "Food");
    let dining = Category::new("cat_dine", "Dining Out", "grp_food");

    // Month 1: 2026-01
    budget
        .init_month("2026-01", from_ref(&dining), from_ref(&food_group))
        .unwrap();
    budget.set_uncategorized_income("2026-01", dollars(1000));
    budget.set_budget("2026-01", "cat_dine", dollars(100));
    // Overspend by $50 (spent $150)
    budget.set_spent("2026-01", "cat_dine", dollars(-150));

    // Leftover is -$50 in Month 1
    assert_eq!(
        budget.get_category_leftover("2026-01", "cat_dine"),
        dollars(-50)
    );
    // Month 1 to-budget is 1000 - 100 = 900
    assert_eq!(budget.get_to_budget("2026-01"), dollars(900));

    // Month 2: 2026-02 (carryover is false by default)
    budget
        .init_month("2026-02", from_ref(&dining), from_ref(&food_group))
        .unwrap();

    // Since carryover is false, the negative leftover does NOT carry into the envelope; envelope resets to $0
    assert_eq!(
        budget.get_category_leftover("2026-02", "cat_dine"),
        Cents::ZERO
    );

    // The $50 overspending is deducted from Month 2's available funds via last_month_overspent
    assert_eq!(budget.get_last_month_overspent("2026-02"), dollars(-50));
    // Available = 900, Overspent = -50, To Budget = 900 - 50 = 850
    assert_eq!(budget.get_to_budget("2026-02"), dollars(850));
}

#[test]
fn test_envelope_budget_overspending_with_carryover() {
    let mut budget = EnvelopeBudget::new();

    let food_group = CategoryGroup::new("grp_food", "Food");
    let dining = Category::new("cat_dine", "Dining Out", "grp_food");

    // Month 1: 2026-01
    budget
        .init_month("2026-01", from_ref(&dining), from_ref(&food_group))
        .unwrap();
    budget.set_uncategorized_income("2026-01", dollars(1000));
    budget.set_budget("2026-01", "cat_dine", dollars(100));
    budget.set_spent("2026-01", "cat_dine", dollars(-150));
    // Turn carryover ON for Dining Out in Month 1
    budget.set_carryover("2026-01", "cat_dine", true);

    // Month 2: 2026-02
    budget
        .init_month("2026-02", from_ref(&dining), from_ref(&food_group))
        .unwrap();

    // With carryover enabled, the -$50 stays inside the Dining Out envelope
    assert_eq!(
        budget.get_category_leftover("2026-02", "cat_dine"),
        dollars(-50)
    );

    // And last_month_overspent is NOT deducted from To Budget
    assert_eq!(budget.get_last_month_overspent("2026-02"), Cents::ZERO);
    assert_eq!(budget.get_to_budget("2026-02"), dollars(900));
}

#[test]
fn test_multi_month_forward_cascade() {
    let mut budget = EnvelopeBudget::new();

    let grp = CategoryGroup::new("grp", "General");
    let cat = Category::new("cat", "Savings", "grp");

    budget
        .init_month("2026-01", from_ref(&cat), from_ref(&grp))
        .unwrap();
    budget
        .init_month("2026-02", from_ref(&cat), from_ref(&grp))
        .unwrap();
    budget
        .init_month("2026-03", from_ref(&cat), from_ref(&grp))
        .unwrap();

    budget.set_uncategorized_income("2026-01", dollars(3000));
    budget.set_budget("2026-01", "cat", dollars(1000));

    // Jan leftover = 1000, Jan to_budget = 2000
    assert_eq!(budget.get_to_budget("2026-01"), dollars(2000));
    // Feb available = 2000, Feb to_budget = 2000, Feb cat leftover = 1000
    assert_eq!(budget.get_to_budget("2026-02"), dollars(2000));
    assert_eq!(
        budget.get_category_leftover("2026-02", "cat"),
        dollars(1000)
    );
    // Mar available = 2000, Mar to_budget = 2000, Mar cat leftover = 1000
    assert_eq!(budget.get_to_budget("2026-03"), dollars(2000));
    assert_eq!(
        budget.get_category_leftover("2026-03", "cat"),
        dollars(1000)
    );

    // Now update January budget from 1000 to 1500
    budget.set_budget("2026-01", "cat", dollars(1500));

    // January updates
    assert_eq!(budget.get_to_budget("2026-01"), dollars(1500));
    assert_eq!(
        budget.get_category_leftover("2026-01", "cat"),
        dollars(1500)
    );

    // Cascades into February
    assert_eq!(budget.get_to_budget("2026-02"), dollars(1500));
    assert_eq!(
        budget.get_category_leftover("2026-02", "cat"),
        dollars(1500)
    );

    // Cascades into March
    assert_eq!(budget.get_to_budget("2026-03"), dollars(1500));
    assert_eq!(
        budget.get_category_leftover("2026-03", "cat"),
        dollars(1500)
    );
}

#[test]
fn test_transaction_integration_and_splits() {
    let mut budget = EnvelopeBudget::new();

    let grp = CategoryGroup::new("grp_living", "Living");
    let rent = Category::new("cat_rent", "Rent", "grp_living");
    let supplies = Category::new("cat_sup", "Supplies", "grp_living");

    budget
        .init_month("2026-11", &[rent, supplies], from_ref(&grp))
        .unwrap();

    budget.set_uncategorized_income("2026-11", dollars(5000));
    budget.set_budget("2026-11", "cat_rent", dollars(1500));
    budget.set_budget("2026-11", "cat_sup", dollars(300));

    // 1. Standard single transaction: Rent payment -$1500
    let mut tx_rent = Transaction::new(
        "tx_1",
        "acct_checking",
        NaiveDate::from_ymd_opt(2026, 11, 5).unwrap(),
        dollars(-1500),
    );
    tx_rent.category_id = Some("cat_rent".to_string());
    budget.add_transaction(&tx_rent, false).unwrap();

    assert_eq!(
        budget.get_category_spent("2026-11", "cat_rent"),
        dollars(-1500)
    );
    assert_eq!(
        budget.get_category_leftover("2026-11", "cat_rent"),
        Cents::ZERO
    );

    // 2. Split transaction: Superstore total -$250, split into -$200 Supplies and -$50 uncategorized/cashback
    let mut parent_tx = Transaction::new(
        "tx_parent",
        "acct_checking",
        NaiveDate::from_ymd_opt(2026, 11, 10).unwrap(),
        dollars(-250),
    );

    let mut sub_1 = Transaction::new(
        "sub_1",
        "acct_checking",
        NaiveDate::from_ymd_opt(2026, 11, 10).unwrap(),
        dollars(-200),
    );
    sub_1.category_id = Some("cat_sup".to_string());

    let sub_2 = Transaction::new(
        "sub_2",
        "acct_checking",
        NaiveDate::from_ymd_opt(2026, 11, 10).unwrap(),
        dollars(-50),
    );

    parent_tx.add_subtransaction(sub_1);
    parent_tx.add_subtransaction(sub_2);

    budget.add_transaction(&parent_tx, false).unwrap();

    assert_eq!(
        budget.get_category_spent("2026-11", "cat_sup"),
        dollars(-200)
    );
    assert_eq!(
        budget.get_category_leftover("2026-11", "cat_sup"),
        dollars(100)
    );
}
