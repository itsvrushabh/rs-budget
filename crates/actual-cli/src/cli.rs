//! Command-line argument definitions using `clap`.

use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

/// Command-line interface for rs-budget.
#[derive(Debug, Parser)]
#[command(
    name = "actual",
    about = "100% Rust command-line budgeting client for rs-budget (Actual Budget parity)",
    version
)]
pub struct Cli {
    /// Path to budget SQLite database file.
    #[arg(long, global = true, default_value = "budget.sqlite")]
    pub db: PathBuf,

    /// Run with an ephemeral in-memory database (useful for testing and scripting).
    #[arg(long, global = true)]
    pub in_memory: bool,

    /// Machine-readable JSON output mode.
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Inspect and update monthly envelope budgets.
    Budget(BudgetArgs),
    /// Manage financial accounts (checking, savings, credit cards).
    Account(AccountArgs),
    /// Inspect, record, and delete transactions.
    Transaction(TransactionArgs),
    /// Batch import financial statement files (QIF, OFX, CSV).
    Import(ImportArgs),
    /// Export transactions and budget data to JSON or CSV.
    Export(ExportArgs),
    /// Manage and execute transaction automation rules.
    Rules(RulesArgs),
    /// View recurring schedules and generate upcoming transactions.
    Schedule(ScheduleArgs),
}

// ─── Budget Subcommands ────────────────────────────────────────────────────────

#[derive(Debug, Args)]
pub struct BudgetArgs {
    #[command(subcommand)]
    pub command: BudgetSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum BudgetSubcommand {
    /// Inspect monthly envelope allocations, leftover balances, and available funds.
    Inspect {
        /// Month in "YYYY-MM" format (defaults to current month if omitted).
        #[arg(long)]
        month: Option<String>,
    },
    /// Set the budgeted amount for a category envelope in a month.
    Set {
        /// Month in "YYYY-MM" format.
        #[arg(long)]
        month: String,
        /// Category UUID or exact name.
        #[arg(long)]
        category: String,
        /// Amount in dollars (e.g. "150.00" or "-50.25").
        #[arg(long, allow_hyphen_values = true)]
        amount: f64,
    },
    /// Toggle carry-over for an overspent category envelope.
    Carryover {
        /// Month in "YYYY-MM" format.
        #[arg(long)]
        month: String,
        /// Category UUID or exact name.
        #[arg(long)]
        category: String,
        /// Carry over spending to next month.
        #[arg(long)]
        carry: bool,
    },
}

// ─── Account Subcommands ───────────────────────────────────────────────────────

#[derive(Debug, Args)]
pub struct AccountArgs {
    #[command(subcommand)]
    pub command: AccountSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum AccountSubcommand {
    /// List open (and optionally closed) accounts with current balances.
    List {
        /// Include closed accounts.
        #[arg(long)]
        all: bool,
    },
    /// Create a new financial account.
    Create {
        /// Display name for the account.
        #[arg(long)]
        name: String,
        /// Whether this is an off-budget / tracking account.
        #[arg(long)]
        offbudget: bool,
    },
    /// Close an account.
    Close {
        /// Account ID.
        #[arg(long)]
        id: String,
    },
    /// Reopen a closed account.
    Reopen {
        /// Account ID.
        #[arg(long)]
        id: String,
    },
}

// ─── Transaction Subcommands ───────────────────────────────────────────────────

#[derive(Debug, Args)]
pub struct TransactionArgs {
    #[command(subcommand)]
    pub command: TransactionSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum TransactionSubcommand {
    /// List transactions with optional account and date filters.
    List {
        /// Filter by Account ID.
        #[arg(long)]
        account: Option<String>,
        /// Filter by month in "YYYY-MM" format.
        #[arg(long)]
        month: Option<String>,
        /// Maximum number of transactions to display.
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },
    /// Add a new transaction.
    Add {
        /// Account ID.
        #[arg(long)]
        account: String,
        /// Date in YYYY-MM-DD format (defaults to today).
        #[arg(long)]
        date: Option<String>,
        /// Amount in dollars (positive = income, negative = expense).
        #[arg(long, allow_hyphen_values = true)]
        amount: f64,
        /// Payee name or UUID.
        #[arg(long)]
        payee: Option<String>,
        /// Category name or UUID.
        #[arg(long)]
        category: Option<String>,
        /// Memo or notes.
        #[arg(long)]
        notes: Option<String>,
        /// Whether the transaction has cleared.
        #[arg(long, default_value_t = true)]
        cleared: bool,
    },
    /// Delete a transaction.
    Delete {
        /// Transaction UUID.
        #[arg(long)]
        id: String,
    },
}

// ─── Import Subcommands ────────────────────────────────────────────────────────

#[derive(Debug, Args)]
pub struct ImportArgs {
    /// Target Account ID to import transactions into.
    #[arg(long)]
    pub account: String,
    /// Path to file (QIF, OFX, or CSV).
    #[arg(long)]
    pub file: PathBuf,
    /// Format: auto, qif, ofx, csv (defaults to auto detection by file extension).
    #[arg(long, default_value = "auto")]
    pub format: ImportFormat,
    /// Proximity deduplication window in days (default 3).
    #[arg(long, default_value_t = 3)]
    pub proximity_days: i64,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ImportFormat {
    Auto,
    Qif,
    Ofx,
    Csv,
}

// ─── Export Subcommands ────────────────────────────────────────────────────────

#[derive(Debug, Args)]
pub struct ExportArgs {
    /// Format: json or csv.
    #[arg(long, default_value = "json")]
    pub format: ExportFormat,
    /// Filter by Account ID (optional).
    #[arg(long)]
    pub account: Option<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ExportFormat {
    Json,
    Csv,
}

// ─── Rules Subcommands ─────────────────────────────────────────────────────────

#[derive(Debug, Args)]
pub struct RulesArgs {
    #[command(subcommand)]
    pub command: RulesSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum RulesSubcommand {
    /// List active transaction automation rules.
    List,
    /// Apply automation rules against existing transactions.
    Apply {
        /// Account ID to apply rules to (all accounts if omitted).
        #[arg(long)]
        account: Option<String>,
    },
}

// ─── Schedule Subcommands ──────────────────────────────────────────────────────

#[derive(Debug, Args)]
pub struct ScheduleArgs {
    #[command(subcommand)]
    pub command: ScheduleSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ScheduleSubcommand {
    /// List all recurring schedules.
    List,
    /// Generate upcoming transactions for a schedule.
    Upcoming {
        /// Schedule ID.
        #[arg(long)]
        id: String,
        /// Date until which to generate upcoming transactions (YYYY-MM-DD).
        #[arg(long)]
        until: String,
    },
}
