//! Report filter for arbitrary queries across date ranges, accounts, categories, and payees.

use chrono::NaiveDate;

use crate::Transaction;

/// Configurable filter for restricting report scope.
///
/// All filter fields are optional — when `None`, no restriction is applied for
/// that dimension. When multiple filters are present, they are combined with
/// logical AND.
#[derive(Debug, Clone, Default)]
pub struct ReportFilter {
    /// Inclusive start date.
    pub start_date: Option<NaiveDate>,
    /// Inclusive end date.
    pub end_date: Option<NaiveDate>,
    /// Restrict to these account IDs (empty = all accounts).
    pub account_ids: Vec<String>,
    /// Restrict to these category IDs (empty = all categories).
    pub category_ids: Vec<String>,
    /// Restrict to these payee IDs (empty = all payees).
    pub payee_ids: Vec<String>,
}

impl ReportFilter {
    /// Create an empty (pass-everything) filter.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the inclusive date range.
    #[must_use]
    pub fn with_date_range(mut self, start: NaiveDate, end: NaiveDate) -> Self {
        self.start_date = Some(start);
        self.end_date = Some(end);
        self
    }

    /// Restrict to specific accounts.
    #[must_use]
    pub fn with_accounts(mut self, ids: Vec<String>) -> Self {
        self.account_ids = ids;
        self
    }

    /// Restrict to specific categories.
    #[must_use]
    pub fn with_categories(mut self, ids: Vec<String>) -> Self {
        self.category_ids = ids;
        self
    }

    /// Restrict to specific payees.
    #[must_use]
    pub fn with_payees(mut self, ids: Vec<String>) -> Self {
        self.payee_ids = ids;
        self
    }

    /// Test whether a transaction passes this filter.
    #[must_use]
    pub fn matches(&self, tx: &Transaction) -> bool {
        if tx.tombstone {
            return false;
        }
        if let Some(start) = self.start_date {
            if tx.date < start {
                return false;
            }
        }
        if let Some(end) = self.end_date {
            if tx.date > end {
                return false;
            }
        }
        if !self.account_ids.is_empty() && !self.account_ids.contains(&tx.account_id) {
            return false;
        }
        if !self.category_ids.is_empty() {
            match &tx.category_id {
                Some(cid) => {
                    if !self.category_ids.contains(cid) {
                        return false;
                    }
                }
                None => return false,
            }
        }
        if !self.payee_ids.is_empty() {
            match &tx.payee_id {
                Some(pid) => {
                    if !self.payee_ids.contains(pid) {
                        return false;
                    }
                }
                None => return false,
            }
        }
        true
    }
}
