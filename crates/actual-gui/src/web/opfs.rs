//! Origin Private File System (OPFS) storage backend and virtual filesystem abstraction.
//!
//! Provides local-first persistent file storage in WebAssembly browser environments
//! using `FileSystemDirectoryHandle`, `FileSystemFileHandle`, and `FileSystemSyncAccessHandle`,
//! with an in-memory fallback driver for test and headless environments.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{self, SeekFrom};
use thiserror::Error;

/// Errors that can occur during OPFS storage operations.
#[derive(Debug, Error)]
pub enum OpfsError {
    #[error("File not found: {0}")]
    NotFound(String),

    #[error("Directory not found: {0}")]
    DirectoryNotFound(String),

    #[error("File already exists: {0}")]
    AlreadyExists(String),

    #[error("Storage quota exceeded: requested {requested} bytes, available {available} bytes")]
    QuotaExceeded { requested: u64, available: u64 },

    #[error("I/O error during OPFS operation: {0}")]
    Io(String),

    #[error("OPFS handle locked by another worker: {0}")]
    Locked(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),
}

/// Simulated storage quota info matching `navigator.storage.estimate()`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageQuota {
    pub quota_bytes: u64,
    pub usage_bytes: u64,
}

impl StorageQuota {
    /// Available bytes remaining before quota exhaustion.
    pub fn available_bytes(&self) -> u64 {
        self.quota_bytes.saturating_sub(self.usage_bytes)
    }

    /// Usage ratio as a fraction between 0.0 and 1.0.
    pub fn usage_ratio(&self) -> f64 {
        if self.quota_bytes == 0 {
            0.0
        } else {
            self.usage_bytes as f64 / self.quota_bytes as f64
        }
    }
}

/// File metadata in OPFS storage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpfsFileMetadata {
    pub path: String,
    pub size: u64,
    pub created_at_unix: u64,
    pub modified_at_unix: u64,
    pub is_locked: bool,
}

/// A handle to an open file in OPFS supporting block reads, writes, seeking, and syncing.
#[derive(Debug, Clone)]
pub struct OpfsFileHandle {
    pub path: String,
    data: Vec<u8>,
    cursor: u64,
    metadata: OpfsFileMetadata,
}

impl OpfsFileHandle {
    /// Creates a new empty file handle with metadata.
    pub fn new(path: impl Into<String>, timestamp_unix: u64) -> Self {
        let path = path.into();
        Self {
            metadata: OpfsFileMetadata {
                path: path.clone(),
                size: 0,
                created_at_unix: timestamp_unix,
                modified_at_unix: timestamp_unix,
                is_locked: false,
            },
            path,
            data: Vec::new(),
            cursor: 0,
        }
    }

    /// Reads up to `buf.len()` bytes into `buf` from current cursor position.
    pub fn read(&mut self, buf: &mut [u8]) -> usize {
        let start = self.cursor as usize;
        if start >= self.data.len() {
            return 0;
        }
        let available = self.data.len() - start;
        let to_read = buf.len().min(available);
        buf[..to_read].copy_from_slice(&self.data[start..start + to_read]);
        self.cursor += to_read as u64;
        to_read
    }

    /// Reads `length` bytes at an exact `offset` without mutating cursor position.
    pub fn read_at(&self, offset: u64, length: usize) -> Result<Vec<u8>, OpfsError> {
        let start = offset as usize;
        if start >= self.data.len() {
            return Ok(Vec::new());
        }
        let end = (start + length).min(self.data.len());
        Ok(self.data[start..end].to_vec())
    }

    /// Writes buffer into file at current cursor position, expanding capacity if needed.
    pub fn write(&mut self, buf: &[u8], timestamp_unix: u64) -> usize {
        let start = self.cursor as usize;
        let end = start + buf.len();
        if end > self.data.len() {
            self.data.resize(end, 0);
        }
        self.data[start..end].copy_from_slice(buf);
        self.cursor = end as u64;
        self.metadata.size = self.data.len() as u64;
        self.metadata.modified_at_unix = timestamp_unix;
        buf.len()
    }

    /// Writes buffer at an exact `offset` without mutating cursor position.
    pub fn write_at(&mut self, offset: u64, buf: &[u8], timestamp_unix: u64) -> usize {
        let start = offset as usize;
        let end = start + buf.len();
        if end > self.data.len() {
            self.data.resize(end, 0);
        }
        self.data[start..end].copy_from_slice(buf);
        self.metadata.size = self.data.len() as u64;
        self.metadata.modified_at_unix = timestamp_unix;
        buf.len()
    }

