//! Payee normalization and cleaning.
//!
//! Cleans raw bank descriptions by stripping common bank artifacts, POS prefixes,
//! store numbers, terminal references, and trailing garbage.

use regex::Regex;

/// Payee cleaning engine that transforms raw bank descriptions into clean merchant names.
pub struct PayeeCleaner {
    prefix_patterns: Vec<Regex>,
    suffix_patterns: Vec<Regex>,
    cleanup_patterns: Vec<Regex>,
    whitespace_re: Regex,
}

impl Default for PayeeCleaner {
    fn default() -> Self {
        Self::new()
    }
}

impl PayeeCleaner {
    /// Constructs a `PayeeCleaner` with default cleaning rules matching upstream conventions.
    #[must_use]
    pub fn new() -> Self {
        let prefix_patterns = vec![
            Regex::new(r"(?i)^(POS\s+DEBIT|POS\s+PURCHASE|DEBIT\s+CARD\s+PURCHASE|PURCHASE\s+AUTHORIZED\s+ON)\s+").unwrap(),
            Regex::new(r"(?i)^(ACH\s+DEBIT|ACH\s+WITHDRAWAL|ACH\s+PAYMENT|DIRECT\s+DEP|AUTOMATIC\s+PAYMENT)\s+").unwrap(),
            Regex::new(r"(?i)^(CHECK\s+#?\s*\d+|CHECK\s+PAID)\s*").unwrap(),
            Regex::new(r"(?i)^(PAYPAL\s*\*|SQ\s*\*|TST\*\s*|AMZN\s+Mktp\s+US\*|SP\s*\*)\s*").unwrap(),
            Regex::new(r"(?i)^(BILL\s+PAYMENT|ELECTRONIC\s+PAYMENT|ONLINE\s+BANKING\s+TRANSFER)\s+").unwrap(),
        ];

        let suffix_patterns = vec![
            // Direct deposit suffix
            Regex::new(r"(?i)\s+(DIRECT\s+DEP(OSIT)?)$").unwrap(),
            // Store numbers like #1234, STORE 1234, # 0123
            Regex::new(r"(?i)\s+(#\s*\d+|STORE\s+#?\d+|LOC\s+#?\d+|UNIT\s+#?\d+)$").unwrap(),
            // Numeric store / terminal / phone / zip code sequences (4 to 12 digits)
            Regex::new(r"\s+\d{4,12}$").unwrap(),
            Regex::new(r"\s+\d{3}-\d{3}-\d{4}$").unwrap(),
            // Trailing dates MM/DD or MM/DD/YY
            Regex::new(r"\s+\d{2}/\d{2}(/\d{2,4})?$").unwrap(),
            // Trailing US state abbreviations (excluding CO to avoid stripping Company / Co)
            Regex::new(r"\s+(?:AL|AK|AZ|AR|CA|CT|DE|FL|GA|HI|ID|IL|IN|IA|KS|KY|LA|ME|MD|MA|MI|MN|MS|MO|MT|NE|NV|NH|NJ|NM|NY|NC|ND|OH|OK|OR|PA|RI|SC|SD|TN|TX|UT|VT|VA|WA|WV|WI|WY)\s*$").unwrap(),
        ];

        let cleanup_patterns = vec![
            // Strip leading www.
            Regex::new(r"(?i)^www\.").unwrap(),
            // Card numbers like ...1234 or XXXX-1234
            Regex::new(r"(?i)(X{4,}|[*]{4,}|\.{3,})\s*\d{4}").unwrap(),
        ];

        let whitespace_re = Regex::new(r"\s+").unwrap();

        Self {
            prefix_patterns,
            suffix_patterns,
            cleanup_patterns,
            whitespace_re,
        }
    }

    /// Cleans a raw bank payee string.
    ///
    /// Returns `None` if the resulting string is empty.
    #[must_use]
    pub fn clean(&self, raw: &str) -> Option<String> {
        let mut s = raw.trim().to_string();
        if s.is_empty() {
            return None;
        }

        // Apply prefix stripping
        for re in &self.prefix_patterns {
            s = re.replace(&s, "").to_string();
        }

        // Apply in-line cleanup
        for re in &self.cleanup_patterns {
            s = re.replace_all(&s, "").to_string();
        }

        // Apply suffix stripping repeatedly until string stops changing
        let mut changed = true;
        while changed {
            changed = false;
            for re in &self.suffix_patterns {
                let replaced = re.replace(&s, "").to_string();
                if replaced != s {
                    s = replaced;
                    changed = true;
                }
            }
        }

        // Normalize multiple spaces and punctuation
        let s = self.whitespace_re.replace_all(&s, " ").trim().to_string();

        if s.is_empty() {
            None
        } else {
            Some(s)
        }
    }
}
