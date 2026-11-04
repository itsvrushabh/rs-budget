//! Synchronization endpoint processing protobuf CRDT messages and Merkle range diffs.

use crate::error::ServerError;
use crate::state::ServerState;
use actual_sync::proto::{MessageEnvelope, SyncRequest, SyncResponse};
use actual_sync::timestamp::Timestamp;
use actual_sync::TrieNode;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::header::CONTENT_TYPE;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use prost::Message as ProstMessage;
use rusqlite::{params, OptionalExtension};
use std::collections::HashSet;
use std::str::FromStr;

/// Handler for `POST /sync/sync` and `POST /sync`.
pub async fn sync_handler(
    State(state): State<ServerState>,
    _headers: HeaderMap,
    body: Bytes,
) -> Result<Response, ServerError> {
    let req = SyncRequest::decode(body.as_ref())?;

    let file_id = req.file_id;
    let group_id = req.group_id;
    let since = req.since;

    let (response_messages, server_trie_json) = {
        let conn = state
            .db
            .lock()
            .map_err(|e| ServerError::Internal(e.to_string()))?;

        // 1. Load existing server Merkle trie
        let trie_json_opt: Option<String> = conn
            .query_row(
                "SELECT trie_json FROM merkle_tries WHERE file_id = ?1 AND group_id = ?2",
                params![file_id, group_id],
                |row| row.get(0),
            )
            .optional()?;

        let mut server_trie: TrieNode = match trie_json_opt {
            Some(json_str) => serde_json::from_str(&json_str).unwrap_or_else(|_| TrieNode::empty()),
            None => TrieNode::empty(),
        };

        // 2. Insert incoming client messages
        let mut client_uploaded_timestamps = HashSet::new();

        for env in req.messages {
            client_uploaded_timestamps.insert(env.timestamp.clone());

            let is_enc_int = if env.is_encrypted { 1 } else { 0 };

            conn.execute(
                r#"
                INSERT OR IGNORE INTO messages (file_id, group_id, timestamp, is_encrypted, content)
                VALUES (?1, ?2, ?3, ?4, ?5)
                "#,
                params![file_id, group_id, env.timestamp, is_enc_int, env.content,],
            )?;

            if let Ok(ts) = Timestamp::from_str(&env.timestamp) {
                server_trie.insert(&ts);
            }
        }

        // 3. Query messages on server that client does not have (timestamp > since and not in uploaded batch)
        let mapper = |row: &rusqlite::Row| -> Result<(String, bool, Vec<u8>), rusqlite::Error> {
            let ts: String = row.get(0)?;
            let is_enc: i64 = row.get(1)?;
            let content: Vec<u8> = row.get(2)?;
            Ok((ts, is_enc != 0, content))
        };

        let mut out_messages = Vec::new();

        if since.is_empty() {
            let mut stmt = conn.prepare(
                "SELECT timestamp, is_encrypted, content FROM messages WHERE file_id = ?1 AND group_id = ?2 ORDER BY timestamp ASC",
            )?;
            let rows = stmt.query_map(params![file_id, group_id], mapper)?;
            for r in rows {
                let (ts, is_enc, content) = r?;
                if !client_uploaded_timestamps.contains(&ts) {
                    out_messages.push(MessageEnvelope {
                        timestamp: ts,
                        is_encrypted: is_enc,
                        content,
                    });
                }
            }
        } else {
            let mut stmt = conn.prepare(
                "SELECT timestamp, is_encrypted, content FROM messages WHERE file_id = ?1 AND group_id = ?2 AND timestamp > ?3 ORDER BY timestamp ASC",
            )?;
            let rows = stmt.query_map(params![file_id, group_id, since], mapper)?;
            for r in rows {
                let (ts, is_enc, content) = r?;
                if !client_uploaded_timestamps.contains(&ts) {
                    out_messages.push(MessageEnvelope {
                        timestamp: ts,
                        is_encrypted: is_enc,
                        content,
                    });
                }
            }
        }

        // 4. Update server Merkle trie in database
        let trie_json = serde_json::to_string(&server_trie)?;
        conn.execute(
            r#"
            INSERT INTO merkle_tries (file_id, group_id, trie_json)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(file_id, group_id) DO UPDATE SET trie_json = ?3
            "#,
            params![file_id, group_id, trie_json],
        )?;

        (out_messages, trie_json)
    };

    // 5. Encode protobuf SyncResponse
    let resp = SyncResponse {
        messages: response_messages,
        merkle: server_trie_json,
    };

    let mut resp_bytes = Vec::with_capacity(resp.encoded_len());
    resp.encode(&mut resp_bytes)?;

    let mut response = resp_bytes.into_response();
    response
        .headers_mut()
        .insert(CONTENT_TYPE, "application/actual-sync".parse().unwrap());

    Ok(response)
}
