//! Data export command handler.

use actual_api::BudgetService;
use serde::Serialize;

use crate::cli::{ExportArgs, ExportFormat};
use crate::output::print_json;

#[derive(Serialize)]
struct ExportRecord {
    id: String,
    date: String,
    account_id: String,
    amount_cents: i64,
    payee_id: Option<String>,
    category_id: Option<String>,
    notes: Option<String>,
    cleared: bool,
}

pub fn handle(svc: &BudgetService, args: ExportArgs, _json_mode: bool) -> Result<(), String> {
    let txs = if let Some(ref acct) = args.account {
        svc.transactions_for_account(acct)
    } else {
        svc.transactions()
    };

    let records: Vec<ExportRecord> = txs
        .into_iter()
        .map(|t| ExportRecord {
            id: t.id.clone(),
            date: t.date.to_string(),
            account_id: t.account_id.clone(),
            amount_cents: t.amount.as_cents(),
            payee_id: t.payee_id.clone(),
            category_id: t.category_id.clone(),
            notes: t.notes.clone(),
            cleared: t.cleared,
        })
        .collect();

    match args.format {
        ExportFormat::Json => {
            print_json(&records);
        }
        ExportFormat::Csv => {
            println!("id,date,account_id,amount_cents,payee_id,category_id,notes,cleared");
            for r in &records {
                let p = r.payee_id.as_deref().unwrap_or("");
                let c = r.category_id.as_deref().unwrap_or("");
                let n = r.notes.as_deref().unwrap_or("");
                println!(
                    "{},{},{},{},{},{},{},{}",
                    r.id, r.date, r.account_id, r.amount_cents, p, c, n, r.cleared
                );
            }
        }
    }

    Ok(())
}
