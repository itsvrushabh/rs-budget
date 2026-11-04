//! Integration tests for actual-server: authentication, file lifecycle, key management,
//! and multi-client CRDT sync protocol.

use actual_crypto::Key;
use actual_server::{create_app, ServerState};
use actual_sync::crdt::CrdtMessage;
use actual_sync::proto::{SyncRequest, SyncResponse};
use actual_sync::timestamp::Timestamp;
use actual_sync::TrieNode;
use prost::Message as ProstMessage;
use serde_json::Value;
use std::time::Duration;
use tokio::net::TcpListener;

async fn spawn_test_server() -> (String, ServerState) {
    let state = ServerState::open_in_memory().expect("in-memory state");
    let app = create_app(state.clone());

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind free port");
    let addr = listener.local_addr().expect("local addr");
    let base_url = format!("http://{}", addr);

    tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve app");
    });

    // Give server a moment to start
    tokio::time::sleep(Duration::from_millis(15)).await;

    (base_url, state)
}

#[tokio::test(flavor = "multi_thread")]
async fn test_server_account_bootstrap_and_login() {
    let (base_url, _) = spawn_test_server().await;

    // 1. Initially needs bootstrap
    let resp: Value = ureq::get(&format!("{base_url}/account/needs-bootstrap"))
        .call()
        .expect("needs bootstrap req")
        .into_json()
        .expect("parse json");
    assert_eq!(resp["data"]["bootstrapped"], false);

    // 2. Bootstrap server password
    let bootstrap_resp: Value = ureq::post(&format!("{base_url}/account/bootstrap"))
        .send_json(serde_json::json!({ "password": "super-secret-master-pass" }))
        .expect("bootstrap req")
        .into_json()
        .expect("parse json");
    assert_eq!(bootstrap_resp["status"], "ok");
    let token = bootstrap_resp["data"]["token"]
        .as_str()
        .expect("token string");
    assert!(!token.is_empty());

    // 3. Now bootstrapped
    let resp2: Value = ureq::get(&format!("{base_url}/account/needs-bootstrap"))
        .call()
        .expect("needs bootstrap req")
        .into_json()
        .expect("parse json");
    assert_eq!(resp2["data"]["bootstrapped"], true);

    // 4. Validate session token
    let val_resp: Value = ureq::post(&format!("{base_url}/account/validate"))
        .set("X-Actual-Token", token)
        .call()
        .expect("validate req")
        .into_json()
        .expect("parse json");
    assert_eq!(val_resp["status"], "ok");

    // 5. Login with wrong password fails (401)
    let bad_login = ureq::post(&format!("{base_url}/account/login"))
        .send_json(serde_json::json!({ "password": "wrong-password" }));
    assert!(bad_login.is_err());

    // 6. Login with correct password succeeds
    let login_resp: Value = ureq::post(&format!("{base_url}/account/login"))
        .send_json(serde_json::json!({ "password": "super-secret-master-pass" }))
        .expect("login req")
        .into_json()
        .expect("parse json");
    assert_eq!(login_resp["status"], "ok");
    assert!(login_resp["data"]["token"].is_string());
}

