//! 100% Rust browser WebAssembly (WASM) client architecture with Origin Private File System (OPFS).
//!
//! Provides in-browser SQLite persistence via OPFS, web sync bridge, offline queuing,
//! Progressive Web App (PWA) manifest generation, and service worker offline caching.

pub mod app;
pub mod opfs;
pub mod pwa;
pub mod sync_bridge;
pub mod vfs;

pub use app::{WebApp, WebBudgetMetadata, WebRoute};
pub use opfs::{OpfsError, OpfsFileHandle, OpfsFileMetadata, OpfsStorageBackend, StorageQuota};
pub use pwa::{IndexHtmlBuilder, ManifestIcon, ServiceWorkerBuilder, WebManifest};
pub use sync_bridge::{OnlineState, QueuedWebMessage, WebSyncBridge, WebSyncConfig, WebSyncState};
pub use vfs::{SqliteLockLevel, SqliteOpenFlags, SqliteOpfsVfs};
