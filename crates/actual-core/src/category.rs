//! Category and Category Group domain entities.

use serde::{Deserialize, Serialize};

/// Category Group entity grouping envelope categories (e.g. "Housing", "Food", "Income").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CategoryGroup {
    /// Unique identifier (UUID).
    pub id: String,
    /// Display name of the category group.
    pub name: String,
    /// Whether this group holds income categories rather than expense budget envelopes.
    pub is_income: bool,
    /// Display sort order.
    pub sort_order: Option<f64>,
    /// Whether this group is hidden from main views.
    pub hidden: bool,
    /// Soft deletion marker.
    pub tombstone: bool,
}

impl CategoryGroup {
    /// Creates a new expense `CategoryGroup`.
    #[must_use]
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            is_income: false,
            sort_order: None,
            hidden: false,
            tombstone: false,
        }
    }

    /// Creates a new income `CategoryGroup`.
    #[must_use]
    pub fn new_income(id: impl Into<String>, name: impl Into<String>) -> Self {
        let mut group = Self::new(id, name);
        group.is_income = true;
        group
    }

    /// Returns whether this group is active (not hidden and not tombstoned).
    #[must_use]
    pub fn is_active(&self) -> bool {
        !self.hidden && !self.tombstone
    }
}

/// Category entity representing an envelope budget category (e.g. "Rent", "Groceries").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Category {
    /// Unique identifier (UUID).
    pub id: String,
    /// Display name of the category.
    pub name: String,
    /// Foreign key referencing the parent `CategoryGroup`.
    pub group_id: String,
    /// Whether this is an income category.
    pub is_income: bool,
    /// Display sort order within the group.
    pub sort_order: Option<f64>,
    /// Whether this category is hidden.
    pub hidden: bool,
    /// Soft deletion marker.
    pub tombstone: bool,
    /// Optional goal or target definition expression.
    pub goal_def: Option<String>,
    /// Optional cleanup template definition.
    pub cleanup_def: Option<String>,
}

impl Category {
    /// Creates a new expense `Category`.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        group_id: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            group_id: group_id.into(),
            is_income: false,
            sort_order: None,
            hidden: false,
            tombstone: false,
            goal_def: None,
            cleanup_def: None,
        }
    }

    /// Creates a new income `Category`.
    #[must_use]
    pub fn new_income(
        id: impl Into<String>,
        name: impl Into<String>,
        group_id: impl Into<String>,
    ) -> Self {
        let mut cat = Self::new(id, name, group_id);
        cat.is_income = true;
        cat
    }

    /// Returns whether this category is active (not hidden and not tombstoned).
    #[must_use]
    pub fn is_active(&self) -> bool {
        !self.hidden && !self.tombstone
    }
}
