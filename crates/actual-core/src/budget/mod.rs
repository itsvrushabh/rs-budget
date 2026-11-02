//! Envelope budgeting domain logic, reactive calculation formulas, and rollover rules.

pub mod envelope;

pub use envelope::{
    cell_available_funds, cell_budget, cell_buffered, cell_carryover, cell_from_last_month,
    cell_group_budget, cell_group_leftover, cell_group_spent, cell_last_month_overspent,
    cell_leftover, cell_spent, cell_to_budget, cell_total_budgeted, cell_total_income,
    cell_total_leftover, cell_total_spent, cell_uncategorized_income, next_month, parse_year_month,
    prev_month, EnvelopeBudget,
};
