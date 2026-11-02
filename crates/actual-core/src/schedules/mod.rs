//! Recurring schedules engine for predicting upcoming transactions and automating recurrences.

pub mod generator;
pub mod recurrence;
pub mod schedule;

pub use generator::generate_upcoming_transactions;
pub use recurrence::{add_months_pinned, Frequency, ScheduleConfig, TimeUnit};
pub use schedule::Schedule;
