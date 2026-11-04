//! Integration tests for client-side CRDT synchronization state machine,
//! change tracking, Last-Write-Wins (LWW) conflict resolution, and offline queue.

use actual_crypto::Key;
use actual_sync::crdt::CrdtMessage;
use actual_sync::proto::{SyncRequest, SyncResponse};
use actual_sync::queue::{BackoffStrategy, OfflineQueue, SyncStatus};
use actual_sync::timestamp::Timestamp;
use actual_sync::{CrdtTracker, SyncClient, TrieNode};
use prost::Message as ProstMessage;
use rusqlite::Connection;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

fn setup_test_db() -> Connection {
    let conn = Connection::open_in_memory().expect("open in-memory db");
    conn.execute_batch(
        r#"
        CREATE TABLE categories (
            id TEXT PRIMARY KEY,
            name TEXT,
            is_income INTEGER DEFAULT 0,
            cat_group TEXT,
            sort_order REAL,
            tombstone INTEGER DEFAULT 0
        );

        CREATE TABLE accounts (
            id TEXT PRIMARY KEY,
            name TEXT,
            offbudget INTEGER DEFAULT 0,
            closed INTEGER DEFAULT 0,
            tombstone INTEGER DEFAULT 0
        );

        CREATE TABLE messages_crdt (
            id INTEGER PRIMARY KEY,
            timestamp TEXT NOT NULL UNIQUE,
            dataset TEXT NOT NULL,
            row TEXT NOT NULL,
            column TEXT NOT NULL,
            value BLOB NOT NULL
        );

        CREATE TABLE messages_clock (
            id INTEGER PRIMARY KEY,
            clock TEXT
        );
        "#,
    )
    .expect("setup schema");
    conn
}

#[test]
fn test_crdt_message_envelope_plain_roundtrip() {
    let ts = Timestamp::parse("2024-05-15T12:00:00.000Z-0001-0123456789ABCDEF").unwrap();
    let msg = CrdtMessage::new(ts, "categories", "cat-1", "name", "Groceries");

    // Plain envelope
    let envelope = msg.to_envelope(None).expect("encode plain envelope");
    assert!(!envelope.is_encrypted);
    assert_eq!(
        envelope.timestamp,
        "2024-05-15T12:00:00.000Z-0001-0123456789ABCDEF"
    );

    let restored = CrdtMessage::from_envelope(&envelope, None).expect("decode plain envelope");
    assert_eq!(restored, msg);
}

#[test]
fn test_crdt_message_envelope_encrypted_roundtrip() {
    let key = Key::from_password(
        Some("key-1".into()),
        "user-secret-budget-pass",
        "salt-12345",
    )
    .expect("derive key");

    let ts = Timestamp::parse("2024-05-15T12:00:00.000Z-0002-0123456789ABCDEF").unwrap();
    let msg = CrdtMessage::new(ts, "accounts", "acc-100", "name", "Secret Offshore Account");

    // Encrypted envelope
    let envelope = msg
        .to_envelope(Some(&key))
        .expect("encode encrypted envelope");
    assert!(envelope.is_encrypted);
    // Wire content must not leak sensitive data
    assert!(!String::from_utf8_lossy(&envelope.content).contains("Secret Offshore Account"));

    // Decrypt with correct key
    let restored =
        CrdtMessage::from_envelope(&envelope, Some(&key)).expect("decode encrypted envelope");
    assert_eq!(restored, msg);

    // Decrypt with wrong key must fail
    let wrong_key = Key::from_password(Some("key-2".into()), "wrong-pass", "salt-12345").unwrap();
    assert!(CrdtMessage::from_envelope(&envelope, Some(&wrong_key)).is_err());

    // Decrypt without key must fail
    assert!(CrdtMessage::from_envelope(&envelope, None).is_err());
}

#[test]
fn test_crdt_tracker_send_change_and_persistence() {
    let conn = setup_test_db();
    let mut tracker = CrdtTracker::with_client_id("clientA");

    let msg1 = tracker
        .send_change(&conn, "accounts", "acc-1", "name", "Checking Account")
        .expect("send change 1");
    let msg2 = tracker
        .send_change(&conn, "accounts", "acc-1", "closed", "0")
        .expect("send change 2");

    assert!(msg2.timestamp > msg1.timestamp);
    assert_ne!(tracker.merkle.hash, 0);

    // Verify written to SQLite
    let messages = CrdtTracker::get_messages_since(&conn, None).expect("get messages");
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].column, "name");
    assert_eq!(messages[1].column, "closed");

    // Reload state from database
    let restored_tracker = CrdtTracker::load_from_sqlite(&conn).expect("restore tracker from db");
    assert_eq!(restored_tracker.merkle.hash, tracker.merkle.hash);
    assert_eq!(
        restored_tracker.clock.timestamp(),
        tracker.clock.timestamp()
    );
}

