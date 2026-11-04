//! CRDT change tracking, SQLite message logging, and Last-Write-Wins (LWW) conflict resolution.

use crate::crdt::CrdtMessage;
use crate::error::SyncError;
use crate::merkle::TrieNode;
use crate::timestamp::{make_client_id, HlcClock, Timestamp, DEFAULT_MAX_DRIFT};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Result summary of applying remote CRDT messages to local database.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyResult {
    /// Number of remote changes that won LWW evaluation and were written to storage.
    pub applied_count: usize,
    /// Number of remote changes that lost LWW evaluation against newer local edits.
    pub ignored_count: usize,
    /// Earliest remote timestamp applied during this batch.
    pub earliest_timestamp: Option<Timestamp>,
    /// Latest remote timestamp applied during this batch.
    pub latest_timestamp: Option<Timestamp>,
}

/// Coordinates local change generation, SQLite CRDT logging, Merkle tree maintenance,
/// and deterministic conflict resolution.
#[derive(Debug, Clone)]
pub struct CrdtTracker {
    /// Hybrid Logical Clock for minting and advancing timestamps.
    pub clock: HlcClock,
    /// Trinary radix Merkle trie reflecting all known mutations.
    pub merkle: TrieNode,
}

impl CrdtTracker {
    /// Initializes a new tracker with a generated client identifier and empty Merkle trie.
    #[must_use]
    pub fn new() -> Self {
        let client_id = make_client_id();
        Self {
            clock: HlcClock::with_client_id(&client_id),
            merkle: TrieNode::empty(),
        }
    }

    /// Initializes a tracker with a specific client ID.
    #[must_use]
    pub fn with_client_id(client_id: &str) -> Self {
        Self {
            clock: HlcClock::with_client_id(client_id),
            merkle: TrieNode::empty(),
        }
    }

    /// Loads the tracker state (HLC clock and Merkle trie) from existing SQLite tables.
    pub fn load_from_sqlite(conn: &Connection) -> Result<Self, SyncError> {
        // 1. Load clock if present
        let clock_str: Option<String> = conn
            .query_row("SELECT clock FROM messages_clock WHERE id = 1", [], |row| {
                row.get(0)
            })
            .optional()?;

        let mut tracker = if let Some(cs) = clock_str {
            let ts = Timestamp::from_str(&cs)?;
            Self {
                clock: HlcClock::new(ts, DEFAULT_MAX_DRIFT),
                merkle: TrieNode::empty(),
            }
        } else {
            Self::new()
        };

        // 2. Rebuild Merkle trie from all recorded timestamps in messages_crdt
        let mut stmt =
            conn.prepare("SELECT timestamp FROM messages_crdt ORDER BY timestamp ASC")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;

        for r in rows {
            let ts_str = r?;
            if let Ok(ts) = Timestamp::from_str(&ts_str) {
                tracker.merkle.insert(&ts);
            }
        }

        Ok(tracker)
    }

    /// Records a new local dataset mutation, mints an HLC timestamp, and updates storage.
    pub fn send_change(
        &mut self,
        conn: &Connection,
        dataset: &str,
        row: &str,
        column: &str,
        value: &str,
    ) -> Result<CrdtMessage, SyncError> {
        let now = chrono::Utc::now().timestamp_millis().max(0) as u64;
        let timestamp = self.clock.send(now)?;

        // Apply mutation to target table
        Self::apply_field_to_table(conn, dataset, row, column, value)?;

        // Write to messages_crdt
        conn.execute(
            "INSERT INTO messages_crdt (timestamp, dataset, row, column, value) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                timestamp.to_string(),
                dataset,
                row,
                column,
                value.as_bytes(),
            ],
        )?;

        // Persist clock
        conn.execute(
            "INSERT INTO messages_clock (id, clock) VALUES (1, ?1) ON CONFLICT(id) DO UPDATE SET clock = ?1",
            params![timestamp.to_string()],
        )?;

        // Update local Merkle trie
        self.merkle.insert(&timestamp);

