//! Statement import command handler.

use actual_api::commands::AddTransaction;
use actual_api::BudgetService;
use actual_core::Transaction;
use actual_import::{
    CsvConfig, CsvParser, Deduplicator, ImportedTransaction, OfxParser, QifParser,
};
use std::fs;

use crate::cli::{ImportArgs, ImportFormat};
use crate::output::print_json;

pub fn handle(svc: &mut BudgetService, args: ImportArgs, json: bool) -> Result<(), String> {
    if svc.account(&args.account).is_none() {
        return Err(format!("Account not found: {}", args.account));
    }

    let content = fs::read_to_string(&args.file)
        .map_err(|e| format!("Failed to read {}: {e}", args.file.display()))?;

    // Determine format
    let format = match args.format {
        ImportFormat::Auto => {
            let ext = args
                .file
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            match ext.as_str() {
                "qif" => ImportFormat::Qif,
                "ofx" | "qfx" => ImportFormat::Ofx,
                "csv" => ImportFormat::Csv,
                _ => {
                    return Err(format!(
                        "Could not auto-detect format for extension '.{ext}'. Use --format"
                    ))
                }
            }
        }
        f => f,
    };

    let parsed: Vec<ImportedTransaction> = match format {
        ImportFormat::Qif => {
            let parser = QifParser::new();
            parser
                .parse(&content)
                .map_err(|e| format!("QIF parse error: {e}"))?
        }
        ImportFormat::Ofx => {
            let parser = OfxParser::new();
            parser
                .parse(&content)
                .map_err(|e| format!("OFX parse error: {e}"))?
        }
        ImportFormat::Csv => {
            let parser = CsvParser::new();
            let config = CsvConfig::default();
            parser
                .parse(&content, &config)
                .map_err(|e| format!("CSV parse error: {e}"))?
        }
        ImportFormat::Auto => unreachable!(),
    };

    let existing: Vec<Transaction> = svc
        .transactions_for_account(&args.account)
        .into_iter()
        .cloned()
        .collect();
    let dedup = Deduplicator::new(args.proximity_days);
    let result = dedup.deduplicate(parsed, &existing);

    let total_parsed = result.unique.len() + result.duplicates.len();
    let mut imported_count = 0;

    for item in result.unique {
        let cmd = AddTransaction {
            id: String::new(),
            account_id: args.account.clone(),
            date: item.date,
            amount: item.amount,
            payee_id: item.payee,
            category_id: item.category,
            notes: item.notes,
            cleared: item.cleared,
        };
        svc.add_transaction(cmd).map_err(|e| e.to_string())?;
        imported_count += 1;
    }

    let duplicates_skipped = result.duplicates.len();

    if json {
        print_json(&serde_json::json!({
            "status": "ok",
            "account_id": args.account,
            "total_parsed": total_parsed,
            "imported": imported_count,
            "duplicates_skipped": duplicates_skipped,
        }));
    } else {
        println!("Import completed for account '{}':", args.account);
        println!("  Total parsed:       {total_parsed}");
        println!("  Imported unique:    {imported_count}");
        println!("  Duplicates skipped: {duplicates_skipped}");
    }

    Ok(())
}
