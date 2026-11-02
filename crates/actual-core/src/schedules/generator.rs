//! Generator for upcoming scheduled transactions.

use chrono::NaiveDate;

use crate::schedules::schedule::Schedule;
use crate::transaction::Transaction;

/// Generates projected or upcoming transactions for a schedule within `[from, until]`.
#[must_use]
pub fn generate_upcoming_transactions(
    schedule: &Schedule,
    from: NaiveDate,
    until: NaiveDate,
) -> Vec<Transaction> {
    if !schedule.active || schedule.completed || schedule.tombstone || from > until {
        return Vec::new();
    }

    let mut projected = Vec::new();

    // Check if the start date itself falls within [from, until]
    let mut candidate = schedule.config.start_date;
    let mut occurrence_count = 1;

    while candidate <= until {
        if let Some(end_date) = schedule.config.end_date {
            if candidate > end_date {
                break;
            }
        }

        if let Some(max_occ) = schedule.config.end_after_occurrences {
            if occurrence_count > max_occ {
                break;
            }
        }

        if candidate >= from {
            let mut tx = Transaction::new(
                format!("{}_proj_{candidate}", schedule.id),
                schedule.account_id.clone(),
                candidate,
                schedule.amount,
            );
            tx.payee_id = schedule.payee_id.clone();
            tx.category_id = schedule.category_id.clone();
            tx.schedule_id = Some(schedule.id.clone());
            tx.cleared = false;
            projected.push(tx);
        }

        candidate = schedule.config.advance_date(candidate);
        occurrence_count += 1;
    }

    projected
}