#[tokio::test(flavor = "multi_thread")]
async fn test_server_user_files_lifecycle() {
    let (base_url, _) = spawn_test_server().await;

    let dummy_archive = b"SIMULATED_ACTUAL_ENCRYPTED_BACKUP_BYTES_123456789";

    // 1. Upload user file
    let upload_resp: Value = ureq::post(&format!("{base_url}/sync/upload-user-file"))
        .set("X-Actual-File-Id", "bgt-file-100")
        .set("X-Actual-Name", "Family Budget")
        .set("X-Actual-Group-Id", "sync-group-alpha")
        .send_bytes(dummy_archive)
        .expect("upload req")
        .into_json()
        .expect("parse json");
    assert_eq!(upload_resp["status"], "ok");
    assert_eq!(upload_resp["data"]["fileId"], "bgt-file-100");
    assert_eq!(upload_resp["data"]["groupId"], "sync-group-alpha");

    // 2. List user files
    let list_resp: Value = ureq::get(&format!("{base_url}/sync/list-user-files"))
        .call()
        .expect("list req")
        .into_json()
        .expect("parse json");
    let files = list_resp["data"].as_array().expect("files array");
    assert_eq!(files.len(), 1);
    assert_eq!(files[0]["fileId"], "bgt-file-100");
    assert_eq!(files[0]["name"], "Family Budget");

    // 3. Download user file
    let mut downloaded = Vec::new();
    ureq::get(&format!(
        "{base_url}/sync/download-user-file?fileId=bgt-file-100"
    ))
    .call()
    .expect("download req")
    .into_reader()
    .read_to_end(&mut downloaded)
    .expect("read body");
    assert_eq!(downloaded, dummy_archive);

    // 4. Update filename
    let rename_resp: Value = ureq::post(&format!("{base_url}/sync/update-user-filename"))
        .send_json(serde_json::json!({
            "fileId": "bgt-file-100",
            "name": "Updated Family Budget"
        }))
        .expect("rename req")
        .into_json()
        .expect("parse json");
    assert_eq!(rename_resp["status"], "ok");

    let info_resp: Value = ureq::get(&format!(
        "{base_url}/sync/get-user-file-info?fileId=bgt-file-100"
    ))
    .call()
    .expect("info req")
    .into_json()
    .expect("parse json");
    assert_eq!(info_resp["data"]["name"], "Updated Family Budget");

    // 5. Delete user file
    let del_resp: Value = ureq::post(&format!("{base_url}/sync/delete-user-file"))
        .send_json(serde_json::json!({ "fileId": "bgt-file-100" }))
        .expect("delete req")
        .into_json()
        .expect("parse json");
    assert_eq!(del_resp["status"], "ok");

    // Now hidden from list-user-files
    let list_resp2: Value = ureq::get(&format!("{base_url}/sync/list-user-files"))
        .call()
        .expect("list req")
        .into_json()
        .expect("parse json");
    assert_eq!(list_resp2["data"].as_array().unwrap().len(), 0);

    // 6. Reset user file
    let reset_resp: Value = ureq::post(&format!("{base_url}/sync/reset-user-file"))
        .send_json(serde_json::json!({ "fileId": "bgt-file-100" }))
        .expect("reset req")
        .into_json()
        .expect("parse json");
    assert_eq!(reset_resp["status"], "ok");
    assert_ne!(reset_resp["data"]["groupId"], "sync-group-alpha");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_server_encryption_keys() {
    let (base_url, _) = spawn_test_server().await;

    // Create file entry first
    let _ = ureq::post(&format!("{base_url}/sync/upload-user-file"))
        .set("X-Actual-File-Id", "file-key-test")
        .set("X-Actual-Name", "Key Test")
        .send_bytes(b"dummy")
        .expect("upload");

    // 1. Initially no key
    let get_resp: Value = ureq::post(&format!("{base_url}/user/get-key"))
        .send_json(serde_json::json!({ "fileId": "file-key-test" }))
        .expect("get key")
        .into_json()
        .expect("parse json");
    assert_eq!(get_resp["data"], Value::Null);

    // 2. Register key
    let create_resp: Value = ureq::post(&format!("{base_url}/user/create-key"))
        .send_json(serde_json::json!({
            "fileId": "file-key-test",
            "keyId": "key-master-777",
            "salt": "base64-random-salt-XYZ",
            "iterations": 10000
        }))
        .expect("create key")
        .into_json()
        .expect("parse json");
    assert_eq!(create_resp["status"], "ok");

    // 3. Query key again
    let get_resp2: Value = ureq::post(&format!("{base_url}/user/get-key"))
        .send_json(serde_json::json!({ "fileId": "file-key-test" }))
        .expect("get key")
        .into_json()
        .expect("parse json");
    assert_eq!(get_resp2["data"]["keyId"], "key-master-777");
    assert_eq!(get_resp2["data"]["salt"], "base64-random-salt-XYZ");
    assert_eq!(get_resp2["data"]["iterations"], 10000);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_server_crdt_sync_endpoint() {
    let (base_url, _) = spawn_test_server().await;

    let key = Key::from_password(Some("k1".into()), "test-pass", "salt").unwrap();

    // 1. Client A sends Message 1
    let ts_a = Timestamp::parse("2024-05-15T12:00:00.000Z-0001-0000000000000001").unwrap();
    let msg_a = CrdtMessage::new(ts_a.clone(), "categories", "cat-food", "name", "Groceries");
    let env_a = msg_a.to_envelope(Some(&key)).unwrap();

    let req_a = SyncRequest {
        messages: vec![env_a.clone()],
        file_id: "file-sync-test".into(),
        group_id: "group-sync-test".into(),
        key_id: "k1".into(),
        since: "".into(),
    };

    let mut body_a = Vec::new();
    req_a.encode(&mut body_a).unwrap();

    let resp_a_raw = ureq::post(&format!("{base_url}/sync/sync"))
        .set("Content-Type", "application/actual-sync")
        .send_bytes(&body_a)
        .expect("client A sync req");

    let mut resp_a_bytes = Vec::new();
    resp_a_raw
        .into_reader()
        .read_to_end(&mut resp_a_bytes)
        .unwrap();
    let sync_resp_a =
        SyncResponse::decode(resp_a_bytes.as_slice()).expect("decode sync response A");

    // Server should return 0 new messages to Client A (since A was the sender)
    assert_eq!(sync_resp_a.messages.len(), 0);
    // Server Merkle trie should be non-empty
    let trie_a: TrieNode = serde_json::from_str(&sync_resp_a.merkle).expect("parse trie");
    assert_ne!(trie_a.hash, 0);

    // 2. Client B syncs with empty messages (pulling server state)
    let req_b1 = SyncRequest {
        messages: Vec::new(),
        file_id: "file-sync-test".into(),
        group_id: "group-sync-test".into(),
        key_id: "k1".into(),
        since: "".into(),
    };
    let mut body_b1 = Vec::new();
    req_b1.encode(&mut body_b1).unwrap();

    let resp_b1_raw = ureq::post(&format!("{base_url}/sync/sync"))
        .set("Content-Type", "application/actual-sync")
        .send_bytes(&body_b1)
        .expect("client B pull req");

    let mut resp_b1_bytes = Vec::new();
    resp_b1_raw
        .into_reader()
        .read_to_end(&mut resp_b1_bytes)
        .unwrap();
    let sync_resp_b1 =
        SyncResponse::decode(resp_b1_bytes.as_slice()).expect("decode sync response B1");

    // Client B must receive Message 1!
    assert_eq!(sync_resp_b1.messages.len(), 1);
    assert_eq!(sync_resp_b1.messages[0], env_a);

    // 3. Client B uploads Message 2
    let ts_b = Timestamp::parse("2024-05-15T12:05:00.000Z-0001-0000000000000002").unwrap();
    let msg_b = CrdtMessage::new(ts_b.clone(), "accounts", "acc-checking", "name", "Checking");
    let env_b = msg_b.to_envelope(Some(&key)).unwrap();

    let req_b2 = SyncRequest {
        messages: vec![env_b.clone()],
        file_id: "file-sync-test".into(),
        group_id: "group-sync-test".into(),
        key_id: "k1".into(),
        since: ts_a.to_string(),
    };
    let mut body_b2 = Vec::new();
    req_b2.encode(&mut body_b2).unwrap();

    let _ = ureq::post(&format!("{base_url}/sync/sync"))
        .set("Content-Type", "application/actual-sync")
        .send_bytes(&body_b2)
        .expect("client B upload req");

    // 4. Client A pulls changes since ts_a
    let req_a2 = SyncRequest {
        messages: Vec::new(),
        file_id: "file-sync-test".into(),
        group_id: "group-sync-test".into(),
        key_id: "k1".into(),
        since: ts_a.to_string(),
    };
    let mut body_a2 = Vec::new();
    req_a2.encode(&mut body_a2).unwrap();

    let resp_a2_raw = ureq::post(&format!("{base_url}/sync/sync"))
        .set("Content-Type", "application/actual-sync")
        .send_bytes(&body_a2)
        .expect("client A pull req");

    let mut resp_a2_bytes = Vec::new();
    resp_a2_raw
        .into_reader()
        .read_to_end(&mut resp_a2_bytes)
        .unwrap();
    let sync_resp_a2 = SyncResponse::decode(resp_a2_bytes.as_slice()).unwrap();

    // Client A receives Message 2
    assert_eq!(sync_resp_a2.messages.len(), 1);
    assert_eq!(sync_resp_a2.messages[0], env_b);

    // Decrypt Message 2
    let decoded_msg_b = CrdtMessage::from_envelope(&sync_resp_a2.messages[0], Some(&key)).unwrap();
    assert_eq!(decoded_msg_b, msg_b);
}
