//! Automated differential testing suite comparing rs-budget calculations against upstream Actual Budget golden fixtures.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

use actual_core::budget::EnvelopeBudget;
use actual_core::category::{Category, CategoryGroup};
use actual_core::cents::Cents;
use actual_core::transaction::Transaction;

fn get_fixture_path(relative_path: &str) -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .join("../../fixtures/budgets")
        .join(relative_path)
}

// Structures for deserializing standard_envelope_budget.json
#[derive(Debug, Deserialize)]
struct StandardBudgetFixture {
    month: String,
    category_groups: Vec<FixtureGroup>,
    categories: Vec<FixtureCategory>,
    transactions: Vec<FixtureTransaction>,
    expected_calculations: ExpectedStandardCalculations,
}

#[derive(Debug, Deserialize)]
struct FixtureGroup {
    id: String,
    name: String,
    is_income: i64,
}

#[derive(Debug, Deserialize)]
struct FixtureCategory {
    id: String,
    group_id: String,
    name: String,
    is_income: i64,
    budgeted: i64,
}

#[derive(Debug, Deserialize)]
struct FixtureTransaction {
    id: String,
    account_id: String,
    category_id: Option<String>,
    date: String,
    amount: i64,
    payee: Option<String>,
    notes: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExpectedStandardCalculations {
    total_income: i64,
    total_budgeted: i64,
    to_budget: i64,
    category_balances: HashMap<String, ExpectedCategoryBalance>,
}

#[derive(Debug, Deserialize)]
struct ExpectedCategoryBalance {
    budgeted: i64,
    spent: i64,
    balance: i64,
}

#[test]
fn test_differential_standard_envelope_fixture() {
    let fixture_path = get_fixture_path("standard_envelope_budget.json");
    let content = fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("Failed to read fixture from {fixture_path:?}: {e}"));
    let fixture: StandardBudgetFixture = serde_json::from_str(&content).unwrap();

    let mut budget = EnvelopeBudget::new();

    let groups: Vec<CategoryGroup> = fixture
        .category_groups
        .iter()
        .map(|g| {
            let mut grp = CategoryGroup::new(&g.id, &g.name);
            grp.is_income = g.is_income != 0;
            grp
        })
        .collect();

    let categories: Vec<Category> = fixture
        .categories
        .iter()
        .map(|c| {
            let mut cat = Category::new(&c.id, &c.name, &c.group_id);
            cat.is_income = c.is_income != 0;
            cat
        })
        .collect();

    budget
        .init_month(&fixture.month, &categories, &groups)
        .unwrap();

    // Set budgeted amounts
    for c in &fixture.categories {
        if c.budgeted > 0 {
            budget.set_budget(&fixture.month, &c.id, Cents::from_cents(c.budgeted));
        }
    }

    // Apply transactions
    for tx in &fixture.transactions {
        let date = chrono::NaiveDate::parse_from_str(&tx.date, "%Y-%m-%d").unwrap();
        let mut t = Transaction::new(&tx.id, &tx.account_id, date, Cents::from_cents(tx.amount));
        t.category_id = tx.category_id.clone();
        t.payee_id = tx.payee.clone();
        t.notes = tx.notes.clone();
        budget.add_transaction(&t, false).unwrap();
    }

    // Assert exact numerical parity
    let expected = &fixture.expected_calculations;
    assert_eq!(
        budget.get_total_income(&fixture.month),
        Cents::from_cents(expected.total_income),
        "Total income mismatch in standard fixture"
    );
    assert_eq!(
        budget.get_total_budgeted(&fixture.month),
        Cents::from_cents(expected.total_budgeted),
        "Total budgeted mismatch in standard fixture"
    );
    assert_eq!(
        budget.get_to_budget(&fixture.month),
        Cents::from_cents(expected.to_budget),
        "To-budget mismatch in standard fixture"
    );

    for (cat_id, expected_bal) in &expected.category_balances {
        assert_eq!(
            budget.get_category_budget(&fixture.month, cat_id),
            Cents::from_cents(expected_bal.budgeted),
            "Category {cat_id} budgeted mismatch"
        );
        assert_eq!(
            budget.get_category_spent(&fixture.month, cat_id),
            Cents::from_cents(expected_bal.spent),
            "Category {cat_id} spent mismatch"
        );
        assert_eq!(
            budget.get_category_leftover(&fixture.month, cat_id),
            Cents::from_cents(expected_bal.balance),
            "Category {cat_id} leftover mismatch"
        );
    }
}

