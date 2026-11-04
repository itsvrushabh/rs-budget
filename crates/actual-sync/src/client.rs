//! Client-side synchronization protocol engine and wire request coordinator.

use crate::change_tracker::CrdtTracker;
use crate::crdt::CrdtMessage;
use crate::error::SyncError;
use crate::merkle::{diff as diff_merkle, TrieNode};
use crate::proto::{SyncRequest, SyncResponse};
use actual_crypto::Key;
use prost::Message as ProstMessage;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

/// Summary of a completed synchronization cycle.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncCycleResult {
    /// Number of local changes uploaded to server.
    pub uploaded_count: usize,
    /// Number of remote changes downloaded from server.
    pub downloaded_count: usize,
    /// Number of downloaded changes applied locally by LWW.
    pub applied_count: usize,
    /// Number of downloaded changes ignored (local edit was newer).
    pub ignored_count: usize,
    /// Timestamp where local and server states diverged (if any).
    pub divergent_timestamp: Option<u64>,
}

/// Sync client coordinating communication with an Actual Budget sync server.
#[derive(Debug, Clone)]
pub struct SyncClient {
    /// Base sync server URL (e.g. "https://sync.actualbudget.com" or "http://localhost:5006").
    pub server_url: String,
    /// Budget file identifier UUID.
    pub file_id: String,
    /// Synchronization group UUID.
    pub group_id: String,
    /// Active encryption key ID if budget is encrypted.
    pub key_id: Option<String>,
    /// Master encryption key for envelope encryption/decryption.
    pub encryption_key: Option<Key>,
}

impl SyncClient {
    /// Creates a new sync client for an unencrypted or encrypted budget.
    #[must_use]
    pub fn new(
        server_url: impl Into<String>,
        file_id: impl Into<String>,
        group_id: impl Into<String>,
    ) -> Self {
        Self {
            server_url: server_url.into().trim_end_matches('/').to_string(),
            file_id: file_id.into(),
            group_id: group_id.into(),
            key_id: None,
            encryption_key: None,
        }
    }

    /// Configures master encryption key and key ID.
    pub fn with_encryption(mut self, key_id: impl Into<String>, key: Key) -> Self {
        self.key_id = Some(key_id.into());
        self.encryption_key = Some(key);
        self
    }

    /// Constructs an encrypted/plain `SyncRequest` protobuf structure.
    pub fn build_sync_request(
        &self,
        since_timestamp: Option<&str>,
        outgoing_messages: &[CrdtMessage],
    ) -> Result<SyncRequest, SyncError> {
        let mut envelopes = Vec::with_capacity(outgoing_messages.len());
        for msg in outgoing_messages {
            let env = msg.to_envelope(self.encryption_key.as_ref())?;
            envelopes.push(env);
        }

        Ok(SyncRequest {
            messages: envelopes,
            file_id: self.file_id.clone(),
            group_id: self.group_id.clone(),
            key_id: self.key_id.clone().unwrap_or_default(),
            since: since_timestamp.unwrap_or_default().to_string(),
        })
    }

    /// Parses and decrypts a `SyncResponse` protobuf structure.
    pub fn parse_sync_response(
        &self,
        response: &SyncResponse,
    ) -> Result<(TrieNode, Vec<CrdtMessage>), SyncError> {
        let server_trie: TrieNode = if response.merkle.is_empty() {
            TrieNode::empty()
        } else {
            serde_json::from_str(&response.merkle)?
        };

        let mut messages = Vec::with_capacity(response.messages.len());
        for env in &response.messages {
            let msg = CrdtMessage::from_envelope(env, self.encryption_key.as_ref())?;
            messages.push(msg);
        }

        Ok((server_trie, messages))
    }

    /// Transmits a `SyncRequest` over HTTP to the sync server endpoint.
    pub fn send_request(&self, request: &SyncRequest) -> Result<SyncResponse, SyncError> {
        let endpoint = format!("{}/sync/sync", self.server_url);

        let mut req_bytes = Vec::with_capacity(request.encoded_len());
        request.encode(&mut req_bytes)?;

        let response = ureq::post(&endpoint)
            .set("Content-Type", "application/actual-sync")
            .set("X-Actual-Sync-Version", "1")
            .send_bytes(&req_bytes)
            .map_err(|e| SyncError::Network(format!("Sync request failed: {e}")))?;

        let mut resp_bytes = Vec::new();
        response
            .into_reader()
            .read_to_end(&mut resp_bytes)
            .map_err(|e| SyncError::Network(format!("Failed to read sync response: {e}")))?;

        let sync_resp = SyncResponse::decode(resp_bytes.as_slice())?;
        Ok(sync_resp)
    }

    /// Performs a full end-to-end sync cycle against a remote server or mock transport.
    pub fn sync_with_server(
        &self,
        conn: &Connection,
        tracker: &mut CrdtTracker,
        last_synced_timestamp: Option<&str>,
    ) -> Result<SyncCycleResult, SyncError> {
        // 1. Gather outgoing changes
        let outgoing = CrdtTracker::get_messages_since(conn, last_synced_timestamp)?;
        let uploaded_count = outgoing.len();

        // 2. Build and transmit sync request
        let request = self.build_sync_request(last_synced_timestamp, &outgoing)?;
        let response = self.send_request(&request)?;

        // 3. Process response
        self.process_sync_response(conn, tracker, &response, uploaded_count)
    }

    /// Processes a `SyncResponse`, compares Merkle trees, and applies incoming delta messages.
    pub fn process_sync_response(
        &self,
        conn: &Connection,
        tracker: &mut CrdtTracker,
        response: &SyncResponse,
        uploaded_count: usize,
    ) -> Result<SyncCycleResult, SyncError> {
        let (server_trie, incoming_messages) = self.parse_sync_response(response)?;

        // Calculate diff between local trie and server trie
        let divergent_timestamp = diff_merkle(&tracker.merkle, &server_trie);

        // Apply incoming messages via Last-Write-Wins
        let apply_res = tracker.apply_remote_messages(conn, &incoming_messages)?;

        Ok(SyncCycleResult {
            uploaded_count,
            downloaded_count: incoming_messages.len(),
            applied_count: apply_res.applied_count,
            ignored_count: apply_res.ignored_count,
            divergent_timestamp,
        })
    }
}
