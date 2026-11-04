//! Offline queue management and resilient backoff reconnection state machine.

use crate::change_tracker::CrdtTracker;
use crate::client::{SyncClient, SyncCycleResult};
use crate::crdt::CrdtMessage;
use crate::error::SyncError;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::time::Duration;

/// Live connectivity and synchronization state.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SyncStatus {
    /// Connected and synchronized with server.
    #[default]
    Synced,
    /// Actively synchronizing with server.
    Syncing,
    /// Disconnected / offline mode, changes queued locally.
    Offline,
    /// Synchronization encountered an error.
    Error(String),
}

/// Exponential backoff scheduler for reconnect attempts.
#[derive(Debug, Clone)]
pub struct BackoffStrategy {
    /// Initial base retry delay in milliseconds (default: 1,000ms).
    pub initial_delay_ms: u64,
    /// Maximum backoff ceiling in milliseconds (default: 60,000ms / 1 min).
    pub max_delay_ms: u64,
    /// Exponential growth multiplier (default: 2.0).
    pub factor: f64,
    /// Number of consecutive failures encountered.
    pub attempt: u32,
}

impl Default for BackoffStrategy {
    fn default() -> Self {
        Self {
            initial_delay_ms: 1000,
            max_delay_ms: 60000,
            factor: 2.0,
            attempt: 0,
        }
    }
}

impl BackoffStrategy {
    /// Constructs a new backoff strategy.
    #[must_use]
    pub fn new(initial_delay_ms: u64, max_delay_ms: u64) -> Self {
        Self {
            initial_delay_ms,
            max_delay_ms,
            factor: 2.0,
            attempt: 0,
        }
    }

    /// Computes the next retry delay and increments the attempt counter.
    pub fn next_delay(&mut self) -> Duration {
        let exp = (self.factor).powi(self.attempt.min(10) as i32);
        let delay = ((self.initial_delay_ms as f64) * exp) as u64;
        let clamped = delay.min(self.max_delay_ms);

        self.attempt = self.attempt.saturating_add(1);
        Duration::from_millis(clamped)
    }

    /// Resets attempt count upon successful sync or reconnect.
    pub fn reset(&mut self) {
        self.attempt = 0;
    }
}

/// Offline queue buffering mutations during network disconnection.
#[derive(Debug, Clone, Default)]
pub struct OfflineQueue {
    queue: VecDeque<CrdtMessage>,
    status: SyncStatus,
    backoff: BackoffStrategy,
}

impl OfflineQueue {
    /// Creates a new, empty offline queue.
    #[must_use]
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            status: SyncStatus::Synced,
            backoff: BackoffStrategy::default(),
        }
    }

    /// Current synchronization status.
    #[must_use]
    pub fn status(&self) -> &SyncStatus {
        &self.status
    }

    /// Returns the number of pending unsynchronized messages in the offline queue.
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.queue.len()
    }

    /// Enqueues a local mutation to be transmitted when network connectivity is restored.
    pub fn enqueue(&mut self, message: CrdtMessage) {
        self.queue.push_back(message);
        if self.status == SyncStatus::Synced {
            self.status = SyncStatus::Offline;
        }
    }

    /// Drains all currently queued messages.
    pub fn drain_messages(&mut self) -> Vec<CrdtMessage> {
        self.queue.drain(..).collect()
    }

    /// Attempts to flush queued messages to the sync server.
    ///
    /// If successful, resets the backoff and marks the status as `Synced`.
    /// If network fails, advances backoff and sets status to `Offline` or `Error`.
    pub fn flush_queue(
        &mut self,
        client: &SyncClient,
        conn: &Connection,
        tracker: &mut CrdtTracker,
        last_synced_timestamp: Option<&str>,
    ) -> Result<SyncCycleResult, SyncError> {
        self.status = SyncStatus::Syncing;

        let outgoing = if self.queue.is_empty() {
            CrdtTracker::get_messages_since(conn, last_synced_timestamp)?
        } else {
            self.drain_messages()
        };

        let req = match client.build_sync_request(last_synced_timestamp, &outgoing) {
            Ok(r) => r,
            Err(e) => {
                self.status = SyncStatus::Error(e.to_string());
                return Err(e);
            }
        };

        let response = match client.send_request(&req) {
            Ok(resp) => resp,
            Err(e) => {
                // Restore outgoing messages into queue so they are not lost
                for msg in outgoing {
                    self.queue.push_front(msg);
                }
                let _next_delay = self.backoff.next_delay();
                self.status = SyncStatus::Offline;
                return Err(e);
            }
        };

        let cycle_res = client.process_sync_response(conn, tracker, &response, outgoing.len())?;

        self.backoff.reset();
        self.status = SyncStatus::Synced;

        Ok(cycle_res)
    }

    /// Explicitly transitions the queue into offline state.
    pub fn set_offline(&mut self) {
        self.status = SyncStatus::Offline;
    }
}