// Structures for multi-month scenario
#[derive(Debug, Deserialize)]
struct MultiMonthFixture {
    groups: Vec<FixtureGroupBool>,
    categories: Vec<FixtureCategoryBool>,
    months: Vec<FixtureMonth>,
}

#[derive(Debug, Deserialize)]
struct FixtureGroupBool {
    id: String,
    name: String,
    is_income: bool,
}

#[derive(Debug, Deserialize)]
struct FixtureCategoryBool {
    id: String,
    group_id: String,
    name: String,
    is_income: bool,
}

#[derive(Debug, Deserialize)]
struct FixtureMonth {
    month: String,
    uncategorized_income: i64,
    allocations: Vec<FixtureAllocation>,
    expected: ExpectedMonthCalculations,
}

#[derive(Debug, Deserialize)]
struct FixtureAllocation {
    category_id: String,
    budgeted: i64,
    spent: i64,
    carryover: bool,
}

#[derive(Debug, Deserialize)]
struct ExpectedMonthCalculations {
    total_income: i64,
    from_last_month: i64,
    available_funds: i64,
    last_month_overspent: i64,
    total_budgeted: i64,
    total_spent: i64,
    total_leftover: i64,
    to_budget: i64,
    category_balances: HashMap<String, ExpectedCategoryFull>,
    group_balances: HashMap<String, ExpectedGroupFull>,
}

#[derive(Debug, Deserialize)]
struct ExpectedCategoryFull {
    budgeted: i64,
    spent: i64,
    leftover: i64,
    carryover: bool,
}

#[derive(Debug, Deserialize)]
struct ExpectedGroupFull {
    budgeted: i64,
    spent: i64,
    leftover: i64,
}

#[test]
fn test_differential_multi_month_rollover_scenario() {
    let fixture_path = get_fixture_path("multi_month_rollover_scenario.json");
    let content = fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("Failed to read fixture from {fixture_path:?}: {e}"));
    let fixture: MultiMonthFixture = serde_json::from_str(&content).unwrap();

    let mut budget = EnvelopeBudget::new();

    let groups: Vec<CategoryGroup> = fixture
        .groups
        .iter()
        .map(|g| {
            let mut grp = CategoryGroup::new(&g.id, &g.name);
            grp.is_income = g.is_income;
            grp
        })
        .collect();

    let categories: Vec<Category> = fixture
        .categories
        .iter()
        .map(|c| {
            let mut cat = Category::new(&c.id, &c.name, &c.group_id);
            cat.is_income = c.is_income;
            cat
        })
        .collect();

    for m in &fixture.months {
        budget.init_month(&m.month, &categories, &groups).unwrap();
        budget.set_uncategorized_income(&m.month, Cents::from_cents(m.uncategorized_income));

        for alloc in &m.allocations {
            budget.set_budget(
                &m.month,
                &alloc.category_id,
                Cents::from_cents(alloc.budgeted),
            );
            budget.set_spent(&m.month, &alloc.category_id, Cents::from_cents(alloc.spent));
            budget.set_carryover(&m.month, &alloc.category_id, alloc.carryover);
        }

        let expected = &m.expected;
        assert_eq!(
            budget.get_total_income(&m.month),
            Cents::from_cents(expected.total_income),
            "Month {} total_income mismatch",
            m.month
        );
        assert_eq!(
            budget.get_from_last_month(&m.month),
            Cents::from_cents(expected.from_last_month),
            "Month {} from_last_month mismatch",
            m.month
        );
        assert_eq!(
            budget.get_available_funds(&m.month),
            Cents::from_cents(expected.available_funds),
            "Month {} available_funds mismatch",
            m.month
        );
        assert_eq!(
            budget.get_last_month_overspent(&m.month),
            Cents::from_cents(expected.last_month_overspent),
            "Month {} last_month_overspent mismatch",
            m.month
        );
        assert_eq!(
            budget.get_total_budgeted(&m.month),
            Cents::from_cents(expected.total_budgeted),
            "Month {} total_budgeted mismatch",
            m.month
        );
        assert_eq!(
            budget.get_total_spent(&m.month),
            Cents::from_cents(expected.total_spent),
            "Month {} total_spent mismatch",
            m.month
        );
        assert_eq!(
            budget.get_total_leftover(&m.month),
            Cents::from_cents(expected.total_leftover),
            "Month {} total_leftover mismatch",
            m.month
        );
        assert_eq!(
            budget.get_to_budget(&m.month),
            Cents::from_cents(expected.to_budget),
            "Month {} to_budget mismatch",
            m.month
        );

        for (cat_id, exp_cat) in &expected.category_balances {
            assert_eq!(
                budget.get_category_budget(&m.month, cat_id),
                Cents::from_cents(exp_cat.budgeted),
                "Month {} cat {} budgeted mismatch",
                m.month,
                cat_id
            );
            assert_eq!(
                budget.get_category_spent(&m.month, cat_id),
                Cents::from_cents(exp_cat.spent),
                "Month {} cat {} spent mismatch",
                m.month,
                cat_id
            );
            assert_eq!(
                budget.get_category_leftover(&m.month, cat_id),
                Cents::from_cents(exp_cat.leftover),
                "Month {} cat {} leftover mismatch",
                m.month,
                cat_id
            );
            assert_eq!(
                budget.get_category_carryover(&m.month, cat_id),
                exp_cat.carryover,
                "Month {} cat {} carryover mismatch",
                m.month,
                cat_id
            );
        }

        for (grp_id, exp_grp) in &expected.group_balances {
            assert_eq!(
                budget.get_group_budget(&m.month, grp_id),
                Cents::from_cents(exp_grp.budgeted),
                "Month {} group {} budget mismatch",
                m.month,
                grp_id
            );
            assert_eq!(
                budget.get_group_spent(&m.month, grp_id),
                Cents::from_cents(exp_grp.spent),
                "Month {} group {} spent mismatch",
                m.month,
                grp_id
            );
            assert_eq!(
                budget.get_group_leftover(&m.month, grp_id),
                Cents::from_cents(exp_grp.leftover),
                "Month {} group {} leftover mismatch",
                m.month,
                grp_id
            );
        }
    }
}

