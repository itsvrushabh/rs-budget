//! Cross-client synchronization integration tests validating end-to-end multi-device
//! interoperability, concurrent offline edits, deterministic Last-Write-Wins (LWW) conflict
//! resolution, Merkle trie hash convergence, AES-256-GCM encrypted envelope exchange,
//! and offline queue reconnects against the 100% Rust self-hosted Axum server.

use actual_crypto::Key;
use actual_server::{create_app, ServerState};
use actual_storage::SqliteStore;
use actual_sync::crdt::CrdtMessage;
use actual_sync::merkle::diff as diff_merkle;
use actual_sync::proto::{SyncRequest, SyncResponse};
use actual_sync::queue::{OfflineQueue, SyncStatus};
use actual_sync::timestamp::Timestamp;
use actual_sync::{CrdtTracker, SyncClient, TrieNode};
use prost::Message as ProstMessage;
use rusqlite::params;
use std::time::Duration;
use tokio::net::TcpListener;

/// Spawns a background Axum server on a random loopback port.
async fn spawn_test_server() -> (String, ServerState) {
    let state = ServerState::open_in_memory().expect("open in-memory server state");
    let app = create_app(state.clone());

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind free port");
    let addr = listener.local_addr().expect("local addr");
    let base_url = format!("http://{}", addr);

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve test app");
    });

    tokio::time::sleep(Duration::from_millis(15)).await;

    (base_url, state)
}

/// Helper to query a single column value from a SQLite table as text.
fn query_str_field(store: &SqliteStore, table: &str, row_id: &str, col: &str) -> Option<String> {
    let sql = format!("SELECT CAST(\"{col}\" AS TEXT) FROM \"{table}\" WHERE id = ?1");
    store
        .conn()
        .query_row(&sql, params![row_id], |row| row.get(0))
        .ok()
}

/// Helper to query row count in a SQLite table.
fn query_row_count(store: &SqliteStore, table: &str) -> usize {
    let sql = format!("SELECT COUNT(1) FROM \"{table}\"");
    let count: i64 = store
        .conn()
        .query_row(&sql, [], |row| row.get(0))
        .unwrap_or(0);
    count as usize
}

