//! Cash flow analytics report.
//!
//! Computes monthly total income, total expenses, net cash flow,
//! and savings rate over historical transaction data.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::cents::Cents;
use crate::reports::filters::ReportFilter;
use crate::transaction::Transaction;

/// A single month's cash flow summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonthlyCashFlow {
    /// Month key in `"YYYY-MM"` format.
    pub month: String,
    /// Total income/inflow for the month (positive cents).
    pub income: Cents,
    /// Total expense/outflow for the month (positive cents representing amount spent).
    pub expenses: Cents,
    /// Net cash flow = income - expenses.
    pub net: Cents,
    /// Savings rate as a percentage between -100.0 and 100.0+ (net / income * 100).
    /// If income <= 0, savings rate is 0.0.
    pub savings_rate: f64,
}

/// Computed cash flow report over a date range.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashFlowReport {
    /// Monthly summaries in chronological order.
    pub months: Vec<MonthlyCashFlow>,
    /// Total cumulative income across the period.
    pub total_income: Cents,
    /// Total cumulative expenses across the period.
    pub total_expenses: Cents,
    /// Overall net cash flow.
    pub total_net: Cents,
    /// Overall period savings rate.
    pub overall_savings_rate: f64,
}

impl CashFlowReport {
    /// Computes cash flow analytics from transactions according to the given filter.
    ///
    /// For cash flow:
    /// - Inflows (`amount > 0`) contribute to income.
    /// - Outflows (`amount < 0`) contribute to expenses (as positive amount).
    /// - Internal account transfers (`transfer_id.is_some()`) should normally be excluded
    ///   or filtered unless explicitly desired. By default, transactions with `transfer_id.is_some()`
    ///   are excluded to prevent double counting.
    #[must_use]
    pub fn compute(transactions: &[Transaction], filter: &ReportFilter) -> Self {
        let mut monthly_data: BTreeMap<String, (i64, i64)> = BTreeMap::new();

        for tx in transactions {
            // Ignore tombstoned, parent split holders, or transfers
            if tx.tombstone || tx.is_parent || tx.transfer_id.is_some() {
                continue;
            }
            if !filter.matches(tx) {
                continue;
            }

            let month_key = tx.date.format("%Y-%m").to_string();
            let entry = monthly_data.entry(month_key).or_insert((0, 0));

            let cents = tx.amount.as_cents();
            if cents > 0 {
                entry.0 += cents; // income
            } else if cents < 0 {
                entry.1 += -cents; // expense
            }
        }

        let mut months = Vec::new();
        let mut sum_income = 0i64;
        let mut sum_expenses = 0i64;

        for (month, (income_cents, expense_cents)) in monthly_data {
            let income = Cents::from_cents(income_cents);
            let expenses = Cents::from_cents(expense_cents);
            let net = Cents::from_cents(income_cents - expense_cents);

            let savings_rate = if income_cents > 0 {
                ((income_cents - expense_cents) as f64 / income_cents as f64) * 100.0
            } else {
                0.0
            };

            sum_income += income_cents;
            sum_expenses += expense_cents;

            months.push(MonthlyCashFlow {
                month,
                income,
                expenses,
                net,
                savings_rate,
            });
        }

        let total_income = Cents::from_cents(sum_income);
        let total_expenses = Cents::from_cents(sum_expenses);
        let total_net = Cents::from_cents(sum_income - sum_expenses);
        let overall_savings_rate = if sum_income > 0 {
            ((sum_income - sum_expenses) as f64 / sum_income as f64) * 100.0
        } else {
            0.0
        };

        Self {
            months,
            total_income,
            total_expenses,
            total_net,
            overall_savings_rate,
        }
    }
}
