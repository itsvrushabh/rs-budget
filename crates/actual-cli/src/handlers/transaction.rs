//! Transaction command handlers.

use actual_api::commands::{AddTransaction, DeleteTransaction};
use actual_api::BudgetService;
use actual_core::Cents;
use chrono::{Local, NaiveDate};
use serde::Serialize;

use crate::cli::TransactionSubcommand;
use crate::output::{format_dollars, print_json};

#[derive(Serialize)]
struct TransactionView {
    id: String,
    date: String,
    account_id: String,
    payee: Option<String>,
    category: Option<String>,
    amount: String,
    cleared: bool,
    notes: Option<String>,
}

pub fn handle(
    svc: &mut BudgetService,
    cmd: TransactionSubcommand,
    json: bool,
) -> Result<(), String> {
    match cmd {
        TransactionSubcommand::List {
            account,
            month,
            limit,
        } => {
            let mut txs = if let Some(ref acct_id) = account {
                svc.transactions_for_account(acct_id)
            } else {
                let mut all = svc.transactions();
                all.sort_by_key(|t| std::cmp::Reverse(t.date));
                all
            };

            if let Some(ref m) = month {
                txs.retain(|t| t.date.format("%Y-%m").to_string() == *m);
            }

            let view: Vec<TransactionView> = txs
                .into_iter()
                .take(limit)
                .map(|t| {
                    let payee_name = t
                        .payee_id
                        .as_deref()
                        .and_then(|id| svc.payee(id))
                        .map(|p| p.name.clone());
                    let cat_name = t
                        .category_id
                        .as_deref()
                        .and_then(|id| svc.category(id))
                        .map(|c| c.name.clone());

                    TransactionView {
                        id: t.id.clone(),
                        date: t.date.to_string(),
                        account_id: t.account_id.clone(),
                        payee: payee_name,
                        category: cat_name,
                        amount: format_dollars(t.amount),
                        cleared: t.cleared,
                        notes: t.notes.clone(),
                    }
                })
                .collect();

            if json {
                print_json(&view);
            } else {
                println!(
                    "{:<10} {:<36} {:<20} {:<18} {:>14} {:>8}",
                    "DATE", "TRANSACTION ID", "PAYEE", "CATEGORY", "AMOUNT", "CLEARED"
                );
                println!(
                    "{:-<10} {:-<36} {:-<20} {:-<18} {:->14} {:->8}",
                    "", "", "", "", "", ""
                );
                for v in &view {
                    let p = v.payee.as_deref().unwrap_or("");
                    let c = v.category.as_deref().unwrap_or("");
                    let clr = if v.cleared { "✓" } else { " " };
                    println!(
                        "{:<10} {:<36} {:<20} {:<18} {:>14} {:>8}",
                        v.date, v.id, p, c, v.amount, clr
                    );
                }
            }
            Ok(())
        }
        TransactionSubcommand::Add {
            account,
            date,
            amount,
            payee,
            category,
            notes,
            cleared,
        } => {
            let tx_date = if let Some(d) = date {
                NaiveDate::parse_from_str(&d, "%Y-%m-%d")
                    .map_err(|e| format!("Invalid date: {e}"))?
            } else {
                Local::now().date_naive()
            };

            let cents = Cents::from_cents((amount * 100.0).round() as i64);

            let id = svc
                .add_transaction(AddTransaction {
                    id: String::new(),
                    account_id: account,
                    date: tx_date,
                    amount: cents,
                    payee_id: payee,
                    category_id: category,
                    notes,
                    cleared,
                })
                .map_err(|e| e.to_string())?;

            if json {
                print_json(&serde_json::json!({
                    "status": "ok",
                    "id": id,
                    "date": tx_date.to_string(),
                    "amount": format_dollars(cents),
                }));
            } else {
                println!(
                    "Added transaction {id} ({}) on {tx_date}",
                    format_dollars(cents)
                );
            }
            Ok(())
        }
        TransactionSubcommand::Delete { id } => {
            svc.delete_transaction(DeleteTransaction {
                transaction_id: id.clone(),
            })
            .map_err(|e| e.to_string())?;

            if json {
                print_json(&serde_json::json!({ "status": "ok", "deleted": id }));
            } else {
                println!("Deleted transaction: {id}");
            }
            Ok(())
        }
    }
}