#[tokio::test(flavor = "multi_thread")]
async fn test_cross_client_concurrent_independent_edits_convergence() {
    let (server_url, server_state) = spawn_test_server().await;

    let file_id = "shared-budget-001";
    let group_id = "sync-group-001";

    // Setup Client 1 (Alice)
    let store_alice = SqliteStore::open_in_memory().expect("open alice db");
    let mut tracker_alice = CrdtTracker::with_client_id("alice_node_01");
    let client_alice = SyncClient::new(&server_url, file_id, group_id);

    // Setup Client 2 (Bob)
    let store_bob = SqliteStore::open_in_memory().expect("open bob db");
    let mut tracker_bob = CrdtTracker::with_client_id("bob_node_02");
    let client_bob = SyncClient::new(&server_url, file_id, group_id);

    // 1. Alice creates entities offline: Category "Groceries", Account "Checking"
    tracker_alice
        .send_change(
            store_alice.conn(),
            "categories",
            "cat-food",
            "name",
            "Groceries",
        )
        .expect("alice cat name");
    tracker_alice
        .send_change(
            store_alice.conn(),
            "categories",
            "cat-food",
            "is_income",
            "0",
        )
        .expect("alice cat income");
    tracker_alice
        .send_change(
            store_alice.conn(),
            "accounts",
            "acc-check",
            "name",
            "Checking",
        )
        .expect("alice acc name");

    // 2. Bob creates entities offline: Category "Utilities", Account "Savings"
    tracker_bob
        .send_change(
            store_bob.conn(),
            "categories",
            "cat-util",
            "name",
            "Utilities",
        )
        .expect("bob cat name");
    tracker_bob
        .send_change(store_bob.conn(), "categories", "cat-util", "is_income", "0")
        .expect("bob cat income");
    tracker_bob
        .send_change(
            store_bob.conn(),
            "accounts",
            "acc-save",
            "name",
            "High Yield Savings",
        )
        .expect("bob acc name");

    // Initially, both clients only see their own local entities
    assert_eq!(
        query_str_field(&store_alice, "categories", "cat-food", "name"),
        Some("Groceries".into())
    );
    assert_eq!(
        query_str_field(&store_alice, "categories", "cat-util", "name"),
        None
    );

    assert_eq!(
        query_str_field(&store_bob, "categories", "cat-util", "name"),
        Some("Utilities".into())
    );
    assert_eq!(
        query_str_field(&store_bob, "categories", "cat-food", "name"),
        None
    );

    // Merkle roots initially diverge
    assert_ne!(tracker_alice.merkle.hash, tracker_bob.merkle.hash);
    assert!(diff_merkle(&tracker_alice.merkle, &tracker_bob.merkle).is_some());

    // 3. Alice syncs to server first
    let res_a1 = client_alice
        .sync_with_server(store_alice.conn(), &mut tracker_alice, None)
        .expect("alice sync 1");
    assert_eq!(res_a1.uploaded_count, 3);
    assert_eq!(res_a1.downloaded_count, 0);

    // 4. Bob syncs to server: uploads 3 changes, downloads Alice's 3 changes
    let res_b1 = client_bob
        .sync_with_server(store_bob.conn(), &mut tracker_bob, None)
        .expect("bob sync 1");
    assert_eq!(res_b1.uploaded_count, 3);
    assert_eq!(res_b1.downloaded_count, 3);
    assert_eq!(res_b1.applied_count, 3);

    // Now Bob has Alice's category "Groceries" and account "Checking"
    assert_eq!(
        query_str_field(&store_bob, "categories", "cat-food", "name"),
        Some("Groceries".into())
    );
    assert_eq!(
        query_str_field(&store_bob, "accounts", "acc-check", "name"),
        Some("Checking".into())
    );

    // 5. Alice performs catch-up sync to receive Bob's changes
    let res_a2 = client_alice
        .sync_with_server(store_alice.conn(), &mut tracker_alice, None)
        .expect("alice sync 2");
    assert_eq!(res_a2.downloaded_count, 3);
    assert_eq!(res_a2.applied_count, 3);
    assert_eq!(res_a2.ignored_count, 0);

    // Now Alice has Bob's category "Utilities" and account "Savings"
    assert_eq!(
        query_str_field(&store_alice, "categories", "cat-util", "name"),
        Some("Utilities".into())
    );
    assert_eq!(
        query_str_field(&store_alice, "accounts", "acc-save", "name"),
        Some("High Yield Savings".into())
    );

    // 6. TOTAL CONVERGENCE VERIFICATION:
    // Merkle root hashes on Alice and Bob must be identical!
    assert_eq!(tracker_alice.merkle.hash, tracker_bob.merkle.hash);
    assert_eq!(
        diff_merkle(&tracker_alice.merkle, &tracker_bob.merkle),
        None
    );

    // Server Merkle trie must also match
    let server_trie: TrieNode = {
        let conn = server_state.db.lock().unwrap();
        let trie_json: String = conn
            .query_row(
                "SELECT trie_json FROM merkle_tries WHERE file_id = ?1 AND group_id = ?2",
                params![file_id, group_id],
                |row| row.get(0),
            )
            .expect("server trie");
        serde_json::from_str(&trie_json).expect("parse server trie")
    };
    assert_eq!(server_trie.hash, tracker_alice.merkle.hash);
    assert_eq!(diff_merkle(&tracker_alice.merkle, &server_trie), None);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_cross_client_deterministic_lww_conflict_resolution() {
    let (server_url, _) = spawn_test_server().await;

    let file_id = "conflict-budget-002";
    let group_id = "sync-group-002";

    let store_a = SqliteStore::open_in_memory().expect("open store a");
    let mut tracker_a = CrdtTracker::with_client_id("node_alpha");
    let client_a = SyncClient::new(&server_url, file_id, group_id);

    let store_b = SqliteStore::open_in_memory().expect("open store b");
    let mut tracker_b = CrdtTracker::with_client_id("node_beta");
    let client_b = SyncClient::new(&server_url, file_id, group_id);

    // Both start with a shared initial category
    tracker_a
        .send_change(
            store_a.conn(),
            "categories",
            "cat-dining",
            "name",
            "Dining Out",
        )
        .unwrap();
    client_a
        .sync_with_server(store_a.conn(), &mut tracker_a, None)
        .unwrap();

    // Client B pulls initial state
    client_b
        .sync_with_server(store_b.conn(), &mut tracker_b, None)
        .unwrap();
    assert_eq!(
        query_str_field(&store_b, "categories", "cat-dining", "name"),
        Some("Dining Out".into())
    );

    // --- CONFLICT SCENARIO ---
    // Client A and Client B go offline and edit the exact same column `name` on `cat-dining`.
    // Client A sets it to "Restaurants & Bars" at timestamp T_a
    // Client B sets it to "Eats & Takeout" at timestamp T_b (after waiting 2ms so T_b > T_a)
    tracker_a
        .send_change(
            store_a.conn(),
            "categories",
            "cat-dining",
            "name",
            "Restaurants & Bars",
        )
        .unwrap();

    tokio::time::sleep(Duration::from_millis(5)).await;

    tracker_b
        .send_change(
            store_b.conn(),
            "categories",
            "cat-dining",
            "name",
            "Eats & Takeout",
        )
        .unwrap();

    // Bob syncs to server first (uploading initial + T_b)
    let b_res = client_b
        .sync_with_server(store_b.conn(), &mut tracker_b, None)
        .unwrap();
    assert_eq!(b_res.uploaded_count, 2);

    // Alice syncs to server (uploading initial + T_a, and pulling T_b)
    let a_res = client_a
        .sync_with_server(store_a.conn(), &mut tracker_a, None)
        .unwrap();
    assert_eq!(a_res.uploaded_count, 2);
    assert_eq!(a_res.downloaded_count, 1);
    // Because T_b > T_a, Alice's tracker must accept T_b as winner!
    assert_eq!(a_res.applied_count, 1);
    assert_eq!(a_res.ignored_count, 0);

    // Bob syncs to pull Alice's T_a
    let b_res2 = client_b
        .sync_with_server(store_b.conn(), &mut tracker_b, None)
        .unwrap();
    assert_eq!(b_res2.downloaded_count, 1);
    // Because T_b > T_a, Bob's tracker must ignore older T_a and previous history!
    assert_eq!(b_res2.applied_count, 0);
    assert_eq!(b_res2.ignored_count, 1);

    // Both clients deterministically display the LWW winner: "Eats & Takeout"
    assert_eq!(
        query_str_field(&store_a, "categories", "cat-dining", "name"),
        Some("Eats & Takeout".into())
    );
    assert_eq!(
        query_str_field(&store_b, "categories", "cat-dining", "name"),
        Some("Eats & Takeout".into())
    );

    // Both clients converge to identical Merkle hash
    assert_eq!(tracker_a.merkle.hash, tracker_b.merkle.hash);
    assert_eq!(diff_merkle(&tracker_a.merkle, &tracker_b.merkle), None);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_cross_client_end_to_end_aes256_gcm_encrypted_sync() {
    let (server_url, server_state) = spawn_test_server().await;

    let file_id = "enc-budget-003";
    let group_id = "sync-group-003";
    let key_id = "master-key-01";

    let key = Key::from_password(
        Some(key_id.into()),
        "ultra-secret-user-passphrase",
        "salt-98765",
    )
    .unwrap();

    let store_alice = SqliteStore::open_in_memory().unwrap();
    let mut tracker_alice = CrdtTracker::with_client_id("alice_enc");
    let client_alice =
        SyncClient::new(&server_url, file_id, group_id).with_encryption(key_id, key.clone());

    let store_bob = SqliteStore::open_in_memory().unwrap();
    let mut tracker_bob = CrdtTracker::with_client_id("bob_enc");
    let client_bob =
        SyncClient::new(&server_url, file_id, group_id).with_encryption(key_id, key.clone());

    // Alice creates confidential accounts and payees
    tracker_alice
        .send_change(
            store_alice.conn(),
            "accounts",
            "acc-confidential",
            "name",
            "Secret Swiss Account",
        )
        .unwrap();
    tracker_alice
        .send_change(
            store_alice.conn(),
            "payees",
            "payee-confidential",
            "name",
            "Anonymous Consultant",
        )
        .unwrap();

    // Alice syncs to server with AES-256-GCM encryption enabled
    let res_a = client_alice
        .sync_with_server(store_alice.conn(), &mut tracker_alice, None)
        .expect("alice enc sync");
    assert_eq!(res_a.uploaded_count, 2);

    // Direct inspection of server state: verify server is ZERO-KNOWLEDGE
    {
        let conn = server_state.db.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT is_encrypted, content FROM messages WHERE file_id = ?1")
            .unwrap();
        let rows = stmt
            .query_map(params![file_id], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, Vec<u8>>(1)?))
            })
            .unwrap();

        for r in rows {
            let (is_enc, content) = r.unwrap();
            assert_eq!(is_enc, 1, "Server must store message as encrypted");
            let raw_str = String::from_utf8_lossy(&content);
            assert!(
                !raw_str.contains("Secret Swiss Account"),
                "Server ciphertext must not leak account name"
            );
            assert!(
                !raw_str.contains("Anonymous Consultant"),
                "Server ciphertext must not leak payee name"
            );
        }
    }

    // Bob syncs from server using the shared key
    let res_b = client_bob
        .sync_with_server(store_bob.conn(), &mut tracker_bob, None)
        .expect("bob enc sync");
    assert_eq!(res_b.downloaded_count, 2);
    assert_eq!(res_b.applied_count, 2);

    // Bob successfully decrypted and applied changes into SQLite
    assert_eq!(
        query_str_field(&store_bob, "accounts", "acc-confidential", "name"),
        Some("Secret Swiss Account".into())
    );
    assert_eq!(
        query_str_field(&store_bob, "payees", "payee-confidential", "name"),
        Some("Anonymous Consultant".into())
    );

    // Merkle hashes match between Alice and Bob
    assert_eq!(tracker_alice.merkle.hash, tracker_bob.merkle.hash);
    assert_eq!(
        diff_merkle(&tracker_alice.merkle, &tracker_bob.merkle),
        None
    );

    // A third client with the WRONG key fails to decrypt
    let wrong_key =
        Key::from_password(Some("wrong-key".into()), "wrong-pass", "salt-98765").unwrap();
    let client_eve =
        SyncClient::new(&server_url, file_id, group_id).with_encryption("wrong-key", wrong_key);

    let store_eve = SqliteStore::open_in_memory().unwrap();
    let mut tracker_eve = CrdtTracker::with_client_id("eve_unauthorized");

    let eve_res = client_eve.sync_with_server(store_eve.conn(), &mut tracker_eve, None);
    assert!(
        eve_res.is_err(),
        "Client with incorrect key must fail decryption"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_cross_client_offline_queue_drain_and_reconnect() {
    let (server_url, _) = spawn_test_server().await;

    let file_id = "queue-budget-004";
    let group_id = "sync-group-004";

    let store_client = SqliteStore::open_in_memory().unwrap();
    let mut tracker = CrdtTracker::with_client_id("mobile_client_01");

    // Initialize OfflineQueue
    let mut queue = OfflineQueue::new();
    assert_eq!(*queue.status(), SyncStatus::Synced);

    // Client makes edits while offline
    let msg1 = tracker
        .send_change(
            store_client.conn(),
            "categories",
            "cat-offline-1",
            "name",
            "Books & Education",
        )
        .unwrap();
    let msg2 = tracker
        .send_change(
            store_client.conn(),
            "accounts",
            "acc-offline-1",
            "name",
            "Cash Wallet",
        )
        .unwrap();

    // Enqueue messages
    queue.enqueue(msg1);
    queue.enqueue(msg2);
    assert_eq!(queue.pending_count(), 2);
    assert_eq!(*queue.status(), SyncStatus::Offline);

    // Flush offline queue through SyncClient
    let client = SyncClient::new(&server_url, file_id, group_id);
    let cycle = queue
        .flush_queue(&client, store_client.conn(), &mut tracker, None)
        .unwrap();

    assert_eq!(cycle.uploaded_count, 2);
    assert_eq!(queue.pending_count(), 0);
    assert_eq!(*queue.status(), SyncStatus::Synced);

    // Another client connects and receives the queued items
    let store_receiver = SqliteStore::open_in_memory().unwrap();
    let mut tracker_receiver = CrdtTracker::with_client_id("desktop_receiver");
    let receiver = SyncClient::new(&server_url, file_id, group_id);

    let recv_cycle = receiver
        .sync_with_server(store_receiver.conn(), &mut tracker_receiver, None)
        .unwrap();
    assert_eq!(recv_cycle.downloaded_count, 2);
    assert_eq!(recv_cycle.applied_count, 2);

    assert_eq!(
        query_str_field(&store_receiver, "categories", "cat-offline-1", "name"),
        Some("Books & Education".into())
    );
    assert_eq!(
        query_str_field(&store_receiver, "accounts", "acc-offline-1", "name"),
        Some("Cash Wallet".into())
    );

    assert_eq!(tracker.merkle.hash, tracker_receiver.merkle.hash);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_cross_client_tombstones_and_deletions() {
    let (server_url, _) = spawn_test_server().await;

    let file_id = "tombstone-budget-005";
    let group_id = "sync-group-005";

    let store_a = SqliteStore::open_in_memory().unwrap();
    let mut tracker_a = CrdtTracker::with_client_id("client_a");
    let client_a = SyncClient::new(&server_url, file_id, group_id);

    let store_b = SqliteStore::open_in_memory().unwrap();
    let mut tracker_b = CrdtTracker::with_client_id("client_b");
    let client_b = SyncClient::new(&server_url, file_id, group_id);

    // 1. Client A creates payee "Coffee Shop"
    tracker_a
        .send_change(
            store_a.conn(),
            "payees",
            "payee-coffee",
            "name",
            "Coffee Shop",
        )
        .unwrap();
    tracker_a
        .send_change(store_a.conn(), "payees", "payee-coffee", "tombstone", "0")
        .unwrap();
    client_a
        .sync_with_server(store_a.conn(), &mut tracker_a, None)
        .unwrap();

    // 2. Client B receives payee
    client_b
        .sync_with_server(store_b.conn(), &mut tracker_b, None)
        .unwrap();
    assert_eq!(
        query_str_field(&store_b, "payees", "payee-coffee", "name"),
        Some("Coffee Shop".into())
    );
    assert_eq!(
        query_str_field(&store_b, "payees", "payee-coffee", "tombstone"),
        Some("0".into())
    );

    // 3. Client A soft-deletes (tombstones) payee
    tracker_a
        .send_change(store_a.conn(), "payees", "payee-coffee", "tombstone", "1")
        .unwrap();
    client_a
        .sync_with_server(store_a.conn(), &mut tracker_a, None)
        .unwrap();

    // 4. Client B pulls change
    client_b
        .sync_with_server(store_b.conn(), &mut tracker_b, None)
        .unwrap();

    // Both clients now show tombstone = 1
    assert_eq!(
        query_str_field(&store_a, "payees", "payee-coffee", "tombstone"),
        Some("1".into())
    );
    assert_eq!(
        query_str_field(&store_b, "payees", "payee-coffee", "tombstone"),
        Some("1".into())
    );

    assert_eq!(tracker_a.merkle.hash, tracker_b.merkle.hash);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_cross_client_interleaved_multi_round_sync() {
    let (server_url, _) = spawn_test_server().await;

    let file_id = "interleaved-budget-006";
    let group_id = "sync-group-006";

    let store_a = SqliteStore::open_in_memory().unwrap();
    let mut tracker_a = CrdtTracker::with_client_id("worker_a");
    let client_a = SyncClient::new(&server_url, file_id, group_id);

    let store_b = SqliteStore::open_in_memory().unwrap();
    let mut tracker_b = CrdtTracker::with_client_id("worker_b");
    let client_b = SyncClient::new(&server_url, file_id, group_id);

    // Perform 5 alternating rounds of modifications and syncs
    for round in 1..=5 {
        let cat_id = format!("cat-round-{round}");
        let acc_id = format!("acc-round-{round}");

        // Worker A creates category
        tracker_a
            .send_change(
                store_a.conn(),
                "categories",
                &cat_id,
                "name",
                &format!("Category {round}"),
            )
            .unwrap();

        // Worker B creates account
        tracker_b
            .send_change(
                store_b.conn(),
                "accounts",
                &acc_id,
                "name",
                &format!("Account {round}"),
            )
            .unwrap();

        // Both sync to server
        client_a
            .sync_with_server(store_a.conn(), &mut tracker_a, None)
            .unwrap();
        client_b
            .sync_with_server(store_b.conn(), &mut tracker_b, None)
            .unwrap();

        // Mutual catch-up sync
        client_a
            .sync_with_server(store_a.conn(), &mut tracker_a, None)
            .unwrap();
        client_b
            .sync_with_server(store_b.conn(), &mut tracker_b, None)
            .unwrap();

        // Verify both have created entities for this round
        assert_eq!(
            query_str_field(&store_a, "categories", &cat_id, "name"),
            Some(format!("Category {round}"))
        );
        assert_eq!(
            query_str_field(&store_b, "categories", &cat_id, "name"),
            Some(format!("Category {round}"))
        );
        assert_eq!(
            query_str_field(&store_a, "accounts", &acc_id, "name"),
            Some(format!("Account {round}"))
        );
        assert_eq!(
            query_str_field(&store_b, "accounts", &acc_id, "name"),
            Some(format!("Account {round}"))
        );

        // Merkle hashes strictly match at the end of each round
        assert_eq!(
            tracker_a.merkle.hash, tracker_b.merkle.hash,
            "Merkle hashes must match after round {round}"
        );
    }

    assert_eq!(query_row_count(&store_a, "categories"), 5);
    assert_eq!(query_row_count(&store_b, "categories"), 5);
    assert_eq!(query_row_count(&store_a, "accounts"), 5);
    assert_eq!(query_row_count(&store_b, "accounts"), 5);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_upstream_sync_wire_format_compatibility() {
    let (server_url, _) = spawn_test_server().await;

    // Build raw SyncRequest matching upstream JavaScript/Node.js protobuf wire structure
    let ts = Timestamp::parse("2024-06-01T10:00:00.000Z-0001-0123456789ABCDEF").unwrap();
    let msg = CrdtMessage::new(
        ts.clone(),
        "category_groups",
        "grp-upstream",
        "name",
        "Upstream Group",
    );
    let env = msg.to_envelope(None).unwrap();

    let req = SyncRequest {
        messages: vec![env.clone()],
        file_id: "upstream-wire-file".into(),
        group_id: "upstream-wire-group".into(),
        key_id: String::new(),
        since: String::new(),
    };

    let mut body = Vec::new();
    req.encode(&mut body).unwrap();

    // Verify official HTTP headers expected by Node.js actual-server and web client
    let resp_raw = ureq::post(&format!("{server_url}/sync/sync"))
        .set("Content-Type", "application/actual-sync")
        .set("X-Actual-Sync-Version", "1")
        .send_bytes(&body)
        .expect("send upstream sync request");

    assert_eq!(resp_raw.status(), 200);
    assert_eq!(
        resp_raw.header("Content-Type"),
        Some("application/actual-sync")
    );

    let mut resp_bytes = Vec::new();
    resp_raw.into_reader().read_to_end(&mut resp_bytes).unwrap();
    let response =
        SyncResponse::decode(resp_bytes.as_slice()).expect("decode upstream protobuf response");

    // Server should return 0 new messages to sender
    assert_eq!(response.messages.len(), 0);

    // Merkle field must be valid JSON deserializable into TrieNode
    let trie: TrieNode =
        serde_json::from_str(&response.merkle).expect("deserialize upstream trie JSON");
    assert_ne!(trie.hash, 0);
}
