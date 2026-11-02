//! Recurring schedule domain entity.

use serde::{Deserialize, Serialize};

use crate::cents::Cents;
use crate::schedules::recurrence::ScheduleConfig;

/// Recurring schedule entity representing predictable bills, incomes, or transfers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Schedule {
    /// Unique identifier (UUID).
    pub id: String,
    /// Optional display name.
    pub name: Option<String>,
    /// Optional linked rule identifier.
    pub rule_id: Option<String>,
    /// Associated bank or budget account.
    pub account_id: String,
    /// Associated payee.
    pub payee_id: Option<String>,
    /// Associated envelope category.
    pub category_id: Option<String>,
    /// Monetary amount (positive = income, negative = expense).
    pub amount: Cents,
    /// Recurrence pattern configuration.
    pub config: ScheduleConfig,
    /// Whether the schedule is active.
    pub active: bool,
    /// Whether the schedule has completed its required occurrences.
    pub completed: bool,
    /// Whether the schedule automatically posts real transactions.
    pub posts_transaction: bool,
    /// Soft-deletion flag.
    pub tombstone: bool,
}

impl Schedule {
    /// Creates a new active `Schedule`.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        account_id: impl Into<String>,
        amount: Cents,
        config: ScheduleConfig,
    ) -> Self {
        Self {
            id: id.into(),
            name: None,
            rule_id: None,
            account_id: account_id.into(),
            payee_id: None,
            category_id: None,
            amount,
            config,
            active: true,
            completed: false,
            posts_transaction: false,
            tombstone: false,
        }
    }

    /// Sets the payee identifier on the schedule.
    #[must_use]
    pub fn with_payee(mut self, payee_id: impl Into<String>) -> Self {
        self.payee_id = Some(payee_id.into());
        self
    }

    /// Sets the category identifier on the schedule.
    #[must_use]
    pub fn with_category(mut self, category_id: impl Into<String>) -> Self {
        self.category_id = Some(category_id.into());
        self
    }

    /// Sets the display name on the schedule.
    #[must_use]
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}