// Structures for cascading midstream adjustment
#[derive(Debug, Deserialize)]
struct CascadingAdjustmentFixture {
    groups: Vec<FixtureGroupBool>,
    categories: Vec<FixtureCategoryBool>,
    initial: HashMap<String, ExpectedMonthCalculations>,
    modification: FixtureModification,
    after_modification: HashMap<String, ExpectedMonthCalculations>,
}

#[derive(Debug, Deserialize)]
struct FixtureModification {
    month: String,
    category_id: String,
    new_budget: i64,
}

#[test]
fn test_differential_cascading_midstream_adjustment() {
    let fixture_path = get_fixture_path("cascading_midstream_adjustment.json");
    let content = fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("Failed to read fixture from {fixture_path:?}: {e}"));
    let fixture: CascadingAdjustmentFixture = serde_json::from_str(&content).unwrap();

    let mut budget = EnvelopeBudget::new();

    let groups: Vec<CategoryGroup> = fixture
        .groups
        .iter()
        .map(|g| {
            let mut grp = CategoryGroup::new(&g.id, &g.name);
            grp.is_income = g.is_income;
            grp
        })
        .collect();

    let categories: Vec<Category> = fixture
        .categories
        .iter()
        .map(|c| {
            let mut cat = Category::new(&c.id, &c.name, &c.group_id);
            cat.is_income = c.is_income;
            cat
        })
        .collect();

    let months = ["2026-01", "2026-02", "2026-03"];
    for month in months {
        budget.init_month(month, &categories, &groups).unwrap();
        budget.set_uncategorized_income(month, Cents::from_cents(300000));
        budget.set_budget(month, "cat_savings", Cents::from_cents(100000));
        budget.set_budget(month, "cat_leisure", Cents::from_cents(50000));
    }

    // Verify initial state
    for month in months {
        let exp = &fixture.initial[month];
        assert_eq!(
            budget.get_to_budget(month),
            Cents::from_cents(exp.to_budget),
            "Initial to_budget mismatch in {month}"
        );
        assert_eq!(
            budget.get_category_leftover(month, "cat_savings"),
            Cents::from_cents(exp.category_balances["cat_savings"].leftover),
            "Initial cat_savings leftover mismatch in {month}"
        );
    }

    // Execute midstream modification in 2026-01
    let modif = &fixture.modification;
    budget.set_budget(
        &modif.month,
        &modif.category_id,
        Cents::from_cents(modif.new_budget),
    );

    // Verify cascaded state across all months
    for month in months {
        let exp = &fixture.after_modification[month];
        assert_eq!(
            budget.get_to_budget(month),
            Cents::from_cents(exp.to_budget),
            "Post-modification to_budget mismatch in {month}"
        );
        assert_eq!(
            budget.get_category_leftover(month, "cat_savings"),
            Cents::from_cents(exp.category_balances["cat_savings"].leftover),
            "Post-modification cat_savings leftover mismatch in {month}"
        );
        assert_eq!(
            budget.get_available_funds(month),
            Cents::from_cents(exp.available_funds),
            "Post-modification available_funds mismatch in {month}"
        );
    }
}
