//! Comprehensive tests for budget archive packaging, `.actual` encryption,
//! backup zip compression, integrity verification, and master key rotation.

use actual_crypto::{
    cipher, BudgetArchive, BudgetMetadata, CryptoError, EncryptedData, KeyRotator,
};
use rusqlite::{params, Connection};
use tempfile::tempdir;

#[test]
fn test_budget_archive_zip_roundtrip() {
    let dummy_db = b"SQLite format 3\0\x04\0\x01\x01\0@  \0\0\0\x01\0\0\0\0";
    let meta = BudgetMetadata {
        id: "bgt-12345".into(),
        budget_name: "Personal Finances 2026".into(),
        group_id: Some("grp-abc".into()),
        cloud_file_id: Some("cloud-file-xyz".into()),
        key_id: None,
        reset_clock: false,
    };

    let archive = BudgetArchive::new(dummy_db.to_vec(), meta.clone());

    // Pack into zip
    let zip_bytes = archive.to_zip().expect("pack zip");
    assert!(!zip_bytes.is_empty());

    // Extract from zip
    let unpacked = BudgetArchive::from_zip(&zip_bytes).expect("unpack zip");
    assert_eq!(unpacked.database, dummy_db);
    assert_eq!(unpacked.metadata, meta);
}

#[test]
fn test_budget_archive_encrypted_actual_roundtrip() {
    let dummy_db = b"SQLite format 3 mock database contents with transactions and categories";
    let meta = BudgetMetadata::new("bgt-vault-001", "Confidential Vault Budget");

    let archive = BudgetArchive::new(dummy_db.to_vec(), meta);

    let (key_a, _salt) =
        KeyRotator::derive_new_key(Some("key-alpha".into()), "master-password-123");

    // Package into .actual encrypted file
    let actual_bytes = archive
        .to_encrypted_actual(&key_a)
        .expect("encrypt archive");
    assert!(!actual_bytes.is_empty());

    // Raw content must be encrypted JSON and not contain cleartext database strings
    let raw_str = String::from_utf8_lossy(&actual_bytes);
    assert!(raw_str.contains("key-alpha"));
    assert!(!raw_str.contains("mock database contents"));

    // Decrypt and unpack
    let restored =
        BudgetArchive::from_encrypted_actual(&actual_bytes, &key_a).expect("decrypt archive");
    assert_eq!(restored.database, dummy_db);
    assert_eq!(restored.metadata.budget_name, "Confidential Vault Budget");
    assert_eq!(restored.metadata.key_id.as_deref(), Some("key-alpha"));
}

#[test]
fn test_budget_archive_wrong_key_and_tampered_payload_rejected() {
    let dummy_db = b"SQLite format 3 test payload";
    let meta = BudgetMetadata::new("bgt-sec", "Secure Budget");
    let archive = BudgetArchive::new(dummy_db.to_vec(), meta);

    let (key_a, _) = KeyRotator::derive_new_key(Some("key-a".into()), "pass-a");
    let (key_b, _) = KeyRotator::derive_new_key(Some("key-b".into()), "pass-b");

    let actual_bytes = archive.to_encrypted_actual(&key_a).unwrap();

    // Decrypting with wrong key must fail
    let err_wrong_key = BudgetArchive::from_encrypted_actual(&actual_bytes, &key_b).unwrap_err();
    assert!(matches!(err_wrong_key, CryptoError::KeyNotFound(_)));

    // Tampered payload
    let mut payload: EncryptedData = serde_json::from_slice(&actual_bytes).unwrap();
    if let Some(first_byte) = payload.value.first_mut() {
        *first_byte ^= 0xFF; // Flip bits in ciphertext
    }
    let tampered_bytes = serde_json::to_vec(&payload).unwrap();

    let err_tampered = BudgetArchive::from_encrypted_actual(&tampered_bytes, &key_a).unwrap_err();
    assert!(matches!(err_tampered, CryptoError::Decryption(_)));
}

