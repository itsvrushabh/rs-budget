//! Open Financial Exchange (OFX / QFX) parser.
//!
//! Supports OFX 1.x (SGML-style open tags) and OFX 2.x (XML-style closed tags),
//! extracting transactions (`STMTTRN`), dates (`DTPOSTED`), amounts (`TRNAMT`),
//! unique identifiers (`FITID`), payees (`NAME`), and memos (`MEMO`).

use regex::Regex;
use time::Date as NaiveDate;

use crate::cleaner::PayeeCleaner;
use crate::error::ImportError;
use crate::models::ImportedTransaction;
use crate::qif::parse_amount;

fn trnamt_re() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)<TRNAMT>([^<\r\n]+)").unwrap())
}

fn dtposted_re() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)<DTPOSTED>([^<\r\n]+)").unwrap())
}

fn fitid_re() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)<FITID>([^<\r\n]+)").unwrap())
}

fn name_re() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)<NAME>([^<\r\n]+)").unwrap())
}

fn memo_re() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?i)<MEMO>([^<\r\n]+)").unwrap())
}

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

        let trnamt = trnamt_re();
        let dtposted = dtposted_re();
        let fitid = fitid_re();
        let name = name_re();
        let memo = memo_re();

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
            let amount = if let Some(amt_cap) = trnamt.captures(block) {
                parse_amount(&amt_cap[1])?
            } else {
                continue;
            };

            // 2. Date
            let date = if let Some(dt_cap) = dtposted.captures(block) {
                parse_ofx_date(&dt_cap[1])?
            } else {
                continue;
            };

            // 3. FITID
            let fitid_val = fitid.captures(block).map(|c| c[1].trim().to_string());

            // 4. Payee name
            let raw_payee = name.captures(block).map(|c| c[1].trim().to_string());
            let cleaned_payee = raw_payee.as_deref().and_then(|p| self.cleaner.clean(p));

            // 5. Memo
            let memo_val = memo.captures(block).map(|c| c[1].trim().to_string());

            let mut tx = ImportedTransaction::new(date, amount);
            tx.imported_id = fitid_val;
            tx.imported_payee = raw_payee;
            tx.payee = cleaned_payee;
            tx.notes = memo_val;
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
        if let Ok(d) = NaiveDate::parse(
            &clean[..10],
            time::macros::format_description!("[year]-[month]-[day]"),
        ) {
            return Ok(d);
        }
    }

    // Standard OFX format: YYYYMMDD...
    if clean.len() >= 8 {
        let ymd = &clean[..8];
        if let Ok(d) =
            NaiveDate::parse(ymd, time::macros::format_description!("[year][month][day]"))
        {
            return Ok(d);
        }
    }

    Err(ImportError::InvalidDate(
        s.to_string(),
        "YYYYMMDD or YYYY-MM-DD".to_string(),
    ))
}
