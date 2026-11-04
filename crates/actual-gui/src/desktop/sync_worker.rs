//! Background synchronization coordinator for the desktop application.

use actual_sync::queue::SyncStatus;
use actual_sync::{CrdtTracker, SyncClient};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Configuration for background synchronization in the desktop client.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Remote server URL (e.g. "http://localhost:5006" or "https://sync.actualbudget.com").
    pub server_url: String,
    /// Target budget file UUID.
    pub file_id: String,
    /// Sync group UUID.
    pub group_id: String,
    /// Periodic automatic sync interval in seconds (0 = disabled).
    pub auto_sync_interval_secs: u64,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            server_url: "http://localhost:5006".into(),
            file_id: String::new(),
            group_id: String::new(),
            auto_sync_interval_secs: 30,
        }
    }
}

/// Statistics and status of the background sync worker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncState {
    /// Current network sync status.
    pub status: SyncStatus,
    /// Last successful sync ISO timestamp string.
    pub last_synced_at: Option<String>,
    /// Total changes uploaded in last cycle.
    pub last_uploaded_count: usize,
    /// Total changes downloaded in last cycle.
    pub last_downloaded_count: usize,
    /// Consecutive sync failures.
    pub failure_count: usize,
}

impl Default for SyncState {
    fn default() -> Self {
        Self {
            status: SyncStatus::Synced,
            last_synced_at: None,
            last_uploaded_count: 0,
            last_downloaded_count: 0,
            failure_count: 0,
        }
    }
}

/// Background worker managing synchronization cycles against an Actual sync server.
pub struct BackgroundSyncWorker {
    pub config: SyncConfig,
    pub state: Arc<Mutex<SyncState>>,
}

impl BackgroundSyncWorker {
    pub fn new(config: SyncConfig) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(SyncState::default())),
        }
    }

    /// Performs a synchronous execution of one sync cycle.
    pub fn execute_sync_cycle(
        &self,
        conn: &Connection,
        tracker: &mut CrdtTracker,
    ) -> Result<SyncStatus, String> {
        {
            let mut st = self.state.lock().map_err(|e| e.to_string())?;
            st.status = SyncStatus::Syncing;
        }

        let client = SyncClient::new(
            &self.config.server_url,
            &self.config.file_id,
            &self.config.group_id,
        );

        match client.sync_with_server(conn, tracker, None) {
            Ok(cycle_res) => {
                let mut st = self.state.lock().map_err(|e| e.to_string())?;
                st.status = SyncStatus::Synced;
                st.last_synced_at = Some(chrono::Utc::now().to_rfc3339());
                st.last_uploaded_count = cycle_res.uploaded_count;
                st.last_downloaded_count = cycle_res.downloaded_count;
                st.failure_count = 0;
                Ok(SyncStatus::Synced)
            }
            Err(e) => {
                let err_msg = e.to_string();
                let mut st = self.state.lock().map_err(|e| e.to_string())?;
                st.failure_count += 1;
                st.status = SyncStatus::Error(err_msg.clone());
                Err(err_msg)
            }
        }
    }

    /// Gets a snapshot of the current sync state.
    pub fn current_state(&self) -> SyncState {
        self.state.lock().map(|s| s.clone()).unwrap_or_default()
    }

    /// Interval duration for auto-sync.
    #[must_use]
    pub fn interval(&self) -> Duration {
        Duration::from_secs(self.config.auto_sync_interval_secs)
    }
}
