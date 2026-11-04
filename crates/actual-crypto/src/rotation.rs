//! Encryption key rotation, user password change re-encryption, and CRDT log migration.

use crate::archive::BudgetArchive;
use crate::cipher::{decrypt, encrypt, random_bytes, EncryptedData};
use crate::error::CryptoError;
use crate::key::Key;
use base64::prelude::*;
use rusqlite::{params, Connection};

/// Utility for rotating master encryption keys across budget archives and database tables.
pub struct KeyRotator;

impl KeyRotator {
    /// Generates a new `Key` and fresh cryptographically secure random salt for a password.
    #[must_use]
    pub fn derive_new_key(key_id: Option<String>, password: &str) -> (Key, String) {
        let salt_bytes = random_bytes(16);
        let salt_b64 = BASE64_STANDARD.encode(&salt_bytes);
        let key = Key::from_password(key_id, password, &salt_b64)
            .expect("key derivation with random salt cannot fail");
        (key, salt_b64)
    }

    /// Re-encrypts an existing `.actual` encrypted budget archive with a new master key.
    pub fn rotate_archive(
        actual_bytes: &[u8],
        old_key: &Key,
        new_key: &Key,
    ) -> Result<Vec<u8>, CryptoError> {
        let mut archive = BudgetArchive::from_encrypted_actual(actual_bytes, old_key)?;
        archive.metadata.key_id = Some(new_key.id().to_string());
        archive.to_encrypted_actual(new_key)
    }

    /// Re-encrypts stored encrypted values in `messages_crdt` from `old_key` to `new_key`.
    ///
    /// Inspects the `value` blob in `messages_crdt`. If it is formatted as an `EncryptedData` JSON payload,
    /// it decrypts the value with `old_key`, re-encrypts with `new_key`, and updates the database row.
    pub fn rotate_crdt_messages(
        conn: &Connection,
        old_key: &Key,
        new_key: &Key,
    ) -> Result<usize, CryptoError> {
        let mut stmt = conn.prepare("SELECT id, value FROM messages_crdt ORDER BY id ASC")?;
        let rows = stmt.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let val_bytes: Vec<u8> = row.get(1)?;
            Ok((id, val_bytes))
        })?;

        let mut to_update = Vec::new();

        for r in rows {
            let (id, val_bytes) = r?;
            // Check if value is serialized EncryptedData JSON
            if let Ok(enc_data) = serde_json::from_slice::<EncryptedData>(&val_bytes) {
                // If encrypted under old_key, re-encrypt under new_key
                if enc_data.meta.key_id == old_key.id() || enc_data.meta.key_id.is_empty() {
                    let decrypted = decrypt(old_key, &enc_data)?;
                    let re_encrypted = encrypt(new_key, &decrypted)?;
                    let new_bytes = serde_json::to_vec(&re_encrypted)?;
                    to_update.push((id, new_bytes));
                }
            }
        }

        let updated_count = to_update.len();
        for (id, new_val) in to_update {
            conn.execute(
                "UPDATE messages_crdt SET value = ?1 WHERE id = ?2",
                params![new_val, id],
            )?;
        }

        Ok(updated_count)
    }
}
