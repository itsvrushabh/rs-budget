//! Budget inspection and allocation handlers.

use actual_api::commands::{SetBudget, SetCarryover};
use actual_api::BudgetService;
use actual_core::Cents;
use chrono::Local;
use serde::Serialize;

use crate::cli::BudgetSubcommand;
use crate::output::{format_dollars, print_json};

#[derive(Serialize)]
struct CategoryView {
    id: String,
    name: String,
    group: String,
    budgeted: String,
    spent: String,
    leftover: String,
    carryover: bool,
}

#[derive(Serialize)]
struct BudgetSummaryView {
    month: String,
    to_budget: String,
    total_budgeted: String,
    total_spent: String,
    available_funds: String,
    categories: Vec<CategoryView>,
}

pub fn handle(svc: &mut BudgetService, cmd: BudgetSubcommand, json: bool) -> Result<(), String> {
    match cmd {
        BudgetSubcommand::Inspect { month } => {
            let m = month.unwrap_or_else(|| Local::now().format("%Y-%m").to_string());
            let budget = svc.budget();

            let to_budget = budget.get_to_budget(&m);
            let total_budgeted = budget.get_total_budgeted(&m);
            let total_spent = budget.get_total_spent(&m);
            let available_funds = budget.get_available_funds(&m);

            let mut categories_view = Vec::new();
            for cat in svc.categories() {
                let grp_name = svc
                    .category_group(&cat.group_id)
                    .map(|g| g.name.clone())
                    .unwrap_or_else(|| "Ungrouped".to_string());

                let b = budget.get_category_budget(&m, &cat.id);
                let s = budget.get_category_spent(&m, &cat.id);
                let l = budget.get_category_leftover(&m, &cat.id);
                let c = budget.get_category_carryover(&m, &cat.id);

                categories_view.push(CategoryView {
                    id: cat.id.clone(),
                    name: cat.name.clone(),
                    group: grp_name,
                    budgeted: format_dollars(b),
                    spent: format_dollars(s),
                    leftover: format_dollars(l),
                    carryover: c,
                });
            }

            if json {
                let summary = BudgetSummaryView {
                    month: m,
                    to_budget: format_dollars(to_budget),
                    total_budgeted: format_dollars(total_budgeted),
                    total_spent: format_dollars(total_spent),
                    available_funds: format_dollars(available_funds),
                    categories: categories_view,
                };
                print_json(&summary);
            } else {
                println!("=== Budget Summary for {m} ===");
                println!("  To Budget:       {:>14}", format_dollars(to_budget));
                println!("  Total Budgeted:  {:>14}", format_dollars(total_budgeted));
                println!("  Total Spent:     {:>14}", format_dollars(total_spent));
                println!("  Available Funds: {:>14}", format_dollars(available_funds));
                println!();
                println!(
                    "{:<24} {:<18} {:>12} {:>12} {:>12} {:>8}",
                    "CATEGORY", "GROUP", "BUDGETED", "SPENT", "LEFTOVER", "CARRY"
                );
                println!(
                    "{:-<24} {:-<18} {:->12} {:->12} {:->12} {:->8}",
                    "", "", "", "", "", ""
                );
                for c in &categories_view {
                    println!(
                        "{:<24} {:<18} {:>12} {:>12} {:>12} {:>8}",
                        c.name,
                        c.group,
                        c.budgeted,
                        c.spent,
                        c.leftover,
                        if c.carryover { "yes" } else { "no" }
                    );
                }
            }
            Ok(())
        }
        BudgetSubcommand::Set {
            month,
            category,
            amount,
        } => {
            // Find category by ID or name
            let cat_id = svc
                .categories()
                .iter()
                .find(|c| c.id == category || c.name.eq_ignore_ascii_case(&category))
                .map(|c| c.id.clone())
                .unwrap_or(category);

            let cents = Cents::from_cents((amount * 100.0).round() as i64);
            svc.set_budget(SetBudget {
                month: month.clone(),
                category_id: cat_id.clone(),
                amount: cents,
            })
            .map_err(|e| e.to_string())?;

            if json {
                print_json(&serde_json::json!({
                    "status": "ok",
                    "month": month,
                    "category_id": cat_id,
                    "amount": format_dollars(cents),
                }));
            } else {
                println!(
                    "Set budget for category '{cat_id}' in {month} to {}",
                    format_dollars(cents)
                );
            }
            Ok(())
        }
        BudgetSubcommand::Carryover {
            month,
            category,
            carry,
        } => {
            let cat_id = svc
                .categories()
                .iter()
                .find(|c| c.id == category || c.name.eq_ignore_ascii_case(&category))
                .map(|c| c.id.clone())
                .unwrap_or(category);

            svc.set_carryover(SetCarryover {
                month: month.clone(),
                category_id: cat_id.clone(),
                carry,
            })
            .map_err(|e| e.to_string())?;

            if json {
                print_json(&serde_json::json!({
                    "status": "ok",
                    "month": month,
                    "category_id": cat_id,
                    "carryover": carry,
                }));
            } else {
                println!("Updated carryover for category '{cat_id}' in {month} to {carry}");
            }
            Ok(())
        }
    }
}
