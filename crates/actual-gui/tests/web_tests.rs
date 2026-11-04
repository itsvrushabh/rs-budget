//! Integration tests for WebAssembly client with Origin Private File System (OPFS) persistence,
//! SQLite VFS bridge, web sync manager, and PWA manifest generation.

use actual_core::cents::Cents;
use actual_core::models::Transaction;
use actual_gui::web::{
    IndexHtmlBuilder, OnlineState, OpfsError, OpfsStorageBackend, ServiceWorkerBuilder,
    SqliteLockLevel, SqliteOpenFlags, SqliteOpfsVfs, WebApp, WebManifest, WebRoute, WebSyncBridge,
    WebSyncConfig, WebSyncState,
};
use actual_sync::Timestamp;
use std::io::SeekFrom;

#[test]
fn test_opfs_storage_read_write_seek_truncate() {
    let mut storage = OpfsStorageBackend::new("/actual-storage");
    assert!(!storage.exists("budgets/test.sqlite"));

    // Create file
    let file = storage
        .create_file("budgets/test.sqlite", 1700000000)
        .unwrap();
    assert_eq!(file.size(), 0);

    // Write data
    let written = file.write(b"Hello OPFS SQLite", 1700000001);
    assert_eq!(written, 17);
    assert_eq!(file.size(), 17);

    // Seek and read
    file.seek(SeekFrom::Start(6)).unwrap();
    let mut buf = [0u8; 4];
    let read_count = file.read(&mut buf);
    assert_eq!(read_count, 4);
    assert_eq!(&buf, b"OPFS");

    // Exact offset read without moving cursor
    let slice = file.read_at(11, 6).unwrap();
    assert_eq!(&slice, b"SQLite");

    // Truncate
    file.truncate(5, 1700000002);
    assert_eq!(file.size(), 5);
    assert_eq!(file.as_bytes(), b"Hello");
}

#[test]
fn test_opfs_storage_quota_enforcement() {
    // Set 100-byte quota limit
    let mut storage = OpfsStorageBackend::new("/actual-storage").with_quota(100);
    assert_eq!(storage.estimate_quota().quota_bytes, 100);
    assert_eq!(storage.estimate_quota().available_bytes(), 100);

    // 50 bytes should succeed
    let data_50 = vec![42u8; 50];
    storage
        .write_file("file1.bin", &data_50, 1700000000)
        .unwrap();
    assert_eq!(storage.estimate_quota().usage_bytes, 50);
    assert_eq!(storage.estimate_quota().available_bytes(), 50);

    // Another 60 bytes should exceed quota (50 + 60 = 110 > 100)
    let data_60 = vec![99u8; 60];
    let err = storage
        .write_file("file2.bin", &data_60, 1700000001)
        .unwrap_err();
    match err {
        OpfsError::QuotaExceeded {
            requested,
            available,
        } => {
            assert_eq!(requested, 60);
            assert_eq!(available, 50);
        }
        other => panic!("Expected QuotaExceeded, got {other:?}"),
    }

    // Removing file releases quota
    storage.remove_file("file1.bin").unwrap();
    assert_eq!(storage.estimate_quota().usage_bytes, 0);
    assert_eq!(storage.estimate_quota().available_bytes(), 100);
}

#[test]
fn test_sqlite_opfs_vfs_lifecycle() {
    let storage = OpfsStorageBackend::new("/actual-opfs");
    let mut vfs = SqliteOpfsVfs::new(storage);

    // Open file
    let file_id = vfs
        .open("budget.db", SqliteOpenFlags::default(), 1700000000)
        .unwrap();

    // Must lock before writing
    let write_err = vfs.write(file_id, 0, b"sqlite header", 1700000000);
    assert!(write_err.is_err());

    // Escalate to Reserved lock
    vfs.lock(file_id, SqliteLockLevel::Reserved).unwrap();

    // Write database page header
    let page_header = b"SQLite format 3\0";
    let written = vfs.write(file_id, 0, page_header, 1700000001).unwrap();
    assert_eq!(written, page_header.len());

    // Flush and sync
    vfs.sync(file_id).unwrap();
    assert_eq!(vfs.file_size(file_id).unwrap(), page_header.len() as u64);

    // Read back
    let mut read_buf = vec![0u8; page_header.len()];
    let read_count = vfs.read(file_id, 0, &mut read_buf).unwrap();
    assert_eq!(read_count, page_header.len());
    assert_eq!(&read_buf, page_header);

    // De-escalate lock and close
    vfs.unlock(file_id, SqliteLockLevel::Shared).unwrap();
    vfs.close(file_id).unwrap();
}

