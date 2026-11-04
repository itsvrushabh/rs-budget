//! `actual-tui` provides an interactive, keyboard-driven full-screen terminal client.

mod app;
mod input;
mod ui;

use std::io::{self, stdout};
use std::path::PathBuf;
use std::time::Duration;

use actual_api::BudgetService;
use app::App;
use clap::Parser;
use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

#[derive(Debug, Parser)]
#[command(
    name = "actual-tui",
    about = "Interactive terminal user interface for rs-budget envelope budgeting",
    version
)]
pub struct CliArgs {
    /// Path to budget SQLite database file.
    #[arg(long, default_value = "budget.sqlite")]
    pub db: PathBuf,

    /// Start with ephemeral in-memory database.
    #[arg(long)]
    pub in_memory: bool,
}

fn main() -> io::Result<()> {
    let args = CliArgs::parse();

    let svc = if args.in_memory {
        BudgetService::open_in_memory().map_err(|e| io::Error::other(e.to_string()))?
    } else {
        BudgetService::open(&args.db).map_err(|e| io::Error::other(e.to_string()))?
    };

    let mut app = App::new(svc);

    // Setup terminal
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;

    // Run event loop
    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("Error in TUI event loop: {err}");
    }

    Ok(())
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    while !app.should_quit {
        terminal.draw(|f| ui::render(f, app))?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                input::handle_key(app, key);
            }
        }
    }
    Ok(())
}
