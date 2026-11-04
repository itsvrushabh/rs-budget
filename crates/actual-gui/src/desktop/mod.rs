//! Desktop client windowing, menu dispatch, native theme detection, and background sync.

pub mod app;
pub mod menu;
pub mod packaging;
pub mod sync_worker;
pub mod theme;
pub mod window;

pub use app::{ActiveTab, DesktopApp};
pub use menu::{MenuAction, MenuBar, MenuItem, Shortcut, ShortcutModifier};
pub use packaging::{LinuxPackaging, MacOsPackaging, OsTarget, WindowsPackaging};
pub use sync_worker::{BackgroundSyncWorker, SyncConfig, SyncState};
pub use theme::{detect_system_dark_mode, ColorPalette, ThemeMode};
pub use window::{WindowConfig, WindowDimensions};
