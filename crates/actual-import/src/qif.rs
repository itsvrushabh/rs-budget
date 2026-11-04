//! Quicken Interchange Format (QIF) parser.
//!
//! Parses standard banking and credit card QIF files into `ImportedTransaction` items,
//! supporting dates, amounts, payees, memos, cleared status, and split categories.

use actual_core::Cents;
use chrono::NaiveDate;

use crate::cleaner::PayeeCleaner;
use crate::error::ImportError;
use crate::models::{ImportedSplit, ImportedTransaction};

/// Parser for Quicken Interchange Format (QIF) files.
pub struct QifParser {
    cleaner: PayeeCleaner,
}

impl Default for QifParser {
    fn default() -> Self {
        Self::new()
    }
}

impl QifParser {
    /// Constructs a new `QifParser`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cleaner: PayeeCleaner::new(),
        }
    }

    /// Parses QIF content into a list of `ImportedTransaction` items.
    ///
    /// # Errors
    /// Returns `ImportError` if date or amount fields cannot be parsed.
    pub fn parse(&self, content: &str) -> Result<Vec<ImportedTransaction>, ImportError> {
        let mut transactions = Vec::new();

        let mut date: Option<NaiveDate> = None;
        let mut amount: Option<Cents> = None;
        let mut payee: Option<String> = None;
        let mut memo: Option<String> = None;
        let mut category: Option<String> = None;
        let mut cleared = true;
        let mut check_num: Option<String> = None;

        let mut splits: Vec<ImportedSplit> = Vec::new();
        let mut current_split_cat: Option<String> = None;
        let mut current_split_amt: Option<Cents> = None;
        let mut current_split_memo: Option<String> = None;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('!') {
                continue;
            }

            let code = line.chars().next().unwrap_or(' ');
            let val = line[1..].trim();

            match code {
                'D' => {
                    date = Some(parse_qif_date(val)?);
                }
                'T' | 'U' => {
                    amount = Some(parse_amount(val)?);
                }
                'P' => {
                    payee = Some(val.to_string());
                }
                'M' => {
                    memo = Some(val.to_string());
                }
                'C' => {
                    cleared = matches!(val, "c" | "C" | "x" | "X" | "*" | "R");
                }
                'N' => {
                    check_num = Some(val.to_string());
                }
                'L' => {
                    category = Some(val.to_string());
                }
                'S' => {
                    // Start of split line
                    if let Some(s_amt) = current_split_amt.take() {
                        splits.push(ImportedSplit {
                            amount: s_amt,
                            category: current_split_cat.take(),
                            notes: current_split_memo.take(),
                        });
                    }
                    current_split_cat = Some(val.to_string());
                }
                '$' => {
                    current_split_amt = Some(parse_amount(val)?);
                }
                'E' => {
                    current_split_memo = Some(val.to_string());
                }
                '^' => {
                    // Flush any pending split
                    if let Some(s_amt) = current_split_amt.take() {
                        splits.push(ImportedSplit {
                            amount: s_amt,
                            category: current_split_cat.take(),
                            notes: current_split_memo.take(),
                        });
                    }

                    if let (Some(d), Some(amt)) = (date.take(), amount.take()) {
                        let raw_payee = payee.take();
                        let cleaned_payee =
                            raw_payee.as_deref().and_then(|p| self.cleaner.clean(p));

                        let mut tx = ImportedTransaction::new(d, amt);
                        tx.imported_payee = raw_payee;
                        tx.payee = cleaned_payee;
                        tx.notes = memo.take();
                        tx.category = category.take();
                        tx.cleared = cleared;
                        tx.imported_id = check_num.take();
                        tx.subtransactions = std::mem::take(&mut splits);

                        transactions.push(tx);
                    }

                    // Reset state
                    date = None;
                    amount = None;
                    payee = None;
                    memo = None;
                    category = None;
                    cleared = true;
                    check_num = None;
                    splits.clear();
                    current_split_cat = None;
                    current_split_amt = None;
                    current_split_memo = None;
                }
                _ => {}
            }
        }

        Ok(transactions)
    }
}

/// Parses date formats commonly found in QIF files.
/// Examples: "2024-05-15", "05/15/2024", "5/15'24", "15/05/2024", "05/15/24".
pub fn parse_qif_date(s: &str) -> Result<NaiveDate, ImportError> {
    let clean = s.replace('\'', "/");
    let formats = [
        "%Y-%m-%d", "%m/%d/%Y", "%m/%d/%y", "%d/%m/%Y", "%d/%m/%y", "%Y/%m/%d",
    ];

    for fmt in &formats {
        if let Ok(d) = NaiveDate::parse_from_str(&clean, fmt) {
            return Ok(d);
        }
    }

    Err(ImportError::InvalidDate(
        s.to_string(),
        "YYYY-MM-DD, MM/DD/YYYY, MM/DD'YY".to_string(),
    ))
}

/// Parses currency amount strings like "-45.50", "1,250.99", "$50", "(25.00)" into `Cents`.
pub fn parse_amount(s: &str) -> Result<Cents, ImportError> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Ok(Cents::ZERO);
    }

    let mut negative = false;
    let mut cleaned = String::new();

    for ch in trimmed.chars() {
        match ch {
            '-' | '(' => negative = true,
            ')' | '$' | ' ' | ',' | '+' => {}
            '.' | '0'..='9' => cleaned.push(ch),
            _ => {}
        }
    }

    if cleaned.is_empty() {
        return Err(ImportError::InvalidAmount(
            s.to_string(),
            "no numeric characters found".to_string(),
        ));
    }

    let val: f64 = cleaned.parse().map_err(|e| {
        ImportError::InvalidAmount(s.to_string(), format!("failed to parse float: {e}"))
    })?;

    let cents_val = (val * 100.0).round() as i64;
    let final_cents = if negative { -cents_val } else { cents_val };

    Ok(Cents::from_cents(final_cents))
}
