//! Comprehensive test suite for actual-gui: editable financial table components,
//! keyboard navigation, IME input buffering, reactive service bridge, and framework evaluation.

use actual_api::commands::{AddTransaction, CreateAccount, CreateCategory, CreateCategoryGroup};
use actual_api::BudgetService;
use actual_core::cents::Cents;
use actual_gui::bridge::GuiBridge;
use actual_gui::evaluation::{FrameworkComparison, FrameworkEvaluation};
use actual_gui::navigation::{ImeBuffer, KeyAction, KeyboardNavigator};
use actual_gui::table::{CellValue, ColumnAlign, EditableTable, TableCell, TableColumn};
use actual_gui::views::{BudgetView, TransactionView};
use chrono::NaiveDate;

#[test]
fn test_cell_value_formatting_and_parsing() {
    // Currency parsing
    let curr_cell = CellValue::Currency(Cents::from_cents(1550));
    assert_eq!(curr_cell.display_string(), "15.50");
    assert_eq!(
        curr_cell.parse_input("24.99").unwrap(),
        CellValue::Currency(Cents::from_cents(2499))
    );
    assert_eq!(
        curr_cell.parse_input("$100.00").unwrap(),
        CellValue::Currency(Cents::from_cents(10000))
    );
    assert_eq!(
        curr_cell.parse_input("(50.00)").unwrap(),
        CellValue::Currency(Cents::from_cents(-5000))
    );
    assert!(curr_cell.parse_input("abc").is_err());

    // Date parsing
    let date_cell = CellValue::Date("2024-05-15".into());
    assert_eq!(date_cell.display_string(), "2024-05-15");
    assert_eq!(
        date_cell.parse_input("2024-12-31").unwrap(),
        CellValue::Date("2024-12-31".into())
    );
    assert!(date_cell.parse_input("invalid-date").is_err());

    // Boolean parsing
    let bool_cell = CellValue::Boolean(true);
    assert_eq!(bool_cell.display_string(), "✓");
    assert_eq!(
        bool_cell.parse_input("false").unwrap(),
        CellValue::Boolean(false)
    );
    assert_eq!(
        bool_cell.parse_input("1").unwrap(),
        CellValue::Boolean(true)
    );
}

#[test]
fn test_editable_table_lifecycle_and_validation() {
    let columns = vec![
        TableColumn::new("name", "Category", 200),
        TableColumn::new("budgeted", "Budgeted", 120)
            .with_align(ColumnAlign::Right)
            .editable(true),
        TableColumn::new("activity", "Activity", 120).with_align(ColumnAlign::Right),
    ];

    let mut table = EditableTable::new(columns, 30, 300);

    let row1 = vec![
        TableCell::new(CellValue::Text("Groceries".into())),
        TableCell::new(CellValue::Currency(Cents::from_cents(50000))),
        TableCell::new(CellValue::Currency(Cents::from_cents(-12500))),
    ];
    let row2 = vec![
        TableCell::new(CellValue::Text("Utilities".into())),
        TableCell::new(CellValue::Currency(Cents::from_cents(20000))),
        TableCell::new(CellValue::Currency(Cents::from_cents(-8500))),
    ];

    table.add_row("cat-1", row1);
    table.add_row("cat-2", row2);
    assert_eq!(table.row_count(), 2);
    assert_eq!(table.col_count(), 3);

    // 1. Cannot edit read-only column 0 ("Category")
    table.set_selection(0, 0);
    assert!(table.start_editing_selected().is_err());

    // 2. Can edit column 1 ("Budgeted")
    table.set_selection(0, 1);
    assert!(table.start_editing_selected().is_ok());
    assert!(table.get_cell(0, 1).unwrap().is_editing());

    // 3. Commit with valid input
    table.get_cell_mut(0, 1).unwrap().edit_buffer = Some("750.00".into());
    let committed = table.commit_editing_selected().unwrap();
    assert_eq!(committed, CellValue::Currency(Cents::from_cents(75000)));
    assert!(!table.get_cell(0, 1).unwrap().is_editing());
    assert!(table.get_cell(0, 1).unwrap().is_dirty);

    // 4. Validation error keeps edit active and stores error
    table.set_selection(1, 1);
    table.start_editing_selected().unwrap();
    table.get_cell_mut(1, 1).unwrap().edit_buffer = Some("not-a-number".into());
    assert!(table.commit_editing_selected().is_err());
    assert!(table.get_cell(1, 1).unwrap().is_editing());
    assert!(table.get_cell(1, 1).unwrap().error.is_some());

    // 5. Cancel discards edit
    table.cancel_editing_selected();
    assert!(!table.get_cell(1, 1).unwrap().is_editing());
    assert_eq!(
        table.get_cell(1, 1).unwrap().value,
        CellValue::Currency(Cents::from_cents(20000))
    );
}

