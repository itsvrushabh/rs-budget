//! Budget database archive packaging, backup zip compression, and authenticated encryption.

use crate::cipher::{decrypt, encrypt, EncryptedData};
use crate::error::CryptoError;
use crate::key::Key;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Cursor, Read, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

/// Metadata accompanying a budget file matching upstream Actual Budget conventions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BudgetMetadata {
    /// Local internal budget ID.
    pub id: String,
    /// User-friendly budget display name.
    pub budget_name: String,
    /// Synchronization group ID if linked to a sync server.
    #[serde(default)]
    pub group_id: Option<String>,
    /// Cloud storage file ID if backed by remote storage.
    #[serde(default)]
    pub cloud_file_id: Option<String>,
    /// Active encryption key ID if budget is protected.
    #[serde(default)]
    pub key_id: Option<String>,
    /// Flag indicating whether HLC clock should reset upon import.
    #[serde(default)]
    pub reset_clock: bool,
}

impl BudgetMetadata {
    /// Constructs a basic metadata descriptor with a budget name.
    #[must_use]
    pub fn new(id: impl Into<String>, budget_name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            budget_name: budget_name.into(),
            group_id: None,
            cloud_file_id: None,
            key_id: None,
            reset_clock: false,
        }
    }
}

/// An in-memory representation of an uncompressed or decrypted budget package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetArchive {
    /// Raw bytes of the SQLite database (`db.sqlite`).
    pub database: Vec<u8>,
    /// Budget descriptor metadata (`metadata.json`).
    pub metadata: BudgetMetadata,
}

impl BudgetArchive {
    /// Creates a new `BudgetArchive` from database bytes and metadata.
    #[must_use]
    pub fn new(database: Vec<u8>, metadata: BudgetMetadata) -> Self {
        Self { database, metadata }
    }

    /// Reads a budget package from a filesystem directory containing `db.sqlite` and `metadata.json`.
    pub fn from_dir(dir: impl AsRef<Path>) -> Result<Self, CryptoError> {
        let dir = dir.as_ref();
        let db_path = dir.join("db.sqlite");
        let meta_path = dir.join("metadata.json");

        let database = fs::read(&db_path)?;
        let meta_str = fs::read_to_string(&meta_path)?;
        let metadata: BudgetMetadata = serde_json::from_str(&meta_str)?;

        Ok(Self { database, metadata })
    }

    /// Writes the unpacked archive to a filesystem directory (`db.sqlite` and `metadata.json`).
    pub fn write_to_dir(&self, dir: impl AsRef<Path>) -> Result<(), CryptoError> {
        let dir = dir.as_ref();
        fs::create_dir_all(dir)?;

        fs::write(dir.join("db.sqlite"), &self.database)?;
        let meta_str = serde_json::to_string_pretty(&self.metadata)?;
        fs::write(dir.join("metadata.json"), meta_str)?;

        Ok(())
    }

    /// Packs the budget into a standard `.zip` backup archive.
    pub fn to_zip(&self) -> Result<Vec<u8>, CryptoError> {
        let mut buffer = Cursor::new(Vec::new());
        {
            let mut zip = ZipWriter::new(&mut buffer);
            let options = SimpleFileOptions::default()
                .compression_method(CompressionMethod::Deflated)
                .unix_permissions(0o644);

            // 1. Write metadata.json
            zip.start_file("metadata.json", options)?;
            let meta_json = serde_json::to_string_pretty(&self.metadata)?;
            zip.write_all(meta_json.as_bytes())?;

            // 2. Write db.sqlite
            zip.start_file("db.sqlite", options)?;
            zip.write_all(&self.database)?;

            zip.finish()?;
        }

        Ok(buffer.into_inner())
    }

    /// Extracts a `BudgetArchive` from standard `.zip` archive bytes.
    pub fn from_zip(zip_bytes: &[u8]) -> Result<Self, CryptoError> {
        let reader = Cursor::new(zip_bytes);
        let mut archive = ZipArchive::new(reader)?;

        let mut database = None;
        let mut metadata = None;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let name = file.name().to_string();

            if name.ends_with("metadata.json") {
                let mut content = String::new();
                file.read_to_string(&mut content)?;
                let meta: BudgetMetadata = serde_json::from_str(&content)?;
                metadata = Some(meta);
            } else if name.ends_with("db.sqlite") || name.ends_with(".sqlite") {
                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes)?;
                database = Some(bytes);
            }
        }

        let database = database.ok_or_else(|| {
            CryptoError::Archive("Zip archive missing 'db.sqlite' file".to_string())
        })?;

        let metadata = metadata.ok_or_else(|| {
            CryptoError::Archive("Zip archive missing 'metadata.json' file".to_string())
        })?;

        Ok(Self { database, metadata })
    }

    /// Packages the budget into an AES-256-GCM authenticated `.actual` encrypted archive.
    pub fn to_encrypted_actual(&self, key: &Key) -> Result<Vec<u8>, CryptoError> {
        // Update key_id in metadata
        let mut meta_clone = self.metadata.clone();
        meta_clone.key_id = Some(key.id().to_string());
        let updated_archive = Self {
            database: self.database.clone(),
            metadata: meta_clone,
        };

        let zip_bytes = updated_archive.to_zip()?;
        let encrypted_payload = encrypt(key, &zip_bytes)?;
        let json_bytes = serde_json::to_vec_pretty(&encrypted_payload)?;
        Ok(json_bytes)
    }

    /// Decrypts and unpacks a `.actual` encrypted archive using the specified key.
    pub fn from_encrypted_actual(actual_bytes: &[u8], key: &Key) -> Result<Self, CryptoError> {
        let encrypted_payload: EncryptedData = serde_json::from_slice(actual_bytes)?;

        // Verify key_id matches if present
        if !encrypted_payload.meta.key_id.is_empty() && encrypted_payload.meta.key_id != key.id() {
            return Err(CryptoError::KeyNotFound(format!(
                "Archive encrypted with key '{}', but provided key is '{}'",
                encrypted_payload.meta.key_id,
                key.id()
            )));
        }

        let decrypted_zip_bytes = decrypt(key, &encrypted_payload)?;
        Self::from_zip(&decrypted_zip_bytes)
    }
}
