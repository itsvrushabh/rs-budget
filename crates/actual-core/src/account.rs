//! Account and Account Group domain entities.

use serde::{Deserialize, Serialize};

/// Account Group entity categorizing accounts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountGroup {
    pub id: String,
    pub name: String,
}

impl AccountGroup {
    /// Creates a new `AccountGroup`.
    #[must_use]
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
        }
    }
}

/// Financial account entity (checking, savings, credit card, investment, etc.).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Account {
    /// Unique identifier (UUID).
    pub id: String,
    /// Display name of the account.
    pub name: String,
    /// Whether this account is an off-budget/tracking account (true) or on-budget (false).
    pub offbudget: bool,
    /// Whether this account is closed.
    pub closed: bool,
    /// Relative sort order for display.
    pub sort_order: Option<f64>,
    /// Soft deletion marker.
    pub tombstone: bool,
    /// Optional foreign key to an `AccountGroup`.
    pub account_group_id: Option<String>,
    /// Last reconciled date/timestamp.
    pub last_reconciled: Option<String>,

    // Bank sync & financial institution fields
    pub account_id: Option<String>,
    pub bank: Option<String>,
    pub bank_name: Option<String>,
    pub mask: Option<String>,
    pub official_name: Option<String>,
}

impl Account {
    /// Creates a new active on-budget `Account`.
    #[must_use]
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            offbudget: false,
            closed: false,
            sort_order: None,
            tombstone: false,
            account_group_id: None,
            last_reconciled: None,
            account_id: None,
            bank: None,
            bank_name: None,
            mask: None,
            official_name: None,
        }
    }

    /// Creates a new off-budget/tracking `Account`.
    #[must_use]
    pub fn new_offbudget(id: impl Into<String>, name: impl Into<String>) -> Self {
        let mut acct = Self::new(id, name);
        acct.offbudget = true;
        acct
    }

    /// Returns whether this account is currently active on-budget (participates in envelope budgeting).
    #[must_use]
    pub fn is_on_budget(&self) -> bool {
        !self.offbudget && !self.closed && !self.tombstone
    }

    /// Returns whether this account is an off-budget / tracking account.
    #[must_use]
    pub fn is_off_budget(&self) -> bool {
        self.offbudget
    }

    /// Returns whether this account is open and not tombstoned.
    #[must_use]
    pub fn is_active(&self) -> bool {
        !self.closed && !self.tombstone
    }
}
