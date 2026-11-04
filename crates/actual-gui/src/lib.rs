//! 100% Rust cross-platform Graphical User Interface (GUI) architecture,
//! editable financial tables, keyboard navigation, IME buffering, reactive service bridge,
//! and UI framework evaluation for rs-budget.

pub mod bridge;
pub mod desktop;
pub mod error;
pub mod evaluation;
pub mod navigation;
pub mod table;
pub mod views;

pub use bridge::GuiBridge;
pub use desktop::{
    ActiveTab, BackgroundSyncWorker, ColorPalette, DesktopApp, MenuAction, MenuBar, ThemeMode,
    WindowConfig,
};
pub use error::GuiError;
pub use evaluation::{
    EvaluationCriterion, FrameworkComparison, FrameworkEvaluation, PlatformTarget,
};
pub use navigation::{ImeBuffer, KeyAction, KeyboardNavigator};
pub use table::{
    CellValue, ColumnAlign, EditableTable, SelectionModel, TableCell, TableColumn, VirtualScroll,
};
pub use views::{BudgetView, SummaryBar, TransactionView};
