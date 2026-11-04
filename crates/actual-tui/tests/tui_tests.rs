//! Interactive TUI tests using Ratatui TestBackend (Issue #16).

#[path = "../src/app.rs"]
mod app;
#[path = "../src/input.rs"]
mod input;
#[path = "../src/ui.rs"]
mod ui;

use actual_api::BudgetService;
use app::{ActiveModal, ActiveView, App};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn test_app_initialization_and_default_categories() {
    let svc = BudgetService::open_in_memory().unwrap();
    let app = App::new(svc);

    assert_eq!(app.active_view, ActiveView::Budget);
    assert_eq!(app.active_modal, ActiveModal::None);

    let items = app.build_budget_items();
    // Default categories seeded: 2 groups + 4 categories = 6 items
    assert_eq!(items.len(), 6);
}

#[test]
fn test_month_navigation() {
    let svc = BudgetService::open_in_memory().unwrap();
    let mut app = App::new(svc);

    let initial_month = app.current_month_str();

    // Navigate next month: 'l'
    input::handle_key(&mut app, key(KeyCode::Char('l')));
    assert_ne!(app.current_month_str(), initial_month);

    // Navigate back: 'h'
    input::handle_key(&mut app, key(KeyCode::Char('h')));
    assert_eq!(app.current_month_str(), initial_month);
}

#[test]
fn test_view_switching() {
    let svc = BudgetService::open_in_memory().unwrap();
    let mut app = App::new(svc);

    assert_eq!(app.active_view, ActiveView::Budget);

    // Tab switches to Accounts
    input::handle_key(&mut app, key(KeyCode::Tab));
    assert_eq!(app.active_view, ActiveView::Accounts);

    // '1' switches back to Budget
    input::handle_key(&mut app, key(KeyCode::Char('1')));
    assert_eq!(app.active_view, ActiveView::Budget);
}

#[test]
fn test_help_modal_toggle() {
    let svc = BudgetService::open_in_memory().unwrap();
    let mut app = App::new(svc);

    assert_eq!(app.active_modal, ActiveModal::None);

    // Open help with '?'
    input::handle_key(&mut app, key(KeyCode::Char('?')));
    assert_eq!(app.active_modal, ActiveModal::Help);

    // Close help with Esc
    input::handle_key(&mut app, key(KeyCode::Esc));
    assert_eq!(app.active_modal, ActiveModal::None);
}

#[test]
fn test_edit_budget_amount_flow() {
    let svc = BudgetService::open_in_memory().unwrap();
    let mut app = App::new(svc);

    // Move cursor down to first category (index 1 after group header)
    input::handle_key(&mut app, key(KeyCode::Char('j')));
    assert_eq!(app.budget_cursor, 1);

    // Open edit with 'e'
    input::handle_key(&mut app, key(KeyCode::Char('e')));
    assert_eq!(app.active_modal, ActiveModal::EditBudget);

    // Type 500
    app.edit_amount_input.clear();
    input::handle_key(&mut app, key(KeyCode::Char('5')));
    input::handle_key(&mut app, key(KeyCode::Char('0')));
    input::handle_key(&mut app, key(KeyCode::Char('0')));
    assert_eq!(app.edit_amount_input, "500");

    // Commit with Enter
    input::handle_key(&mut app, key(KeyCode::Enter));
    assert_eq!(app.active_modal, ActiveModal::None);

    // Verify budget allocation was updated in EnvelopeBudget
    let m = app.current_month_str();
    let b = app
        .svc
        .budget()
        .get_category_budget(&m, &app.edit_category_id);
    assert_eq!(b.as_cents(), 50000); // $500.00
}

#[test]
fn test_add_transaction_flow() {
    let svc = BudgetService::open_in_memory().unwrap();
    let mut app = App::new(svc);

    // Open add transaction modal with 'a'
    input::handle_key(&mut app, key(KeyCode::Char('a')));
    assert_eq!(app.active_modal, ActiveModal::AddTransaction);

    // Type amount: "-35.50"
    for c in "-35.50".chars() {
        input::handle_key(&mut app, key(KeyCode::Char(c)));
    }
    assert_eq!(app.add_tx_amount, "-35.50");

    // Tab to Payee
    input::handle_key(&mut app, key(KeyCode::Tab));
    assert_eq!(app.add_tx_field_idx, 1);
    for c in "Coffee Shop".chars() {
        input::handle_key(&mut app, key(KeyCode::Char(c)));
    }

    // Submit with Enter
    input::handle_key(&mut app, key(KeyCode::Enter));
    assert_eq!(app.active_modal, ActiveModal::None);

    // Verify transaction recorded
    assert_eq!(app.svc.transactions().len(), 1);
    assert_eq!(app.svc.transactions()[0].amount.as_cents(), -3550);
}

#[test]
fn test_tui_render_with_test_backend() {
    let svc = BudgetService::open_in_memory().unwrap();
    let app = App::new(svc);

    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    terminal.draw(|f| ui::render(f, &app)).unwrap();
    let buffer = terminal.backend().buffer();

    // Verify header and table strings rendered into terminal cells
    let content = format!("{buffer:?}");
    assert!(content.contains("Envelope Budget Allocation") || content.contains("CATEGORY"));
    assert!(content.contains("To Budget"));
}
