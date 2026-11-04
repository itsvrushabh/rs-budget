//! Net worth progression report.
//!
//! Computes monthly snapshots of total assets minus total liabilities from
//! the transaction history.

use std::collections::{BTreeMap, HashMap};

use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::account::Account;
use crate::cents::Cents;
use crate::reports::filters::ReportFilter;
use crate::transaction::Transaction;

/// A single month's net worth snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonthlyNetWorth {
    /// Month key in `"YYYY-MM"` format.
    pub month: String,
    /// Total balance across all asset accounts at end of month.
    pub total_assets: Cents,
    /// Total balance across all liability accounts at end of month (positive = debt owed).
    pub total_liabilities: Cents,
    /// Net worth = assets - |liabilities|.
    pub net_worth: Cents,
}

/// Computed net worth report across a date range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetWorthReport {
    /// Monthly snapshots in chronological order.
    pub months: Vec<MonthlyNetWorth>,
}

/// Classification of an account for net worth purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountType {
    /// Asset: checking, savings, investment, cash — positive balances represent wealth.
    Asset,
    /// Liability: credit card, loan — negative balances represent debt.
    Liability,
}

impl NetWorthReport {
    /// Compute net worth progression from transaction history.
    ///
    /// # Parameters
    /// - `transactions` — all transactions (pre-sorted not required).
    /// - `accounts` — full account list for classification.
    /// - `account_types` — maps account ID → [`AccountType`]. Accounts not present
    ///   default to [`AccountType::Asset`].
    /// - `filter` — optional date range and account restrictions.
    ///
    /// The report produces one entry per month covering the filter's date range.
    /// Each month accumulates running balances from all prior transactions up to
    /// and including the last day of that month.
    #[must_use]
    pub fn compute(
        transactions: &[Transaction],
        accounts: &[Account],
        account_types: &HashMap<String, AccountType>,
        filter: &ReportFilter,
    ) -> Self {
        // Build running balance per account.
        let mut sorted_txs: Vec<&Transaction> = transactions
            .iter()
            .filter(|tx| !tx.tombstone && !tx.is_parent)
            .collect();
        sorted_txs.sort_by_key(|tx| tx.date);

        if sorted_txs.is_empty() {
            return Self { months: Vec::new() };
        }

        // Determine date range.
        let start = filter
            .start_date
            .unwrap_or(sorted_txs.first().unwrap().date);
        let end = filter.end_date.unwrap_or(sorted_txs.last().unwrap().date);

        if start > end {
            return Self { months: Vec::new() };
        }

        // Build set of relevant account IDs.
        let account_set: Vec<&str> = if filter.account_ids.is_empty() {
            accounts.iter().map(|a| a.id.as_str()).collect()
        } else {
            filter.account_ids.iter().map(|s| s.as_str()).collect()
        };

        // Running balance per account — accumulate ALL transactions up to each month.
        let mut balances: HashMap<&str, i64> = HashMap::new();

        // Collect month boundaries.
        let mut months_map: BTreeMap<String, (Cents, Cents)> = BTreeMap::new();

        // Generate all months in range.
        let mut cursor = first_of_month(start);
        while cursor <= end {
            let month_key = cursor.format("%Y-%m").to_string();
            months_map
                .entry(month_key)
                .or_insert((Cents::ZERO, Cents::ZERO));
            cursor = next_month(cursor);
        }

        // Process transactions in date order, snapshotting at month boundaries.
        let mut tx_idx = 0;

        for month_key in months_map.keys().cloned().collect::<Vec<_>>() {
            let month_end = last_day_of_month_from_key(&month_key);

            // Accumulate transactions up to this month's end.
            while tx_idx < sorted_txs.len() && sorted_txs[tx_idx].date <= month_end {
                let tx = sorted_txs[tx_idx];
                if account_set.contains(&tx.account_id.as_str()) {
                    *balances.entry(tx.account_id.as_str()).or_insert(0) += tx.amount.as_cents();
                }
                tx_idx += 1;
            }

            // Compute assets and liabilities.
            let mut assets = 0i64;
            let mut liabilities = 0i64;
            for acct_id in &account_set {
                let bal = balances.get(acct_id).copied().unwrap_or(0);
                let acct_type = account_types
                    .get(*acct_id)
                    .copied()
                    .unwrap_or(AccountType::Asset);
                match acct_type {
                    AccountType::Asset => assets += bal,
                    AccountType::Liability => liabilities += bal,
                }
            }

            months_map.insert(
                month_key,
                (Cents::from_cents(assets), Cents::from_cents(liabilities)),
            );
        }

        let months = months_map
            .into_iter()
            .map(|(month, (total_assets, total_liabilities))| {
                let net_worth = total_assets + total_liabilities;
                MonthlyNetWorth {
                    month,
                    total_assets,
                    total_liabilities,
                    net_worth,
                }
            })
            .collect();

        Self { months }
    }
}

/// Return the first day of the month containing `date`.
fn first_of_month(date: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap()
}

/// Return the first day of the next month.
fn next_month(date: NaiveDate) -> NaiveDate {
    if date.month() == 12 {
        NaiveDate::from_ymd_opt(date.year() + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(date.year(), date.month() + 1, 1).unwrap()
    }
}

/// Parse a `"YYYY-MM"` key and return the last day of that month.
fn last_day_of_month_from_key(key: &str) -> NaiveDate {
    let parts: Vec<&str> = key.split('-').collect();
    let year: i32 = parts[0].parse().unwrap();
    let month: u32 = parts[1].parse().unwrap();
    let first = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let next = next_month(first);
    next.pred_opt().unwrap()
}
