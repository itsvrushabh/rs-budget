//! UI rendering functions and layout using Ratatui widgets.

use actual_core::Cents;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Row, Table, Tabs};
use ratatui::Frame;

use crate::app::{ActiveModal, ActiveView, App, BudgetItem};

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header & Month
            Constraint::Min(8),    // Main Content
            Constraint::Length(2), // Footer & Status
        ])
        .split(frame.area());

    render_header(frame, app, chunks[0]);

    match app.active_view {
        ActiveView::Budget => render_budget_view(frame, app, chunks[1]),
        ActiveView::Accounts => render_accounts_view(frame, app, chunks[1]),
    }

    render_footer(frame, app, chunks[2]);

    // Modals
    match app.active_modal {
        ActiveModal::Help => render_help_modal(frame),
        ActiveModal::EditBudget => render_edit_budget_modal(frame, app),
        ActiveModal::AddTransaction => render_add_transaction_modal(frame, app),
        ActiveModal::None => {}
    }
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(30), // Tabs
            Constraint::Min(20),    // Month navigation
            Constraint::Length(35), // To Budget / Available
        ])
        .split(area);

    // Tabs
    let tab_titles = vec![" [1] Budget ", " [2] Accounts "];
    let selected_tab = match app.active_view {
        ActiveView::Budget => 0,
        ActiveView::Accounts => 1,
    };
    let tabs = Tabs::new(tab_titles)
        .block(Block::default().borders(Borders::BOTTOM))
        .select(selected_tab)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(tabs, header_chunks[0]);

    // Month Navigation
    let m = app.current_month_str();
    let month_text = vec![
        Span::raw("◀ [h] "),
        Span::styled(
            format!(" {m} "),
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::White),
        ),
        Span::raw(" [l] ▶"),
    ];
    let month_para = Paragraph::new(Line::from(month_text))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(month_para, header_chunks[1]);

    // Available Funds / To Budget
    let to_budget = app.svc.budget().get_to_budget(&m);
    let tb_color = if to_budget.as_cents() < 0 {
        Color::LightRed
    } else {
        Color::LightGreen
    };
    let tb_text = vec![
        Span::raw("To Budget: "),
        Span::styled(
            format_cents(to_budget),
            Style::default().fg(tb_color).add_modifier(Modifier::BOLD),
        ),
    ];
    let tb_para = Paragraph::new(Line::from(tb_text))
        .alignment(Alignment::Right)
        .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(tb_para, header_chunks[2]);
}

