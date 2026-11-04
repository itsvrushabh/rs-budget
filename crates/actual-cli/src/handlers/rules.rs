//! Rules inspection and execution command handlers.

use actual_api::commands::ApplyRules;
use actual_api::BudgetService;

use crate::cli::RulesSubcommand;
use crate::output::print_json;

pub fn handle(svc: &mut BudgetService, cmd: RulesSubcommand, json: bool) -> Result<(), String> {
    match cmd {
        RulesSubcommand::List => {
            if json {
                print_json(&serde_json::json!({
                    "status": "ok",
                    "rules_count": 0,
                    "rules": []
                }));
            } else {
                println!("No transaction automation rules currently configured.");
            }
            Ok(())
        }
        RulesSubcommand::Apply { account } => {
            let tx_ids: Vec<String> = if let Some(ref acct) = account {
                svc.transactions_for_account(acct)
                    .into_iter()
                    .map(|t| t.id.clone())
                    .collect()
            } else {
                svc.transactions()
                    .into_iter()
                    .map(|t| t.id.clone())
                    .collect()
            };

            let modified = svc
                .apply_rules(ApplyRules {
                    transaction_ids: tx_ids.clone(),
                })
                .map_err(|e| e.to_string())?;

            if json {
                print_json(&serde_json::json!({
                    "status": "ok",
                    "evaluated": tx_ids.len(),
                    "modified": modified.len(),
                    "modified_ids": modified,
                }));
            } else {
                println!(
                    "Rules applied across {} transactions ({} modified).",
                    tx_ids.len(),
                    modified.len()
                );
            }
            Ok(())
        }
    }
}
