//! Configurable CSV statement parser.
//!
//! Supports customizable delimiters, date formats, column mappings,
//! and either signed amount columns or split dual inflow/outflow columns.

use actual_core::Cents;
use chrono::NaiveDate;
use csv::ReaderBuilder;
use serde::{Deserialize, Serialize};

use crate::cleaner::PayeeCleaner;
use crate::error::ImportError;
use crate::models::ImportedTransaction;
use crate::qif::{parse_amount, parse_qif_date};

/// Configuration options for mapping and parsing CSV statement files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsvConfig {
    /// Field delimiter character (default `,`).
    pub delimiter: u8,
    /// Whether the CSV has a header row.
    pub has_headers: bool,
    /// Header name or 0-indexed column number for the transaction date.
    pub date_column: String,
    /// Explicit date format string (e.g. `"%m/%d/%Y"`). If `None`, automatic format detection is used.
    pub date_format: Option<String>,
    /// Header name or column index for the Payee name.
    pub payee_column: Option<String>,
    /// Header name or column index for description/notes/memo.
    pub notes_column: Option<String>,
    /// Header name or column index for a single signed amount column (e.g. `"-25.00"` or `"$100"`).
    pub amount_column: Option<String>,
    /// Header name or column index for credit / inflow amounts.
    pub inflow_column: Option<String>,
    /// Header name or column index for debit / outflow amounts.
    pub outflow_column: Option<String>,
    /// Header name or column index for unique transaction ID.
    pub id_column: Option<String>,
    /// Header name or column index for category.
    pub category_column: Option<String>,
    /// Reverse signed amount polarity (true if bank represents expenses as positive).
    pub flip_amount: bool,
}

impl Default for CsvConfig {
    fn default() -> Self {
        Self {
            delimiter: b',',
            has_headers: true,
            date_column: "Date".to_string(),
            date_format: None,
            payee_column: Some("Payee".to_string()),
            notes_column: Some("Memo".to_string()),
            amount_column: Some("Amount".to_string()),
            inflow_column: None,
            outflow_column: None,
            id_column: None,
            category_column: None,
            flip_amount: false,
        }
    }
}

/// CSV Statement Parser.
pub struct CsvParser {
    cleaner: PayeeCleaner,
}

impl Default for CsvParser {
    fn default() -> Self {
        Self::new()
    }
}

impl CsvParser {
    /// Constructs a new `CsvParser`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cleaner: PayeeCleaner::new(),
        }
    }

    /// Parses CSV content according to the provided `config`.
    ///
    /// # Errors
    /// Returns `ImportError` if CSV parsing, date formatting, or amount calculation fails.
    pub fn parse(
        &self,
        content: &str,
        config: &CsvConfig,
    ) -> Result<Vec<ImportedTransaction>, ImportError> {
        let mut reader = ReaderBuilder::new()
            .delimiter(config.delimiter)
            .has_headers(config.has_headers)
            .flexible(true)
            .from_reader(content.as_bytes());

        let headers = if config.has_headers {
            Some(reader.headers()?.clone())
        } else {
            None
        };

        let resolve_col = |col_ref: &Option<String>| -> Option<usize> {
            col_ref.as_ref().and_then(|col| {
                if let Ok(idx) = col.parse::<usize>() {
                    Some(idx)
                } else if let Some(ref hdrs) = headers {
                    hdrs.iter()
                        .position(|h| h.trim().eq_ignore_ascii_case(col.trim()))
                } else {
                    None
                }
            })
        };

        let date_col_idx = if let Ok(idx) = config.date_column.parse::<usize>() {
            Some(idx)
        } else if let Some(ref hdrs) = headers {
            hdrs.iter()
                .position(|h| h.trim().eq_ignore_ascii_case(config.date_column.trim()))
        } else {
            None
        }
        .ok_or_else(|| ImportError::MissingField(config.date_column.clone()))?;

        let payee_idx = resolve_col(&config.payee_column);
        let notes_idx = resolve_col(&config.notes_column);
        let amount_idx = resolve_col(&config.amount_column);
        let inflow_idx = resolve_col(&config.inflow_column);
        let outflow_idx = resolve_col(&config.outflow_column);
        let id_idx = resolve_col(&config.id_column);
        let category_idx = resolve_col(&config.category_column);

        let mut transactions = Vec::new();

        for result in reader.records() {
            let record = result?;
            if record.is_empty() {
                continue;
            }

            // 1. Parse Date
            let raw_date = record.get(date_col_idx).unwrap_or("").trim();
            if raw_date.is_empty() {
                continue;
            }

            let date = if let Some(ref fmt) = config.date_format {
                NaiveDate::parse_from_str(raw_date, fmt)
                    .map_err(|_| ImportError::InvalidDate(raw_date.to_string(), fmt.clone()))?
            } else {
                parse_qif_date(raw_date)?
            };

            // 2. Parse Amount (either single signed column or dual inflow/outflow)
            let mut amount = Cents::ZERO;

            if let Some(idx) = amount_idx {
                if let Some(val) = record.get(idx) {
                    amount = parse_amount(val)?;
                    if config.flip_amount {
                        amount = -amount;
                    }
                }
            } else if inflow_idx.is_some() || outflow_idx.is_some() {
                let mut inflow = Cents::ZERO;
                let mut outflow = Cents::ZERO;

                if let Some(idx) = inflow_idx {
                    if let Some(val) = record.get(idx) {
                        let parsed = parse_amount(val)?;
                        inflow = Cents::from_cents(parsed.as_cents().abs());
                    }
                }
                if let Some(idx) = outflow_idx {
                    if let Some(val) = record.get(idx) {
                        let parsed = parse_amount(val)?;
                        outflow = Cents::from_cents(parsed.as_cents().abs());
                    }
                }

                amount = inflow - outflow;
            }

            // 3. Metadata fields
            let raw_payee = payee_idx
                .and_then(|idx| record.get(idx))
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from);
            let cleaned_payee = raw_payee.as_deref().and_then(|p| self.cleaner.clean(p));

            let notes = notes_idx
                .and_then(|idx| record.get(idx))
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from);
            let id = id_idx
                .and_then(|idx| record.get(idx))
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from);
            let cat = category_idx
                .and_then(|idx| record.get(idx))
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(String::from);

            let mut tx = ImportedTransaction::new(date, amount);
            tx.imported_payee = raw_payee;
            tx.payee = cleaned_payee;
            tx.notes = notes;
            tx.imported_id = id;
            tx.category = cat;
            tx.cleared = true;

            transactions.push(tx);
        }

        Ok(transactions)
    }
}
