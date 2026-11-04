//! Recurring schedule command handlers.

use actual_api::commands::GetUpcomingTransactions;
use actual_api::BudgetService;
use chrono::NaiveDate;
use serde::Serialize;

use crate::cli::ScheduleSubcommand;
use crate::output::{format_dollars, print_json};

#[derive(Serialize)]
struct UpcomingView {
    date: String,
    account_id: String,
    amount: String,
    notes: Option<String>,
}

pub fn handle(svc: &BudgetService, cmd: ScheduleSubcommand, json: bool) -> Result<(), String> {
    match cmd {
        ScheduleSubcommand::List => {
            if json {
                print_json(&serde_json::json!({
                    "status": "ok",
                    "schedules_count": 0,
                    "schedules": []
                }));
            } else {
                println!("No recurring schedules registered.");
            }
            Ok(())
        }
        ScheduleSubcommand::Upcoming { id, until } => {
            let until_date = NaiveDate::parse_from_str(&until, "%Y-%m-%d")
                .map_err(|e| format!("Invalid until date: {e}"))?;

            let res = svc
                .get_upcoming_transactions(GetUpcomingTransactions {
                    schedule_id: id.clone(),
                    from_date: chrono::Local::now().date_naive(),
                    until_date,
                })
                .map_err(|e| e.to_string())?;

            let list: Vec<UpcomingView> = res
                .transactions
                .into_iter()
                .map(|t| UpcomingView {
                    date: t.date.to_string(),
                    account_id: t.account_id,
                    amount: format_dollars(t.amount),
                    notes: t.notes,
                })
                .collect();

            if json {
                print_json(&list);
            } else {
                println!("Upcoming transactions for schedule '{id}' until {until}:");
                println!("{:<12} {:<36} {:>14}", "DATE", "ACCOUNT", "AMOUNT");
                println!("{:-<12} {:-<36} {:->14}", "", "", "");
                for item in &list {
                    println!(
                        "{:<12} {:<36} {:>14}",
                        item.date, item.account_id, item.amount
                    );
                }
            }
            Ok(())
        }
    }
}
