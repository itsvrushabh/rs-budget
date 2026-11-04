//! Terminal keyboard event handling.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{ActiveModal, ActiveView, App};

pub fn handle_key(app: &mut App, key: KeyEvent) {
    // Global quit
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }

    // Modal key handling
    match app.active_modal {
        ActiveModal::Help => {
            if matches!(
                key.code,
                KeyCode::Esc | KeyCode::Char('?') | KeyCode::Enter | KeyCode::Char('q')
            ) {
                app.active_modal = ActiveModal::None;
            }
            return;
        }
        ActiveModal::EditBudget => {
            match key.code {
                KeyCode::Esc => app.active_modal = ActiveModal::None,
                KeyCode::Enter => app.commit_edit_budget(),
                KeyCode::Backspace => {
                    app.edit_amount_input.pop();
                }
                KeyCode::Char(c) if c.is_ascii_digit() || c == '.' || c == '-' => {
                    app.edit_amount_input.push(c);
                }
                _ => {}
            }
            return;
        }
        ActiveModal::AddTransaction => {
            match key.code {
                KeyCode::Esc => app.active_modal = ActiveModal::None,
                KeyCode::Tab => {
                    app.add_tx_field_idx = (app.add_tx_field_idx + 1) % 4;
                }
                KeyCode::BackTab => {
                    app.add_tx_field_idx = if app.add_tx_field_idx == 0 {
                        3
                    } else {
                        app.add_tx_field_idx - 1
                    };
                }
                KeyCode::Enter => app.commit_add_transaction(),
                KeyCode::Backspace => match app.add_tx_field_idx {
                    0 => {
                        app.add_tx_amount.pop();
                    }
                    1 => {
                        app.add_tx_payee.pop();
                    }
                    2 => {
                        app.add_tx_category.pop();
                    }
                    3 => {
                        app.add_tx_notes.pop();
                    }
                    _ => {}
                },
                KeyCode::Char(c) => match app.add_tx_field_idx {
                    0 if c.is_ascii_digit() || c == '.' || c == '-' => app.add_tx_amount.push(c),
                    1 => app.add_tx_payee.push(c),
                    2 => app.add_tx_category.push(c),
                    3 => app.add_tx_notes.push(c),
                    _ => {}
                },
                _ => {}
            }
            return;
        }
        ActiveModal::None => {}
    }

    // In-view Search bar handling
    if app.search_active {
        match key.code {
            KeyCode::Esc => {
                app.search_query.clear();
                app.search_active = false;
            }
            KeyCode::Enter => {
                app.search_active = false;
            }
            KeyCode::Backspace => {
                app.search_query.pop();
            }
            KeyCode::Char(c) => {
                app.search_query.push(c);
            }
            _ => {}
        }
        return;
    }

    // Normal navigation mode
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('?') | KeyCode::F(1) => app.active_modal = ActiveModal::Help,
        KeyCode::Tab => {
            app.active_view = match app.active_view {
                ActiveView::Budget => ActiveView::Accounts,
                ActiveView::Accounts => ActiveView::Budget,
            };
        }
        KeyCode::Char('1') => app.active_view = ActiveView::Budget,
        KeyCode::Char('2') => app.active_view = ActiveView::Accounts,

        // Month navigation
        KeyCode::Char('h') | KeyCode::Left => app.prev_month(),
        KeyCode::Char('l') | KeyCode::Right => app.next_month(),

        // Vertical cursor movement
        KeyCode::Char('j') | KeyCode::Down => match app.active_view {
            ActiveView::Budget => {
                let count = app.build_budget_items().len();
                if count > 0 && app.budget_cursor + 1 < count {
                    app.budget_cursor += 1;
                }
            }
            ActiveView::Accounts => {
                let count = app.filtered_transactions().len();
                if count > 0 && app.transaction_cursor + 1 < count {
                    app.transaction_cursor += 1;
                }
            }
        },
        KeyCode::Char('k') | KeyCode::Up => match app.active_view {
            ActiveView::Budget => {
                if app.budget_cursor > 0 {
                    app.budget_cursor -= 1;
                }
            }
            ActiveView::Accounts => {
                if app.transaction_cursor > 0 {
                    app.transaction_cursor -= 1;
                }
            }
        },

        // Actions
        KeyCode::Char('e') | KeyCode::Enter if app.active_view == ActiveView::Budget => {
            app.open_edit_budget();
        }
        KeyCode::Char('a') => {
            app.open_add_transaction();
        }
        KeyCode::Char('/') if app.active_view == ActiveView::Accounts => {
            app.search_active = true;
        }

        _ => {}
    }
}