fn render_budget_view(frame: &mut Frame, app: &App, area: Rect) {
    let items = app.build_budget_items();
    let rows: Vec<Row> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_selected = i == app.budget_cursor;
            match item {
                BudgetItem::Group {
                    name,
                    total_budgeted,
                    total_spent,
                    total_leftover,
                    ..
                } => {
                    let style = Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD);
                    Row::new(vec![
                        format!("▾ {name}"),
                        format_cents(*total_budgeted),
                        format_cents(*total_spent),
                        format_cents(*total_leftover),
                        String::new(),
                    ])
                    .style(style)
                }
                BudgetItem::Category {
                    name,
                    budgeted,
                    spent,
                    leftover,
                    carryover,
                    ..
                } => {
                    let mut style = Style::default();
                    if is_selected {
                        style = style
                            .bg(Color::Rgb(40, 50, 70))
                            .add_modifier(Modifier::BOLD);
                    }

                    let leftover_style = if leftover.as_cents() < 0 {
                        Style::default().fg(Color::Red)
                    } else if leftover.as_cents() > 0 {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::Gray)
                    };

                    Row::new(vec![
                        Span::raw(format!("   {name}")),
                        Span::raw(format_cents(*budgeted)),
                        Span::raw(format_cents(*spent)),
                        Span::styled(format_cents(*leftover), leftover_style),
                        Span::raw(if *carryover { "✓" } else { "" }),
                    ])
                    .style(style)
                }
            }
        })
        .collect();

    let widths = [
        Constraint::Percentage(40),
        Constraint::Percentage(15),
        Constraint::Percentage(15),
        Constraint::Percentage(15),
        Constraint::Percentage(15),
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["CATEGORY", "BUDGETED", "SPENT", "LEFTOVER", "CARRY"])
                .style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
                .bottom_margin(1),
        )
        .block(
            Block::default()
                .title(" Envelope Budget Allocation ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        );

    frame.render_widget(table, area);
}

fn render_accounts_view(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30), // Accounts sidebar
            Constraint::Percentage(70), // Transaction register
        ])
        .split(area);

    // Sidebar: Accounts
    let accounts = app.svc.accounts();
    let account_items: Vec<ListItem> = accounts
        .iter()
        .enumerate()
        .map(|(i, acc)| {
            let is_selected = app.selected_account_id.as_deref() == Some(&acc.id)
                || (app.selected_account_id.is_none() && i == 0);

            let txs = app.svc.transactions_for_account(&acc.id);
            let bal_cents: i64 = txs.iter().map(|t| t.amount.as_cents()).sum();
            let bal_str = format_cents(Cents::from_cents(bal_cents));

            let mut style = Style::default();
            if is_selected {
                style = style.fg(Color::Cyan).add_modifier(Modifier::BOLD);
            }

            ListItem::new(Line::from(vec![
                Span::styled(format!("{:<18}", acc.name), style),
                Span::styled(
                    format!("{:>10}", bal_str),
                    Style::default().fg(Color::Green),
                ),
            ]))
        })
        .collect();

    let accounts_list = List::new(account_items).block(
        Block::default()
            .title(" Accounts ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );
    frame.render_widget(accounts_list, chunks[0]);

    // Right: Transactions Register
    let reg_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search bar
            Constraint::Min(4),    // Transaction table
        ])
        .split(chunks[1]);

    // Search bar
    let search_bar = Paragraph::new(format!(" Search: {}", app.search_query)).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Filter (press '/' to type) "),
    );
    frame.render_widget(search_bar, reg_chunks[0]);

    // Transactions Table
    let txs = app.filtered_transactions();
    let rows: Vec<Row> = txs
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let is_selected = i == app.transaction_cursor;
            let mut style = Style::default();
            if is_selected {
                style = style
                    .bg(Color::Rgb(40, 50, 70))
                    .add_modifier(Modifier::BOLD);
            }

            let amt_style = if t.amount.as_cents() < 0 {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::Green)
            };

            let payee_name = t.payee_id.as_deref().unwrap_or("—");
            let cat_name = t
                .category_id
                .as_deref()
                .and_then(|cid| app.svc.category(cid))
                .map(|c| c.name.as_str())
                .unwrap_or("—");

            Row::new(vec![
                Span::raw(t.date.to_string()),
                Span::raw(payee_name),
                Span::raw(cat_name),
                Span::styled(format_cents(t.amount), amt_style),
                Span::raw(if t.cleared { "✓" } else { "" }),
            ])
            .style(style)
        })
        .collect();

    let widths = [
        Constraint::Length(12),
        Constraint::Percentage(35),
        Constraint::Percentage(25),
        Constraint::Length(15),
        Constraint::Length(6),
    ];

    let tx_table = Table::new(rows, widths)
        .header(
            Row::new(vec!["DATE", "PAYEE", "CATEGORY", "AMOUNT", "CLR"]).style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        )
        .block(
            Block::default()
                .title(" Transactions Register ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        );

    frame.render_widget(tx_table, reg_chunks[1]);
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let msg = app.status_message.as_deref().unwrap_or(
        "[Tab] View | [j/k] Navigate | [h/l] Month | [e] Edit Budget | [a] Add Tx | [?] Help | [q] Quit",
    );
    let footer = Paragraph::new(msg)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    frame.render_widget(footer, area);
}

// ─── Modals ───────────────────────────────────────────────────────────────────

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn render_help_modal(frame: &mut Frame) {
    let area = centered_rect(60, 60, frame.area());
    frame.render_widget(Clear, area);

    let text = vec![
        Line::from(vec![Span::styled(
            "rs-budget TUI Shortcuts",
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Cyan),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Tab / 1, 2", Style::default().fg(Color::Yellow)),
            Span::raw("  Switch between Budget and Accounts view"),
        ]),
        Line::from(vec![
            Span::styled("j / k / ↑ / ↓", Style::default().fg(Color::Yellow)),
            Span::raw(" Navigate categories or transactions"),
        ]),
        Line::from(vec![
            Span::styled("h / l / ← / →", Style::default().fg(Color::Yellow)),
            Span::raw(" Previous / Next month in budget view"),
        ]),
        Line::from(vec![
            Span::styled("e / Enter", Style::default().fg(Color::Yellow)),
            Span::raw("      Edit budgeted envelope amount"),
        ]),
        Line::from(vec![
            Span::styled("a", Style::default().fg(Color::Yellow)),
            Span::raw("              Add a new transaction"),
        ]),
        Line::from(vec![
            Span::styled("/", Style::default().fg(Color::Yellow)),
            Span::raw("              Filter transactions in Accounts view"),
        ]),
        Line::from(vec![
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::raw("            Close active modal or clear search"),
        ]),
        Line::from(vec![
            Span::styled("? / F1", Style::default().fg(Color::Yellow)),
            Span::raw("         Toggle this help modal"),
        ]),
        Line::from(vec![
            Span::styled("q / Ctrl+C", Style::default().fg(Color::Yellow)),
            Span::raw("     Quit application"),
        ]),
    ];

    let block = Block::default()
        .title(" Help & Keybindings ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let p = Paragraph::new(text).block(block).alignment(Alignment::Left);
    frame.render_widget(p, area);
}

fn render_edit_budget_modal(frame: &mut Frame, app: &App) {
    let area = centered_rect(50, 25, frame.area());
    frame.render_widget(Clear, area);

    let text = vec![
        Line::from(vec![
            Span::raw("Category: "),
            Span::styled(
                &app.edit_category_name,
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::raw("New Budget Amount ($): "),
            Span::styled(
                format!("{}_", app.edit_amount_input),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Press Enter to save, Esc to cancel",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let block = Block::default()
        .title(" Edit Budget Allocation ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let p = Paragraph::new(text).block(block);
    frame.render_widget(p, area);
}

fn render_add_transaction_modal(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 45, frame.area());
    frame.render_widget(Clear, area);

    let fields = [
        ("Amount ($)", &app.add_tx_amount),
        ("Payee", &app.add_tx_payee),
        ("Category", &app.add_tx_category),
        ("Notes", &app.add_tx_notes),
    ];

    let mut lines = Vec::new();
    lines.push(Line::from(Span::styled(
        "Record New Transaction",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    for (i, (label, val)) in fields.iter().enumerate() {
        let is_focused = i == app.add_tx_field_idx;
        let style = if is_focused {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        let cursor_char = if is_focused { "_" } else { "" };
        lines.push(Line::from(vec![
            Span::styled(format!("{:<15}: ", label), style),
            Span::styled(format!("{}{}", val, cursor_char), style),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Tab: next field | Enter: submit | Esc: cancel",
        Style::default().fg(Color::DarkGray),
    )));

    let block = Block::default()
        .title(" Add Transaction ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let p = Paragraph::new(lines).block(block);
    frame.render_widget(p, area);
}

fn format_cents(c: Cents) -> String {
    let raw = c.as_cents();
    let abs = raw.abs();
    let dollars = abs / 100;
    let fraction = abs % 100;
    let sign = if raw < 0 { "-" } else { "" };
    format!("{sign}${dollars}.{fraction:02}")
}