#[test]
fn test_web_sync_bridge_online_offline_transitions() {
    let mut bridge = WebSyncBridge::new(WebSyncConfig {
        server_url: "https://sync.actualbudget.org".into(),
        sync_id: "budget-uuid-123".into(),
        auth_token: Some("secret-token".into()),
        auto_sync_interval_secs: 15,
    });

    assert_eq!(bridge.network_state, OnlineState::Online);
    assert_eq!(bridge.sync_state, WebSyncState::Idle);

    // Go offline
    bridge.set_online_state(OnlineState::Offline);
    assert_eq!(bridge.network_state, OnlineState::Offline);

    // Enqueue changes while offline
    let ts = Timestamp::new(1700000000000, 0, "node000000000001");
    let msg1 = actual_sync::CrdtMessage::new(ts, "categories", "cat-1", "name", "Dining Out");
    bridge.enqueue_change(msg1);
    assert_eq!(bridge.pending_queue_size(), 1);
    assert_eq!(bridge.sync_state, WebSyncState::OfflineQueued);

    // Flushing while offline must fail
    let flush_res = bridge.flush_offline_queue(1700000005);
    assert!(flush_res.is_err());

    // Build sync request to verify protobuf encoding
    let sync_req = bridge.build_sync_request("file-1", "group-1");
    assert_eq!(sync_req.messages.len(), 1);
    assert_eq!(sync_req.file_id, "file-1");
    assert_eq!(sync_req.group_id, "group-1");

    // Go online
    bridge.set_online_state(OnlineState::Online);
    assert_eq!(bridge.network_state, OnlineState::Online);
    assert_eq!(bridge.sync_state, WebSyncState::Syncing);

    // Flush offline queue
    let flushed = bridge.flush_offline_queue(1700000010).unwrap();
    assert_eq!(flushed, 1);
    assert_eq!(bridge.pending_queue_size(), 0);
    assert_eq!(bridge.sync_state, WebSyncState::Synced);
    assert_eq!(bridge.last_sync_timestamp(), Some(1700000010));
}

#[test]
fn test_pwa_manifest_and_service_worker() {
    let manifest = WebManifest::default();
    let json = manifest.to_json().unwrap();

    assert!(json.contains("\"name\": \"Actual Budget\""));
    assert!(json.contains("\"short_name\": \"Actual\""));
    assert!(json.contains("\"display\": \"standalone\""));
    assert!(json.contains("\"background_color\": \"#18181b\""));
    assert!(json.contains("192x192"));
    assert!(json.contains("512x512"));

    let sw = ServiceWorkerBuilder::default().generate_script();
    assert!(sw.contains("actual-budget-v1"));
    assert!(sw.contains("CACHE_NAME"));
    assert!(sw.contains("addEventListener(\"install\""));
    assert!(sw.contains("addEventListener(\"activate\""));
    assert!(sw.contains("addEventListener(\"fetch\""));
    assert!(sw.contains("/sync/"));

    let html = IndexHtmlBuilder::generate_html("Actual Budget", "#2563eb");
    assert!(html.contains("<canvas id=\"budget-canvas\"></canvas>"));
    assert!(html.contains("manifest.webmanifest"));
    assert!(html.contains("serviceWorker.register"));
    assert!(html.contains("navigator.storage.getDirectory"));
}

#[test]
fn test_web_app_budget_lifecycle() {
    let mut app = WebApp::new("/actual-web-data", WebSyncConfig::default());
    assert_eq!(app.current_route, WebRoute::Budget);

    // Create budget
    app.create_budget("personal-2026", "Personal 2026", 1700000000)
        .unwrap();
    assert!(app.active_budget.is_some());
    assert_eq!(app.category_groups.len(), 1);
    assert_eq!(app.categories.len(), 1);

    // Add account
    app.add_account("acc-checking", "Checking Account", false);
    assert_eq!(app.accounts.len(), 1);
    assert_eq!(app.sync_bridge.pending_queue_size(), 1);

    // Add transaction
    let date = chrono::NaiveDate::from_ymd_opt(2026, 11, 4).unwrap();
    let tx = Transaction::new("tx-100", "acc-checking", date, Cents(5000));
    app.add_transaction(tx);
    assert_eq!(app.transactions.len(), 1);
    assert_eq!(app.total_balance(), Cents(5000));
    assert_eq!(app.sync_bridge.pending_queue_size(), 2);

    // Navigate to transactions view
    app.navigate_to(WebRoute::Transactions);
    assert_eq!(app.current_route, WebRoute::Transactions);

    // Save budget to OPFS
    app.save_active_budget(1700000005).unwrap();
    assert!(app.storage.exists("budgets/personal-2026.actual"));

    // Reload budget into new app instance
    let mut reloaded_app = WebApp::new("/actual-web-data", WebSyncConfig::default());
    reloaded_app.storage = app.storage.clone();
    reloaded_app
        .load_budget("budgets/personal-2026.actual", 1700000006)
        .unwrap();

    assert_eq!(reloaded_app.category_groups.len(), 1);
    assert_eq!(reloaded_app.categories.len(), 1);
    assert_eq!(reloaded_app.accounts.len(), 1);
    assert_eq!(reloaded_app.accounts[0].name, "Checking Account");
}

#[test]
fn test_web_app_network_reconnect_sync() {
    let mut app = WebApp::new("/actual-web-data", WebSyncConfig::default());
    app.create_budget("sync-test", "Sync Test", 1700000000)
        .unwrap();

    // Go offline
    app.update_network_status(OnlineState::Offline, 1700000001);
    assert_eq!(app.sync_status(), WebSyncState::OfflineQueued);

    // Add changes while offline
    app.add_account("acc-savings", "Savings Account", false);
    assert_eq!(app.sync_bridge.pending_queue_size(), 1);
    assert_eq!(app.sync_status(), WebSyncState::OfflineQueued);

    // Restore network
    app.update_network_status(OnlineState::Online, 1700000005);
    assert_eq!(app.sync_bridge.pending_queue_size(), 0);
    assert_eq!(app.sync_status(), WebSyncState::Synced);
}
