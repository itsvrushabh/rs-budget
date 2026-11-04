//! Account management command handlers.

use actual_api::commands::{CloseAccount, CreateAccount, ReopenAccount};
use actual_api::BudgetService;
use actual_core::Cents;
use serde::Serialize;

use crate::cli::AccountSubcommand;
use crate::output::{format_dollars, print_json};

#[derive(Serialize)]
struct AccountView {
    id: String,
    name: String,
    offbudget: bool,
    closed: bool,
    balance: String,
    raw_balance_cents: i64,
}

pub fn handle(svc: &mut BudgetService, cmd: AccountSubcommand, json: bool) -> Result<(), String> {
    match cmd {
        AccountSubcommand::List { all } => {
            let mut list = Vec::new();
            for acc in svc.accounts() {
                if acc.closed && !all {
                    continue;
                }
                let txs = svc.transactions_for_account(&acc.id);
                let balance_cents: i64 = txs.iter().map(|t| t.amount.as_cents()).sum();
                list.push(AccountView {
                    id: acc.id.clone(),
                    name: acc.name.clone(),
                    offbudget: acc.offbudget,
                    closed: acc.closed,
                    balance: format_dollars(Cents::from_cents(balance_cents)),
                    raw_balance_cents: balance_cents,
                });
            }

            if json {
                print_json(&list);
            } else {
                println!(
                    "{:<36} {:<24} {:>14} {:>10} {:>8}",
                    "ACCOUNT ID", "NAME", "BALANCE", "TYPE", "STATUS"
                );
                println!("{:-<36} {:-<24} {:->14} {:->10} {:->8}", "", "", "", "", "");
                for a in &list {
                    let acct_type = if a.offbudget { "tracking" } else { "on-budget" };
                    let status = if a.closed { "closed" } else { "active" };
                    println!(
                        "{:<36} {:<24} {:>14} {:>10} {:>8}",
                        a.id, a.name, a.balance, acct_type, status
                    );
                }
            }
            Ok(())
        }
        AccountSubcommand::Create { name, offbudget } => {
            let id = svc
                .create_account(CreateAccount {
                    id: String::new(),
                    name: name.clone(),
                    offbudget,
                })
                .map_err(|e| e.to_string())?;

            if json {
                print_json(&serde_json::json!({
                    "status": "ok",
                    "id": id,
                    "name": name,
                    "offbudget": offbudget,
                }));
            } else {
                println!("Created account '{name}' with ID: {id}");
            }
            Ok(())
        }
        AccountSubcommand::Close { id } => {
            svc.close_account(CloseAccount {
                account_id: id.clone(),
            })
            .map_err(|e| e.to_string())?;

            if json {
                print_json(&serde_json::json!({ "status": "ok", "closed": id }));
            } else {
                println!("Closed account: {id}");
            }
            Ok(())
        }
        AccountSubcommand::Reopen { id } => {
            svc.reopen_account(ReopenAccount {
                account_id: id.clone(),
            })
            .map_err(|e| e.to_string())?;

            if json {
                print_json(&serde_json::json!({ "status": "ok", "reopened": id }));
            } else {
                println!("Reopened account: {id}");
            }
            Ok(())
        }
    }
}
