//! 100% Rust cross-platform Graphical User Interface (GUI) architecture,
//! editable financial tables, keyboard navigation, IME buffering, reactive service bridge,
//! and UI framework evaluation for rs-budget.

#[cfg(not(target_arch = "wasm32"))]
pub mod bridge;
#[cfg(not(target_arch = "wasm32"))]
pub mod desktop;
pub mod error;
pub mod evaluation;
pub mod navigation;
pub mod table;
pub mod views;
pub mod web;

#[cfg(not(target_arch = "wasm32"))]
pub use bridge::GuiBridge;
#[cfg(not(target_arch = "wasm32"))]
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
pub use web::{
    IndexHtmlBuilder, OnlineState, OpfsError, OpfsStorageBackend, SqliteLockLevel, SqliteOpenFlags,
    SqliteOpfsVfs, WebApp, WebBudgetMetadata, WebManifest, WebRoute, WebSyncBridge, WebSyncConfig,
    WebSyncState,
};
