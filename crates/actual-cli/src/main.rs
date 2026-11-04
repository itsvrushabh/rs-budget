//! `actual-cli` provides non-interactive command-line budgeting workflows for rs-budget.

mod cli;
mod handlers;
mod output;

use actual_api::BudgetService;
use clap::Parser;
use cli::{Cli, Commands};

fn main() {
    let args = Cli::parse();

    let mut svc = if args.in_memory {
        match BudgetService::open_in_memory() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error opening in-memory database: {e}");
                std::process::exit(1);
            }
        }
    } else {
        match BudgetService::open(&args.db) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error opening database at {}: {e}", args.db.display());
                std::process::exit(1);
            }
        }
    };

    let result = match args.command {
        Commands::Budget(sub) => handlers::budget::handle(&mut svc, sub.command, args.json),
        Commands::Account(sub) => handlers::account::handle(&mut svc, sub.command, args.json),
        Commands::Transaction(sub) => {
            handlers::transaction::handle(&mut svc, sub.command, args.json)
        }
        Commands::Import(sub) => handlers::import::handle(&mut svc, sub, args.json),
        Commands::Export(sub) => handlers::export::handle(&svc, sub, args.json),
        Commands::Rules(sub) => handlers::rules::handle(&mut svc, sub.command, args.json),
        Commands::Schedule(sub) => handlers::schedule::handle(&svc, sub.command, args.json),
    };

    if let Err(err) = result {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}
