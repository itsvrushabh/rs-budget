//! Payee domain entity.

use serde::{Deserialize, Serialize};

/// Payee entity representing a merchant, person, or transfer account endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Payee {
    /// Unique identifier (UUID).
    pub id: String,
    /// Display name of the payee.
    pub name: String,
    /// If this payee represents a transfer to another account, the target `Account` id.
    pub transfer_acct: Option<String>,
    /// Whether this payee is marked as a favorite.
    pub favorite: bool,
    /// Whether auto-categorization learning is enabled for this payee.
    pub learn_categories: bool,
    /// Soft deletion marker.
    pub tombstone: bool,
}

impl Payee {
    /// Creates a standard merchant/person `Payee`.
    #[must_use]
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            transfer_acct: None,
            favorite: false,
            learn_categories: true,
            tombstone: false,
        }
    }

    /// Creates a transfer `Payee` linked to a target account.
    #[must_use]
    pub fn new_transfer(
        id: impl Into<String>,
        name: impl Into<String>,
        target_account_id: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            transfer_acct: Some(target_account_id.into()),
            favorite: false,
            learn_categories: false,
            tombstone: false,
        }
    }

    /// Returns whether this payee is a transfer destination to another account.
    #[must_use]
    pub fn is_transfer(&self) -> bool {
        self.transfer_acct.is_some()
    }

    /// Returns the target account ID if this payee is a transfer payee.
    #[must_use]
    pub fn transfer_account_id(&self) -> Option<&str> {
        self.transfer_acct.as_deref()
    }

    /// Returns whether this payee is active (not tombstoned).
    #[must_use]
    pub fn is_active(&self) -> bool {
        !self.tombstone
    }
}
