use chrono::NaiveDate;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::slice::from_ref;

use actual_core::budget::EnvelopeBudget;
use actual_core::category::{Category, CategoryGroup};
use actual_core::cents::Cents;
use actual_core::rules::{
    Action, ActionField, ActionOp, ActionValue, Condition, ConditionField, ConditionOp,
    ConditionValue, ConditionsOp, Rule, RuleStage, RulesEngine,
};
use actual_core::spreadsheet::{CellValue, Spreadsheet};
use actual_core::transaction::Transaction;

fn bench_cents_arithmetic(c: &mut Criterion) {
    c.bench_function("cents_addition_and_checked_math", |b| {
        let a = Cents::from_cents(123456);
        let d = Cents::from_cents(789012);
        b.iter(|| {
            let res = black_box(a) + black_box(d);
            black_box(res);
        })
    });
}

fn bench_envelope_budget_recalculation(c: &mut Criterion) {
    let mut budget = EnvelopeBudget::new();
    let group = CategoryGroup::new("grp_living", "Living");
    let groceries = Category::new("cat_groceries", "Groceries", "grp_living");
    let rent = Category::new("cat_rent", "Rent", "grp_living");
    let dining = Category::new("cat_dining", "Dining", "grp_living");

    budget
        .init_month("2026-11", &[groceries, rent, dining], from_ref(&group))
        .unwrap();
    budget.set_uncategorized_income("2026-11", Cents::from_cents(400000));
    budget.set_budget("2026-11", "cat_groceries", Cents::from_cents(60000));
    budget.set_spent("2026-11", "cat_groceries", Cents::from_cents(-45000));
    budget.set_budget("2026-11", "cat_rent", Cents::from_cents(120000));
    budget.set_spent("2026-11", "cat_rent", Cents::from_cents(-120000));
    budget.set_budget("2026-11", "cat_dining", Cents::from_cents(20000));
    budget.set_spent("2026-11", "cat_dining", Cents::from_cents(-23000));

    c.bench_function("envelope_budget_single_month_cascade", |b| {
        b.iter(|| {
            let leftover = budget.get_category_leftover("2026-11", "cat_groceries");
            let to_budget = budget.get_to_budget("2026-11");
            let available = budget.get_available_funds("2026-11");
            black_box((leftover, to_budget, available));
        })
    });
}

fn bench_rules_engine_matching(c: &mut Criterion) {
    let mut engine = RulesEngine::new();
    for i in 0..20 {
        let rule = Rule {
            id: format!("rule_{}", i),
            stage: RuleStage::Pre,
            conditions_op: ConditionsOp::And,
            conditions: vec![Condition::new(
                ConditionField::ImportedPayee,
                ConditionOp::Contains,
                ConditionValue::Text(format!("Store_{}", i)),
            )],
            actions: vec![Action::new(
                ActionField::Category,
                ActionOp::Set,
                ActionValue::Text(format!("cat_{}", i)),
            )],
            tombstone: false,
        };
        engine.add_rule(rule);
    }

    let mut tx = Transaction::new(
        "tx_1",
        "acc_checking",
        NaiveDate::from_ymd_opt(2026, 11, 8).unwrap(),
        Cents::from_cents(-3500),
    );
    tx.imported_payee = Some("Store_15 Market".into());

    c.bench_function("rules_engine_apply_20_rules", |b| {
        b.iter(|| {
            let mut test_tx = tx.clone();
            let applied = engine.apply(&mut test_tx);
            black_box(applied);
        })
    });
}

fn bench_spreadsheet_dag(c: &mut Criterion) {
    let mut sheet = Spreadsheet::new();
    sheet.set_static("base_income", Cents::from_cents(500000));
    sheet.set_static("tax_rate_pct", 20i64);
    sheet
        .set_dynamic("tax", &["base_income", "tax_rate_pct"], |args| {
            let income = args[0].as_cents();
            let rate = args[1].as_int();
            let tax_cents = (income.as_cents() * rate) / 100;
            CellValue::Cents(Cents::from_cents(tax_cents))
        })
        .unwrap();

    c.bench_function("spreadsheet_dag_cell_cascade", |b| {
        b.iter(|| {
            let tax = sheet.get_cents("tax");
            black_box(tax);
        })
    });
}

criterion_group!(
    benches,
    bench_cents_arithmetic,
    bench_envelope_budget_recalculation,
    bench_rules_engine_matching,
    bench_spreadsheet_dag
);
criterion_main!(benches);