        Ok(CrdtMessage::new(timestamp, dataset, row, column, value))
    }

    /// Retrieves local CRDT messages recorded since a given timestamp string (or all if None).
    pub fn get_messages_since(
        conn: &Connection,
        since_timestamp: Option<&str>,
    ) -> Result<Vec<CrdtMessage>, SyncError> {
        let sql = match since_timestamp {
            Some(_) => {
                "SELECT timestamp, dataset, row, column, value FROM messages_crdt WHERE timestamp > ?1 ORDER BY timestamp ASC"
            }
            None => {
                "SELECT timestamp, dataset, row, column, value FROM messages_crdt ORDER BY timestamp ASC"
            }
        };

        let mut stmt = conn.prepare(sql)?;
        let mut rows = if let Some(since) = since_timestamp {
            stmt.query(params![since])?
        } else {
            stmt.query([])?
        };

        let mut messages = Vec::new();
        while let Some(r) = rows.next()? {
            let ts_str: String = r.get(0)?;
            let dataset: String = r.get(1)?;
            let row: String = r.get(2)?;
            let column: String = r.get(3)?;
            let val_bytes: Vec<u8> = r.get(4)?;
            let value = String::from_utf8_lossy(&val_bytes).to_string();

            if let Ok(ts) = Timestamp::from_str(&ts_str) {
                messages.push(CrdtMessage::new(ts, dataset, row, column, value));
            }
        }

        Ok(messages)
    }

    /// Evaluates and applies remote CRDT messages using Last-Write-Wins (LWW) conflict resolution.
    ///
    /// For each incoming message:
    /// 1. Finds the current latest timestamp for `(dataset, row, column)` in `messages_crdt`.
    /// 2. If the remote timestamp is strictly newer (or no local entry exists), applies the update
    ///    to the target dataset table, logs to `messages_crdt`, inserts into Merkle trie, and advances clock.
    /// 3. If local timestamp is newer or equal, the remote change is ignored.
    pub fn apply_remote_messages(
        &mut self,
        conn: &Connection,
        remote_messages: &[CrdtMessage],
    ) -> Result<ApplyResult, SyncError> {
        let mut result = ApplyResult::default();

        for msg in remote_messages {
            // 1. Query current winner timestamp for this field
            let local_ts_str: Option<String> = conn
                .query_row(
                    "SELECT timestamp FROM messages_crdt WHERE dataset = ?1 AND row = ?2 AND column = ?3 ORDER BY timestamp DESC LIMIT 1",
                    params![msg.dataset, msg.row, msg.column],
                    |row| row.get(0),
                )
                .optional()?;

            let remote_wins = match local_ts_str {
                Some(local_str) => match Timestamp::from_str(&local_str) {
                    Ok(local_ts) => msg.timestamp > local_ts,
                    Err(_) => true,
                },
                None => true,
            };

            if remote_wins {
                // Apply update to underlying dataset table
                Self::apply_field_to_table(conn, &msg.dataset, &msg.row, &msg.column, &msg.value)?;

                // Record in messages_crdt
                conn.execute(
                    "INSERT OR REPLACE INTO messages_crdt (timestamp, dataset, row, column, value) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        msg.timestamp.to_string(),
                        msg.dataset,
                        msg.row,
                        msg.column,
                        msg.value.as_bytes(),
                    ],
                )?;

                // Advance Merkle trie and HLC clock
                self.merkle.insert(&msg.timestamp);
                let now = chrono::Utc::now().timestamp_millis().max(0) as u64;
                let _ = self.clock.recv(&msg.timestamp, now);

                // Update result stats
                result.applied_count += 1;
                result.earliest_timestamp = Some(match result.earliest_timestamp {
                    Some(prev) => prev.min(msg.timestamp.clone()),
                    None => msg.timestamp.clone(),
                });
                result.latest_timestamp = Some(match result.latest_timestamp {
                    Some(prev) => prev.max(msg.timestamp.clone()),
                    None => msg.timestamp.clone(),
                });
            } else {
                result.ignored_count += 1;
            }
        }

        // Persist latest clock if any messages were applied
        if result.applied_count > 0 {
            let clock_ts = self.clock.timestamp();
            conn.execute(
                "INSERT INTO messages_clock (id, clock) VALUES (1, ?1) ON CONFLICT(id) DO UPDATE SET clock = ?1",
                params![clock_ts.to_string()],
            )?;
        }

        Ok(result)
    }

    /// Dynamically applies a single column value update or insert to a destination SQLite table.
    fn apply_field_to_table(
        conn: &Connection,
        table: &str,
        row_id: &str,
        column: &str,
        value: &str,
    ) -> Result<(), SyncError> {
        // Sanitize table and column identifiers to avoid SQL injection
        if !is_valid_identifier(table) || !is_valid_identifier(column) {
            return Err(SyncError::Conflict(format!(
                "Invalid SQL identifier in CRDT message: table '{table}', col '{column}'"
            )));
        }

        // Check if row already exists
        let exists: bool = conn
            .query_row(
                &format!("SELECT 1 FROM \"{table}\" WHERE id = ?1"),
                params![row_id],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);

        if exists {
            let sql = format!("UPDATE \"{table}\" SET \"{column}\" = ?1 WHERE id = ?2");
            conn.execute(&sql, params![value, row_id])?;
        } else {
            let sql = format!("INSERT INTO \"{table}\" (id, \"{column}\") VALUES (?1, ?2)");
            conn.execute(&sql, params![row_id, value])?;
        }

        Ok(())
    }
}

impl Default for CrdtTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Ensures an SQL identifier consists strictly of alphanumeric and underscore characters.
fn is_valid_identifier(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}