#[test]
fn test_key_rotation_archive_re_encryption() {
    let dummy_db = b"SQLite format 3 sensitive customer transactions";
    let meta = BudgetMetadata::new("bgt-rotate", "Pre-rotation Budget");
    let archive = BudgetArchive::new(dummy_db.to_vec(), meta);

    let (key_old, _) = KeyRotator::derive_new_key(Some("key-old-v1".into()), "old-pass-2025");
    let (key_new, _) = KeyRotator::derive_new_key(Some("key-new-v2".into()), "new-pass-2026");

    let encrypted_v1 = archive.to_encrypted_actual(&key_old).expect("encrypt v1");

    // Rotate to key_new
    let encrypted_v2 = KeyRotator::rotate_archive(&encrypted_v1, &key_old, &key_new)
        .expect("rotate archive to new key");

    // Old key must no longer open the archive
    assert!(BudgetArchive::from_encrypted_actual(&encrypted_v2, &key_old).is_err());

    // New key successfully decrypts archive
    let restored =
        BudgetArchive::from_encrypted_actual(&encrypted_v2, &key_new).expect("open with new key");
    assert_eq!(restored.database, dummy_db);
    assert_eq!(restored.metadata.key_id.as_deref(), Some("key-new-v2"));
}

#[test]
fn test_key_rotation_crdt_messages_table() {
    let conn = Connection::open_in_memory().expect("open sqlite");
    conn.execute_batch(
        r#"
        CREATE TABLE messages_crdt (
            id INTEGER PRIMARY KEY,
            timestamp TEXT NOT NULL UNIQUE,
            dataset TEXT NOT NULL,
            row TEXT NOT NULL,
            column TEXT NOT NULL,
            value BLOB NOT NULL
        );
        "#,
    )
    .expect("create crdt table");

    let (key_old, _) = KeyRotator::derive_new_key(Some("key-old".into()), "pass-1");
    let (key_new, _) = KeyRotator::derive_new_key(Some("key-new".into()), "pass-2");

    // Insert 2 encrypted values under key_old and 1 unencrypted plain value
    let val1 = cipher::encrypt(&key_old, b"Amount changed to $150.00").unwrap();
    let val1_bytes = serde_json::to_vec(&val1).unwrap();

    let val2 = cipher::encrypt(&key_old, b"Payee set to Electric Utility").unwrap();
    let val2_bytes = serde_json::to_vec(&val2).unwrap();

    let val3_plain = b"Plain unencrypted value";

    conn.execute(
        "INSERT INTO messages_crdt (id, timestamp, dataset, row, column, value) VALUES (1, 'T1', 'transactions', 'tx1', 'amount', ?1)",
        params![val1_bytes],
    ).unwrap();

    conn.execute(
        "INSERT INTO messages_crdt (id, timestamp, dataset, row, column, value) VALUES (2, 'T2', 'transactions', 'tx1', 'payee', ?1)",
        params![val2_bytes],
    ).unwrap();

    conn.execute(
        "INSERT INTO messages_crdt (id, timestamp, dataset, row, column, value) VALUES (3, 'T3', 'categories', 'c1', 'name', ?1)",
        params![val3_plain.as_slice()],
    ).unwrap();

    // Rotate CRDT messages
    let updated =
        KeyRotator::rotate_crdt_messages(&conn, &key_old, &key_new).expect("rotate crdt table");
    assert_eq!(updated, 2);

    // Verify row 1 is now decryptable under key_new and contains the original plaintext
    let new_val1_bytes: Vec<u8> = conn
        .query_row("SELECT value FROM messages_crdt WHERE id = 1", [], |r| {
            r.get(0)
        })
        .unwrap();
    let new_enc1: EncryptedData = serde_json::from_slice(&new_val1_bytes).unwrap();
    let decrypted1 = cipher::decrypt(&key_new, &new_enc1).expect("decrypt with new key");
    assert_eq!(decrypted1, b"Amount changed to $150.00");

    // Row 3 remains unchanged plain
    let plain_bytes: Vec<u8> = conn
        .query_row("SELECT value FROM messages_crdt WHERE id = 3", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(plain_bytes, val3_plain);
}

#[test]
fn test_archive_filesystem_directory_roundtrip() {
    let temp = tempdir().expect("create temp dir");
    let export_dir = temp.path().join("my_budget");

    let dummy_db = b"SQLite format 3 disk file test";
    let meta = BudgetMetadata::new("bgt-fs-999", "Disk Export Budget");
    let archive = BudgetArchive::new(dummy_db.to_vec(), meta.clone());

    // Write to directory
    archive.write_to_dir(&export_dir).expect("write to dir");
    assert!(export_dir.join("db.sqlite").exists());
    assert!(export_dir.join("metadata.json").exists());

    // Read back from directory
    let loaded = BudgetArchive::from_dir(&export_dir).expect("load from dir");
    assert_eq!(loaded.database, dummy_db);
    assert_eq!(loaded.metadata, meta);
}
