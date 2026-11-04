//! Desktop application state controller, menu dispatcher, and lifecycle coordinator.

use crate::bridge::GuiBridge;
use crate::desktop::menu::{MenuAction, MenuBar};
use crate::desktop::sync_worker::{BackgroundSyncWorker, SyncConfig};
use crate::desktop::theme::{detect_system_dark_mode, ColorPalette, ThemeMode};
use crate::desktop::window::WindowConfig;
use crate::error::GuiError;
use crate::views::{BudgetView, TransactionView};
use actual_api::BudgetService;
use actual_sync::CrdtTracker;
use std::path::{Path, PathBuf};

/// Active view tab in the desktop application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActiveTab {
    #[default]
    Budget,
    Transactions,
    Reports,
    Settings,
}

/// Central desktop application controller.
pub struct DesktopApp {
    /// Active budget database path on disk (or in-memory).
    pub budget_path: Option<PathBuf>,
    /// Reactive domain service bridge.
    pub bridge: GuiBridge,
    /// Monthly envelope budget view component.
    pub budget_view: BudgetView,
    /// Transaction ledger view component.
    pub tx_view: TransactionView,
    /// Window layout and position configuration.
    pub window_config: WindowConfig,
    /// Desktop menu bar.
    pub menu_bar: MenuBar,
    /// Theme preference mode.
    pub theme_mode: ThemeMode,
    /// Active color palette.
    pub palette: ColorPalette,
    /// Active UI tab.
    pub active_tab: ActiveTab,
    /// Background synchronization worker.
    pub sync_worker: Option<BackgroundSyncWorker>,
    /// CRDT mutation tracker.
    pub tracker: CrdtTracker,
}

impl DesktopApp {
    /// Creates a new in-memory desktop application instance.
    pub fn open_in_memory() -> Result<Self, GuiError> {
        let service =
            BudgetService::open_in_memory().map_err(|e| GuiError::Service(e.to_string()))?;
        let tracker = CrdtTracker::load_from_sqlite(service.store().conn()).unwrap_or_default();
        let bridge = GuiBridge::new(service);

        let now = chrono::Local::now();
        let month_str = now.format("%Y-%m").to_string();

        let mut app = Self {
            budget_path: None,
            bridge,
            budget_view: BudgetView::new(month_str, 600),
            tx_view: TransactionView::new(None, 600),
            window_config: WindowConfig::default(),
            menu_bar: MenuBar::standard_menu(),
            theme_mode: ThemeMode::System,
            palette: ColorPalette::resolve(ThemeMode::System, detect_system_dark_mode()),
            active_tab: ActiveTab::Budget,
            sync_worker: None,
            tracker,
        };

        app.bridge.refresh_budget_view(&mut app.budget_view)?;
        app.bridge.refresh_transaction_view(&mut app.tx_view)?;

        Ok(app)
    }

    /// Creates a desktop application instance backed by an on-disk SQLite database file.
    pub fn open_file(path: impl AsRef<Path>) -> Result<Self, GuiError> {
        let path = path.as_ref().to_path_buf();
        let service = BudgetService::open(&path).map_err(|e| GuiError::Service(e.to_string()))?;
        let tracker = CrdtTracker::load_from_sqlite(service.store().conn()).unwrap_or_default();
        let bridge = GuiBridge::new(service);

        let now = chrono::Local::now();
        let month_str = now.format("%Y-%m").to_string();

        let mut app = Self {
            budget_path: Some(path),
            bridge,
            budget_view: BudgetView::new(month_str, 600),
            tx_view: TransactionView::new(None, 600),
            window_config: WindowConfig::default(),
            menu_bar: MenuBar::standard_menu(),
            theme_mode: ThemeMode::System,
            palette: ColorPalette::resolve(ThemeMode::System, detect_system_dark_mode()),
            active_tab: ActiveTab::Budget,
            sync_worker: None,
            tracker,
        };

        app.bridge.refresh_budget_view(&mut app.budget_view)?;
        app.bridge.refresh_transaction_view(&mut app.tx_view)?;

        Ok(app)
    }

    /// Configures remote sync connection.
    pub fn configure_sync(&mut self, config: SyncConfig) {
        self.sync_worker = Some(BackgroundSyncWorker::new(config));
    }

    /// Executes an on-demand sync cycle against remote server.
    pub fn sync_now(&mut self) -> Result<(), GuiError> {
        if let Some(worker) = &self.sync_worker {
            let conn = self.bridge.service.store().conn();
            worker
                .execute_sync_cycle(conn, &mut self.tracker)
                .map_err(GuiError::Service)?;

            self.bridge.refresh_budget_view(&mut self.budget_view)?;
            self.bridge.refresh_transaction_view(&mut self.tx_view)?;
        }
        Ok(())
    }

    /// Dispatches a logical menu action.
    pub fn dispatch_action(&mut self, action: MenuAction) -> Result<bool, GuiError> {
        match action {
            MenuAction::Undo => {
                self.bridge.undo(&mut self.budget_view)?;
                self.bridge.refresh_transaction_view(&mut self.tx_view)?;
                Ok(true)
            }
            MenuAction::Redo => {
                self.bridge.redo(&mut self.budget_view)?;
                self.bridge.refresh_transaction_view(&mut self.tx_view)?;
                Ok(true)
            }
            MenuAction::ToggleTheme => {
                self.theme_mode = match self.theme_mode {
                    ThemeMode::Dark => ThemeMode::Light,
                    ThemeMode::Light => ThemeMode::System,
                    ThemeMode::System => ThemeMode::Dark,
                };
                self.palette = ColorPalette::resolve(self.theme_mode, detect_system_dark_mode());
                Ok(true)
            }
            MenuAction::ToggleFullscreen => {
                self.window_config.is_fullscreen = !self.window_config.is_fullscreen;
                Ok(true)
            }
            MenuAction::SyncNow => {
                self.sync_now()?;
                Ok(true)
            }
            MenuAction::FindTransaction => {
                self.active_tab = ActiveTab::Transactions;
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}
