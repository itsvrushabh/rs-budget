//! Open Financial Exchange (OFX / QFX) parser.
//!
//! Supports OFX 1.x (SGML-style open tags) and OFX 2.x (XML-style closed tags),
//! extracting transactions (`STMTTRN`), dates (`DTPOSTED`), amounts (`TRNAMT`),
//! unique identifiers (`FITID`), payees (`NAME`), and memos (`MEMO`).

use chrono::NaiveDate;
use regex::Regex;

use crate::cleaner::PayeeCleaner;
use crate::error::ImportError;
use crate::models::ImportedTransaction;
use crate::qif::parse_amount;

/// OFX / QFX Statement Parser.
pub struct OfxParser {
    cleaner: PayeeCleaner,
}

impl Default for OfxParser {
    fn default() -> Self {
        Self::new()
    }
}

impl OfxParser {
    /// Constructs a new `OfxParser`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cleaner: PayeeCleaner::new(),
        }
    }

    /// Parses OFX or QFX content (both SGML and XML flavors) into `ImportedTransaction` items.
    ///
    /// # Errors
    /// Returns `ImportError` if required fields are unparseable.
    pub fn parse(&self, content: &str) -> Result<Vec<ImportedTransaction>, ImportError> {
        let mut transactions = Vec::new();

        let trnamt_re = Regex::new(r"(?i)<TRNAMT>([^<\r\n]+)").unwrap();
        let dtposted_re = Regex::new(r"(?i)<DTPOSTED>([^<\r\n]+)").unwrap();
        let fitid_re = Regex::new(r"(?i)<FITID>([^<\r\n]+)").unwrap();
        let name_re = Regex::new(r"(?i)<NAME>([^<\r\n]+)").unwrap();
        let memo_re = Regex::new(r"(?i)<MEMO>([^<\r\n]+)").unwrap();

        let lower = content.to_ascii_lowercase();
        let tag = "<stmttrn>";
        let mut start_idx = 0;

        while let Some(pos) = lower[start_idx..].find(tag) {
            let block_start = start_idx + pos + tag.len();
            let search_slice = &lower[block_start..];
            let end_offset = [
                search_slice.find("</stmttrn>"),
                search_slice.find("<stmttrn>"),
                search_slice.find("</banktranlist>"),
            ]
            .into_iter()
            .flatten()
            .min()
            .unwrap_or(search_slice.len());

            let block = &content[block_start..block_start + end_offset];
            start_idx = block_start + end_offset;

            // 1. Amount
            let amount = if let Some(amt_cap) = trnamt_re.captures(block) {
                parse_amount(&amt_cap[1])?
            } else {
                continue;
            };

            // 2. Date
            let date = if let Some(dt_cap) = dtposted_re.captures(block) {
                parse_ofx_date(&dt_cap[1])?
            } else {
                continue;
            };

            // 3. FITID
            let fitid = fitid_re.captures(block).map(|c| c[1].trim().to_string());

            // 4. Payee name
            let raw_payee = name_re.captures(block).map(|c| c[1].trim().to_string());
            let cleaned_payee = raw_payee.as_deref().and_then(|p| self.cleaner.clean(p));

            // 5. Memo
            let memo = memo_re.captures(block).map(|c| c[1].trim().to_string());

            let mut tx = ImportedTransaction::new(date, amount);
            tx.imported_id = fitid;
            tx.imported_payee = raw_payee;
            tx.payee = cleaned_payee;
            tx.notes = memo;
            tx.cleared = true;

            transactions.push(tx);
        }

        Ok(transactions)
    }
}

/// Parses OFX date formats like "20240115", "20240115120000", "20240115120000[-5:EST]", "2024-01-15".
pub fn parse_ofx_date(s: &str) -> Result<NaiveDate, ImportError> {
    let clean = s.trim();

    // Check ISO format YYYY-MM-DD
    if clean.len() >= 10 && &clean[4..5] == "-" && &clean[7..8] == "-" {
        if let Ok(d) = NaiveDate::parse_from_str(&clean[..10], "%Y-%m-%d") {
            return Ok(d);
        }
    }

    // Standard OFX format: YYYYMMDD...
    if clean.len() >= 8 {
        let ymd = &clean[..8];
        if let Ok(d) = NaiveDate::parse_from_str(ymd, "%Y%m%d") {
            return Ok(d);
        }
    }

    Err(ImportError::InvalidDate(
        s.to_string(),
        "YYYYMMDD or YYYY-MM-DD".to_string(),
    ))
}