#[test]
fn test_table_virtual_scroll_under_high_row_count() {
    let columns = vec![TableColumn::new("id", "ID", 100)];
    let mut table = EditableTable::new(columns, 30, 300);

    // Add 10,000 rows
    for i in 0..10_000 {
        table.add_row(
            format!("row-{i}"),
            vec![TableCell::new(CellValue::Text(format!("Item {i}")))],
        );
    }

    assert_eq!(table.row_count(), 10_000);

    // Initially at top: viewport height 300, row height 30 -> 10 visible + 3 overscan
    let (start, end) = table.virtual_scroll.visible_range();
    assert_eq!(start, 0);
    assert!(end <= 15);

    // Scroll to middle (row 5000)
    table.set_selection(5000, 0);
    let (start_mid, end_mid) = table.virtual_scroll.visible_range();
    assert!(start_mid <= 5000);
    assert!(end_mid >= 5000);
    assert!(
        end_mid - start_mid <= 20,
        "Virtual window must render <= 20 rows"
    );
}

#[test]
fn test_keyboard_navigation_and_tab_cycling() {
    let columns = vec![
        TableColumn::new("c0", "Col 0", 100).editable(true),
        TableColumn::new("c1", "Col 1", 100).editable(true),
    ];
    let mut table = EditableTable::new(columns, 30, 300);

    table.add_row(
        "r0",
        vec![
            TableCell::new(CellValue::Text("0,0".into())),
            TableCell::new(CellValue::Text("0,1".into())),
        ],
    );
    table.add_row(
        "r1",
        vec![
            TableCell::new(CellValue::Text("1,0".into())),
            TableCell::new(CellValue::Text("1,1".into())),
        ],
    );

    table.set_selection(0, 0);

    // Arrow Down
    KeyboardNavigator::handle_action(&mut table, KeyAction::MoveDown).unwrap();
    assert_eq!(table.selection.row, 1);
    assert_eq!(table.selection.col, 0);

    // Arrow Right
    KeyboardNavigator::handle_action(&mut table, KeyAction::MoveRight).unwrap();
    assert_eq!(table.selection.row, 1);
    assert_eq!(table.selection.col, 1);

    // NextCell (Tab) wraps from (1,1) to (0,0)
    KeyboardNavigator::handle_action(&mut table, KeyAction::NextCell).unwrap();
    assert_eq!(table.selection.row, 0);
    assert_eq!(table.selection.col, 0);

    // NextCell (Tab) advances to (0,1)
    KeyboardNavigator::handle_action(&mut table, KeyAction::NextCell).unwrap();
    assert_eq!(table.selection.row, 0);
    assert_eq!(table.selection.col, 1);

    // PrevCell (Shift+Tab) moves back to (0,0)
    KeyboardNavigator::handle_action(&mut table, KeyAction::PrevCell).unwrap();
    assert_eq!(table.selection.row, 0);
    assert_eq!(table.selection.col, 0);

    // PrevCell from (0,0) wraps to last cell (1,1)
    KeyboardNavigator::handle_action(&mut table, KeyAction::PrevCell).unwrap();
    assert_eq!(table.selection.row, 1);
    assert_eq!(table.selection.col, 1);

    // StartEdit on (1,1)
    KeyboardNavigator::handle_action(&mut table, KeyAction::StartEdit).unwrap();
    assert!(table.get_cell(1, 1).unwrap().is_editing());

    // CommitEdit finishes edit
    KeyboardNavigator::handle_action(&mut table, KeyAction::CommitEdit).unwrap();
    assert!(!table.get_cell(1, 1).unwrap().is_editing());
}

#[test]
fn test_ime_buffer_composition_and_cjk_input() {
    let mut ime = ImeBuffer::new();
    assert!(!ime.is_composing);

    // 1. User types Japanese Hiragana composition "とうきょう"
    ime.update_preedit("とうきょう", 5);
    assert!(ime.is_composing);
    assert_eq!(ime.preedit, "とうきょう");
    assert_eq!(ime.cursor, 5);

    // 2. IME engine provides candidate list
    ime.set_candidates(vec!["東京".into(), "東経".into(), "とうきょう".into()]);
    assert_eq!(ime.selected_candidate, Some(0));

    // 3. User navigates candidates
    ime.next_candidate();
    assert_eq!(ime.selected_candidate, Some(1)); // "東経"
    ime.prev_candidate();
    assert_eq!(ime.selected_candidate, Some(0)); // "東京"

    // 4. Commit candidate
    let committed = ime.commit();
    assert_eq!(committed, "東京");
    assert!(!ime.is_composing);
    assert!(ime.preedit.is_empty());

    // 5. Cancel composition
    ime.update_preedit("nihon", 5);
    assert!(ime.is_composing);
    ime.reset();
    assert!(!ime.is_composing);
    assert!(ime.preedit.is_empty());
}

