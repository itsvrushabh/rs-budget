//! Recurrence patterns, frequencies, and next date calculation.

use chrono::{Datelike, Days, Months, NaiveDate};
use serde::{Deserialize, Serialize};

/// Time units for custom recurrence frequencies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimeUnit {
    Days,
    Weeks,
    Months,
    Years,
}

/// Recurrence frequency defining how often a schedule repeats.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Frequency {
    Daily,
    Weekly,
    Biweekly,
    Monthly,
    Yearly,
    Custom { interval: u32, unit: TimeUnit },
}

/// Configuration settings for a recurrence schedule.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScheduleConfig {
    /// Date when the recurrence schedule begins.
    pub start_date: NaiveDate,
    /// Optional date when the recurrence schedule stops.
    pub end_date: Option<NaiveDate>,
    /// Recurrence repetition frequency.
    pub frequency: Frequency,
    /// Multiplier interval (e.g. every 2 weeks).
    pub interval: u32,
    /// Optional day-of-month pinning (1..=31).
    pub day_of_month: Option<u32>,
    /// Optional maximum number of occurrences before completing.
    pub end_after_occurrences: Option<u32>,
}

impl ScheduleConfig {
    /// Creates a standard new schedule configuration.
    #[must_use]
    pub fn new(start_date: NaiveDate, frequency: Frequency) -> Self {
        Self {
            start_date,
            end_date: None,
            frequency,
            interval: 1,
            day_of_month: None,
            end_after_occurrences: None,
        }
    }

    /// Computes the next scheduled occurrence strictly after the given `from` date.
    #[must_use]
    pub fn next_occurrence(&self, from: NaiveDate) -> Option<NaiveDate> {
        let mut curr = self.start_date;
        let mut occurrence_count = 1;

        if let Some(end_date) = self.end_date {
            if from >= end_date {
                return None;
            }
        }

        while curr <= from {
            if let Some(max_occ) = self.end_after_occurrences {
                if occurrence_count >= max_occ {
                    return None;
                }
            }

            curr = self.advance_date(curr);
            occurrence_count += 1;

            if let Some(end_date) = self.end_date {
                if curr > end_date {
                    return None;
                }
            }
        }

        Some(curr)
    }

    /// Advances the given date by one recurrence period according to frequency and interval.
    #[must_use]
    pub fn advance_date(&self, date: NaiveDate) -> NaiveDate {
        match self.frequency {
            Frequency::Daily => date + Days::new(u64::from(self.interval.max(1))),
            Frequency::Weekly => date + Days::new(u64::from(self.interval.max(1) * 7)),
            Frequency::Biweekly => date + Days::new(u64::from(self.interval.max(1) * 14)),
            Frequency::Monthly => {
                let months = self.interval.max(1);
                add_months_pinned(date, months, self.day_of_month)
            }
            Frequency::Yearly => {
                let years = self.interval.max(1);
                add_months_pinned(date, years * 12, self.day_of_month)
            }
            Frequency::Custom { interval, unit } => match unit {
                TimeUnit::Days => date + Days::new(u64::from(interval.max(1))),
                TimeUnit::Weeks => date + Days::new(u64::from(interval.max(1) * 7)),
                TimeUnit::Months => add_months_pinned(date, interval.max(1), self.day_of_month),
                TimeUnit::Years => add_months_pinned(date, interval.max(1) * 12, self.day_of_month),
            },
        }
    }
}

/// Helper function to add months with optional day-of-month pinning and end-of-month clamping.
#[must_use]
pub fn add_months_pinned(date: NaiveDate, months: u32, pinned_day: Option<u32>) -> NaiveDate {
    let target_day = pinned_day.unwrap_or_else(|| date.day());
    let base = date.checked_add_months(Months::new(months)).unwrap_or(date);

    let year = base.year();
    let month = base.month();

    // Clamp day to valid range for target month
    for d in (1..=target_day.min(31)).rev() {
        if let Some(clamped) = NaiveDate::from_ymd_opt(year, month, d) {
            return clamped;
        }
    }

    base
}
