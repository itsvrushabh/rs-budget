//! Financial reports and analytics engine.
//!
//! Provides net worth progression, cash flow analytics, and category spending
//! breakdown calculations over transaction histories.

mod cash_flow;
mod filters;
mod net_worth;
mod spending;

pub use cash_flow::{CashFlowReport, MonthlyCashFlow};
pub use filters::ReportFilter;
pub use net_worth::{AccountType, MonthlyNetWorth, NetWorthReport};
pub use spending::{CategorySpending, GroupSpending, SpendingReport};