#[test]
fn test_gui_bridge_budget_view_and_undo_redo() {
    let mut service = BudgetService::open_in_memory().expect("open service");

    // Setup initial budget entities
    let group_id = service
        .create_category_group(CreateCategoryGroup {
            id: String::new(),
            name: "Living Expenses".into(),
            is_income: false,
        })
        .unwrap();
    let _cat_id = service
        .create_category(CreateCategory {
            id: String::new(),
            group_id,
            name: "Groceries".into(),
            is_income: false,
        })
        .unwrap();

    let mut bridge = GuiBridge::new(service);
    let mut view = BudgetView::new("2024-05", 400);

    // Initial refresh
    bridge.refresh_budget_view(&mut view).unwrap();
    assert_eq!(view.table.row_count(), 1);
    assert_eq!(
        view.table.get_cell(0, 0).unwrap().value,
        CellValue::Text("Groceries".into())
    );
    assert_eq!(
        view.table.get_cell(0, 1).unwrap().value,
        CellValue::Currency(Cents::ZERO)
    );

    // Edit Budgeted amount inline to $450.00
    view.table.set_selection(0, 1);
    view.table.start_editing_selected().unwrap();
    view.table.get_cell_mut(0, 1).unwrap().edit_buffer = Some("450.00".into());
    bridge.commit_budget_cell(&mut view, 0, 1).unwrap();

    // Verify view and summary updated
    assert_eq!(
        view.table.get_cell(0, 1).unwrap().value,
        CellValue::Currency(Cents::from_cents(45000))
    );
    assert_eq!(view.summary.total_budgeted, Cents::from_cents(45000));

    // Test Undo
    let undone = bridge.undo(&mut view).unwrap();
    assert!(undone);
    assert_eq!(
        view.table.get_cell(0, 1).unwrap().value,
        CellValue::Currency(Cents::ZERO)
    );
    assert_eq!(view.summary.total_budgeted, Cents::ZERO);

    // Test Redo
    let redone = bridge.redo(&mut view).unwrap();
    assert!(redone);
    assert_eq!(
        view.table.get_cell(0, 1).unwrap().value,
        CellValue::Currency(Cents::from_cents(45000))
    );
}

#[test]
fn test_gui_bridge_transaction_view_and_filtering() {
    let mut service = BudgetService::open_in_memory().expect("open service");

    let acc_id = service
        .create_account(CreateAccount {
            id: String::new(),
            name: "Primary Checking".into(),
            offbudget: false,
        })
        .unwrap();

    service
        .add_transaction(AddTransaction {
            id: String::new(),
            account_id: acc_id.clone(),
            date: NaiveDate::from_ymd_opt(2024, 5, 10).unwrap(),
            amount: Cents::from_cents(-4520),
            payee_id: None,
            category_id: None,
            notes: Some("Whole Foods Groceries".into()),
            cleared: true,
        })
        .unwrap();

    service
        .add_transaction(AddTransaction {
            id: String::new(),
            account_id: acc_id.clone(),
            date: NaiveDate::from_ymd_opt(2024, 5, 12).unwrap(),
            amount: Cents::from_cents(-1500),
            payee_id: None,
            category_id: None,
            notes: Some("Coffee Shop".into()),
            cleared: false,
        })
        .unwrap();

    let bridge = GuiBridge::new(service);
    let mut view = TransactionView::new(Some(acc_id), 400);

    // Unfiltered view
    bridge.refresh_transaction_view(&mut view).unwrap();
    assert_eq!(view.table.row_count(), 2);

    // Filter by "coffee"
    view.filter_query = "coffee".into();
    bridge.refresh_transaction_view(&mut view).unwrap();
    assert_eq!(view.table.row_count(), 1);
    assert_eq!(
        view.table.get_cell(0, 4).unwrap().value,
        CellValue::Text("Coffee Shop".into())
    );
}

#[test]
fn test_framework_evaluation_comparison_and_report() {
    let slint = FrameworkEvaluation::slint_profile();
    assert_eq!(slint.name, "Slint");
    assert!(slint.total_score() >= 80);

    let dioxus = FrameworkEvaluation::dioxus_profile();
    assert_eq!(dioxus.name, "Dioxus");
    assert!(dioxus.total_score() >= 80);

    let comparison = FrameworkComparison::evaluate();
    let report = comparison.to_markdown_report();

    assert!(report.contains("Slint vs Dioxus"));
    assert!(report.contains("Table Rendering & Virtualization"));
    assert!(report.contains("Keyboard Navigation & IME"));
    assert!(report.contains("Web (WASM + OPFS)"));
    assert!(report.contains("Total Weighted Score"));
}