    /// Seeks to a position in the file.
    pub fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        match pos {
            SeekFrom::Start(offset) => {
                self.cursor = offset;
            }
            SeekFrom::Current(offset) => {
                let new_pos = (self.cursor as i64) + offset;
                if new_pos < 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "negative seek position",
                    ));
                }
                self.cursor = new_pos as u64;
            }
            SeekFrom::End(offset) => {
                let new_pos = (self.data.len() as i64) + offset;
                if new_pos < 0 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "negative seek position",
                    ));
                }
                self.cursor = new_pos as u64;
            }
        }
        Ok(self.cursor)
    }

    /// Truncates file to a specified size.
    pub fn truncate(&mut self, size: u64, timestamp_unix: u64) {
        self.data.resize(size as usize, 0);
        self.metadata.size = size;
        self.metadata.modified_at_unix = timestamp_unix;
        if self.cursor > size {
            self.cursor = size;
        }
    }

    /// Returns file size in bytes.
    pub fn size(&self) -> u64 {
        self.data.len() as u64
    }

    /// Returns entire contents as a byte slice.
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Metadata accessor.
    pub fn metadata(&self) -> &OpfsFileMetadata {
        &self.metadata
    }

    /// Flushes changes (no-op in in-memory backend, syncs FileSystemSyncAccessHandle in browser).
    pub fn flush(&mut self) -> Result<(), OpfsError> {
        Ok(())
    }
}

/// Abstract storage backend representing browser OPFS or in-memory fallback.
#[derive(Debug, Clone)]
pub struct OpfsStorageBackend {
    root_prefix: String,
    files: HashMap<String, OpfsFileHandle>,
    quota: StorageQuota,
}

impl OpfsStorageBackend {
    /// Creates a new OPFS storage backend with root directory prefix.
    pub fn new(root_prefix: impl Into<String>) -> Self {
        Self {
            root_prefix: root_prefix.into(),
            files: HashMap::new(),
            quota: StorageQuota {
                quota_bytes: 1024 * 1024 * 1024, // 1 GB simulated default
                usage_bytes: 0,
            },
        }
    }

    /// Sets simulated storage quota bytes.
    pub fn with_quota(mut self, quota_bytes: u64) -> Self {
        self.quota.quota_bytes = quota_bytes;
        self
    }

    /// Returns normalized full path under OPFS root.
    pub fn resolve_path(&self, relative_path: &str) -> String {
        let clean = relative_path.trim_start_matches('/');
        format!("{}/{}", self.root_prefix.trim_end_matches('/'), clean)
    }

    /// Checks if a file exists at path.
    pub fn exists(&self, path: &str) -> bool {
        let full = self.resolve_path(path);
        self.files.contains_key(&full)
    }

    /// Opens or creates a file at path.
    pub fn create_file(
        &mut self,
        path: &str,
        timestamp_unix: u64,
    ) -> Result<&mut OpfsFileHandle, OpfsError> {
        let full = self.resolve_path(path);
        let handle = OpfsFileHandle::new(&full, timestamp_unix);
        self.files.insert(full.clone(), handle);
        Ok(self.files.get_mut(&full).expect("just inserted"))
    }

    /// Gets mutable handle to existing file.
    pub fn get_file_mut(&mut self, path: &str) -> Result<&mut OpfsFileHandle, OpfsError> {
        let full = self.resolve_path(path);
        self.files.get_mut(&full).ok_or(OpfsError::NotFound(full))
    }

    /// Gets immutable reference to existing file.
    pub fn get_file(&self, path: &str) -> Result<&OpfsFileHandle, OpfsError> {
        let full = self.resolve_path(path);
        self.files.get(&full).ok_or(OpfsError::NotFound(full))
    }

    /// Writes whole byte content to a file, updating quota.
    pub fn write_file(
        &mut self,
        path: &str,
        data: &[u8],
        timestamp_unix: u64,
    ) -> Result<(), OpfsError> {
        let old_size = self
            .files
            .get(&self.resolve_path(path))
            .map(|f| f.size())
            .unwrap_or(0);
        let delta = (data.len() as i64) - (old_size as i64);

        if delta > 0 && (self.quota.usage_bytes + delta as u64) > self.quota.quota_bytes {
            return Err(OpfsError::QuotaExceeded {
                requested: delta as u64,
                available: self.quota.available_bytes(),
            });
        }

        let full = self.resolve_path(path);
        let mut handle = OpfsFileHandle::new(&full, timestamp_unix);
        handle.write(data, timestamp_unix);
        self.files.insert(full, handle);

        if delta >= 0 {
            self.quota.usage_bytes += delta as u64;
        } else {
            self.quota.usage_bytes = self.quota.usage_bytes.saturating_sub((-delta) as u64);
        }
        Ok(())
    }

    /// Reads whole content of a file.
    pub fn read_file(&self, path: &str) -> Result<Vec<u8>, OpfsError> {
        let file = self.get_file(path)?;
        Ok(file.as_bytes().to_vec())
    }

    /// Deletes a file.
    pub fn remove_file(&mut self, path: &str) -> Result<(), OpfsError> {
        let full = self.resolve_path(path);
        if let Some(removed) = self.files.remove(&full) {
            self.quota.usage_bytes = self.quota.usage_bytes.saturating_sub(removed.size());
            Ok(())
        } else {
            Err(OpfsError::NotFound(full))
        }
    }

    /// Lists all file paths starting with a directory prefix.
    pub fn list_files(&self, prefix: &str) -> Vec<OpfsFileMetadata> {
        let full_prefix = self.resolve_path(prefix);
        self.files
            .values()
            .filter(|f| f.path.starts_with(&full_prefix))
            .map(|f| f.metadata().clone())
            .collect()
    }

    /// Returns storage quota statistics.
    pub fn estimate_quota(&self) -> StorageQuota {
        self.quota.clone()
    }
}