#[test]
fn test_crdt_lww_conflict_resolution() {
    let conn_a = setup_test_db();
    let conn_b = setup_test_db();

    let mut tracker_a = CrdtTracker::with_client_id("0000000000000001");
    let mut tracker_b = CrdtTracker::with_client_id("0000000000000002");

    // Client A creates category with name "Food" at T1
    let msg_a = tracker_a
        .send_change(&conn_a, "categories", "cat-groceries", "name", "Food")
        .unwrap();

    // Client B modifies same category name to "Dining Out" at T2
    // Ensure T2 physical time is strictly ahead
    thread::sleep(Duration::from_millis(5));
    let msg_b = tracker_b
        .send_change(&conn_b, "categories", "cat-groceries", "name", "Dining Out")
        .unwrap();

    assert!(
        msg_b.timestamp > msg_a.timestamp,
        "Client B timestamp must be newer"
    );

    // 1. Client A receives Client B's message -> B wins (LWW)
    let apply_res_a = tracker_a
        .apply_remote_messages(&conn_a, std::slice::from_ref(&msg_b))
        .expect("apply remote msg on client A");
    assert_eq!(apply_res_a.applied_count, 1);
    assert_eq!(apply_res_a.ignored_count, 0);

    // Verify row in Client A's categories table was updated to "Dining Out"
    let val_a: String = conn_a
        .query_row(
            "SELECT name FROM categories WHERE id = 'cat-groceries'",
            [],
            |r| r.get(0),
        )
        .expect("query category name");
    assert_eq!(val_a, "Dining Out");

    // 2. Client B receives Client A's older message -> A loses (LWW ignored)
    let apply_res_b = tracker_b
        .apply_remote_messages(&conn_b, std::slice::from_ref(&msg_a))
        .expect("apply remote msg on client B");
    assert_eq!(apply_res_b.applied_count, 0);
    assert_eq!(apply_res_b.ignored_count, 1);

    // Verify row in Client B's categories table remains "Dining Out"
    let val_b: String = conn_b
        .query_row(
            "SELECT name FROM categories WHERE id = 'cat-groceries'",
            [],
            |r| r.get(0),
        )
        .expect("query category name");
    assert_eq!(val_b, "Dining Out");

    // Both clients converged to identical state!
    assert_eq!(val_a, val_b);
}

#[test]
fn test_sync_client_request_response_building() {
    let key = Key::from_password(Some("key-sync".into()), "sync-pass", "salt").unwrap();
    let client = SyncClient::new("https://sync.example.com", "file-uuid-1", "group-uuid-1")
        .with_encryption("key-sync", key.clone());

    let ts = Timestamp::parse("2024-05-15T10:00:00.000Z-0001-0000000000000001").unwrap();
    let outgoing = vec![CrdtMessage::new(ts, "accounts", "acc-1", "closed", "1")];

    // Build SyncRequest
    let req = client
        .build_sync_request(None, &outgoing)
        .expect("build sync request");
    assert_eq!(req.file_id, "file-uuid-1");
    assert_eq!(req.group_id, "group-uuid-1");
    assert_eq!(req.key_id, "key-sync");
    assert_eq!(req.messages.len(), 1);
    assert!(req.messages[0].is_encrypted);

    // Mock incoming SyncResponse with remote trie and incoming envelope
    let remote_ts = Timestamp::parse("2024-05-15T10:05:00.000Z-0001-0000000000000002").unwrap();
    let remote_msg = CrdtMessage::new(remote_ts, "categories", "cat-fun", "name", "Entertainment");
    let remote_env = remote_msg.to_envelope(Some(&key)).unwrap();

    let mut server_trie = TrieNode::empty();
    server_trie.insert(&remote_msg.timestamp);

    let resp = SyncResponse {
        messages: vec![remote_env],
        merkle: serde_json::to_string(&server_trie).unwrap(),
    };

    let (parsed_trie, decoded_messages) = client
        .parse_sync_response(&resp)
        .expect("parse sync response");
    assert_eq!(parsed_trie.hash, server_trie.hash);
    assert_eq!(decoded_messages.len(), 1);
    assert_eq!(decoded_messages[0], remote_msg);
}

#[test]
fn test_offline_queue_buffering_and_backoff() {
    let mut queue = OfflineQueue::new();
    assert_eq!(queue.status(), &SyncStatus::Synced);
    assert_eq!(queue.pending_count(), 0);

    let ts1 = Timestamp::parse("2024-05-15T10:00:00.000Z-0001-0000000000000001").unwrap();
    let msg1 = CrdtMessage::new(ts1, "categories", "cat-1", "name", "Books");

    // Enqueue message
    queue.enqueue(msg1.clone());
    assert_eq!(queue.status(), &SyncStatus::Offline);
    assert_eq!(queue.pending_count(), 1);

    // Test backoff calculation
    let mut backoff = BackoffStrategy::new(1000, 16000);
    assert_eq!(backoff.next_delay(), Duration::from_millis(1000));
    assert_eq!(backoff.next_delay(), Duration::from_millis(2000));
    assert_eq!(backoff.next_delay(), Duration::from_millis(4000));
    assert_eq!(backoff.next_delay(), Duration::from_millis(8000));
    assert_eq!(backoff.next_delay(), Duration::from_millis(16000));
    // Ceiled at max
    assert_eq!(backoff.next_delay(), Duration::from_millis(16000));

    backoff.reset();
    assert_eq!(backoff.next_delay(), Duration::from_millis(1000));
}

#[test]
fn test_end_to_end_wire_sync_mock_server() {
    let conn = setup_test_db();
    let mut tracker = CrdtTracker::with_client_id("clientLocal");

    // Generate a local change
    tracker
        .send_change(&conn, "accounts", "acc-wire-1", "name", "Wire Checking")
        .expect("send change");

    // Bind local TCP mock sync server
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local tcp");
    let port = listener.local_addr().unwrap().port();

    let server_handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept connection");
        let mut buffer = Vec::new();
        let mut temp = [0u8; 1024];

        let mut header_end = None;
        let mut content_length = 0usize;

        while header_end.is_none() {
            let n = stream.read(&mut temp).expect("read header");
            if n == 0 {
                break;
            }
            buffer.extend_from_slice(&temp[..n]);
            if let Some(pos) = buffer.windows(4).position(|w| w == b"\r\n\r\n") {
                header_end = Some(pos);
                let header_str = String::from_utf8_lossy(&buffer[..pos]);
                for line in header_str.lines() {
                    if let Some(val) = line.to_lowercase().strip_prefix("content-length:") {
                        content_length = val.trim().parse::<usize>().unwrap_or(0);
                    }
                }
            }
        }

        let header_pos = header_end.expect("found headers");
        let body_start = header_pos + 4;

        while buffer.len() - body_start < content_length {
            let n = stream.read(&mut temp).expect("read body");
            if n == 0 {
                break;
            }
            buffer.extend_from_slice(&temp[..n]);
        }

        let body_bytes = &buffer[body_start..body_start + content_length];
        let req = SyncRequest::decode(body_bytes).expect("decode sync request protobuf");

        assert_eq!(req.file_id, "budget-file-123");
        assert_eq!(req.messages.len(), 1);

        // Build server response returning an additional remote transaction
        let remote_ts = Timestamp::parse("2024-05-15T15:30:00.000Z-0001-0000000000000099").unwrap();
        let remote_msg =
            CrdtMessage::new(remote_ts, "categories", "cat-wire", "name", "Wire Category");
        let remote_env = remote_msg.to_envelope(None).unwrap();

        let mut server_trie = TrieNode::empty();
        server_trie.insert(&remote_msg.timestamp);

        let sync_resp = SyncResponse {
            messages: vec![remote_env],
            merkle: serde_json::to_string(&server_trie).unwrap(),
        };

        let mut resp_body = Vec::new();
        sync_resp.encode(&mut resp_body).unwrap();

        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/actual-sync\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            resp_body.len()
        );

        stream.write_all(header.as_bytes()).unwrap();
        stream.write_all(&resp_body).unwrap();
    });

    let client = SyncClient::new(
        format!("http://127.0.0.1:{port}"),
        "budget-file-123",
        "group-1",
    );

    let result = client
        .sync_with_server(&conn, &mut tracker, None)
        .expect("sync with mock server");

    server_handle.join().unwrap();

    assert_eq!(result.uploaded_count, 1);
    assert_eq!(result.downloaded_count, 1);
    assert_eq!(result.applied_count, 1);

    // Verify remote category was inserted into local categories table
    let cat_name: String = conn
        .query_row(
            "SELECT name FROM categories WHERE id = 'cat-wire'",
            [],
            |r| r.get(0),
        )
        .expect("query remote category");
    assert_eq!(cat_name, "Wire Category");
}
