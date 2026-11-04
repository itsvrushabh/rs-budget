//! SQLite WebAssembly OPFS Virtual File System (VFS) bridge.
//!
//! Maps standard SQLite VFS routines (`xOpen`, `xRead`, `xWrite`, `xTruncate`,
//! `xSync`, `xFileSize`, `xLock`, `xUnlock`, `xClose`) to the Origin Private
//! File System (OPFS) synchronous access handle primitives.

use crate::web::opfs::{OpfsError, OpfsStorageBackend};
use serde::{Deserialize, Serialize};

/// SQLite lock level definitions matching `sqlite3.h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SqliteLockLevel {
    None = 0,
    Shared = 1,
    Reserved = 2,
    Pending = 3,
    Exclusive = 4,
}

/// Simulated file open flags matching SQLite `SQLITE_OPEN_*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SqliteOpenFlags {
    pub read_only: bool,
    pub read_write: bool,
    pub create: bool,
    pub exclusive: bool,
}

impl Default for SqliteOpenFlags {
    fn default() -> Self {
        Self {
            read_only: false,
            read_write: true,
            create: true,
            exclusive: false,
        }
    }
}

/// An active SQLite file opened under the OPFS VFS.
#[derive(Debug)]
pub struct SqliteVfsFile {
    pub path: String,
    pub lock: SqliteLockLevel,
    pub page_size: usize,
    pub is_dirty: bool,
}

/// SQLite OPFS Virtual File System manager.
pub struct SqliteOpfsVfs {
    backend: OpfsStorageBackend,
    open_files: Vec<SqliteVfsFile>,
}

impl SqliteOpfsVfs {
    /// Creates a new SQLite OPFS VFS backed by the given OPFS storage layer.
    pub fn new(backend: OpfsStorageBackend) -> Self {
        Self {
            backend,
            open_files: Vec::new(),
        }
    }

    /// `xOpen`: Opens or creates a database file.
    pub fn open(
        &mut self,
        path: &str,
        flags: SqliteOpenFlags,
        timestamp_unix: u64,
    ) -> Result<usize, OpfsError> {
        let exists = self.backend.exists(path);
        if !exists {
            if !flags.create {
                return Err(OpfsError::NotFound(path.to_string()));
            }
            self.backend.create_file(path, timestamp_unix)?;
        }

        let file = SqliteVfsFile {
            path: path.to_string(),
            lock: SqliteLockLevel::None,
            page_size: 4096, // Default SQLite page size
            is_dirty: false,
        };
        let index = self.open_files.len();
        self.open_files.push(file);
        Ok(index)
    }

    /// `xRead`: Reads `length` bytes at `offset` into buffer.
    pub fn read(&self, file_id: usize, offset: u64, buf: &mut [u8]) -> Result<usize, OpfsError> {
        let file = self
            .open_files
            .get(file_id)
            .ok_or_else(|| OpfsError::NotFound(format!("File handle {file_id}")))?;
        let data = self.backend.get_file(&file.path)?;
        let bytes = data.read_at(offset, buf.len())?;
        buf[..bytes.len()].copy_from_slice(&bytes);
        Ok(bytes.len())
    }

    /// `xWrite`: Writes `buf` at `offset` into the database file.
    pub fn write(
        &mut self,
        file_id: usize,
        offset: u64,
        buf: &[u8],
        timestamp_unix: u64,
    ) -> Result<usize, OpfsError> {
        let file = self
            .open_files
            .get_mut(file_id)
            .ok_or_else(|| OpfsError::NotFound(format!("File handle {file_id}")))?;

        if file.lock < SqliteLockLevel::Reserved {
            return Err(OpfsError::Locked(
                "Cannot write without Reserved or Exclusive lock".into(),
            ));
        }

        let path = file.path.clone();
        let handle = self.backend.get_file_mut(&path)?;
        let written = handle.write_at(offset, buf, timestamp_unix);
        file.is_dirty = true;
        Ok(written)
    }

    /// `xTruncate`: Truncates file to `size` bytes.
    pub fn truncate(
        &mut self,
        file_id: usize,
        size: u64,
        timestamp_unix: u64,
    ) -> Result<(), OpfsError> {
        let file = self
            .open_files
            .get_mut(file_id)
            .ok_or_else(|| OpfsError::NotFound(format!("File handle {file_id}")))?;

        let path = file.path.clone();
        let handle = self.backend.get_file_mut(&path)?;
        handle.truncate(size, timestamp_unix);
        file.is_dirty = true;
        Ok(())
    }

    /// `xSync`: Flushes pending writes through the OPFS SyncAccessHandle.
    pub fn sync(&mut self, file_id: usize) -> Result<(), OpfsError> {
        let file = self
            .open_files
            .get_mut(file_id)
            .ok_or_else(|| OpfsError::NotFound(format!("File handle {file_id}")))?;

        let path = file.path.clone();
        let handle = self.backend.get_file_mut(&path)?;
        handle.flush()?;
        file.is_dirty = false;
        Ok(())
    }

    /// `xFileSize`: Returns database size in bytes.
    pub fn file_size(&self, file_id: usize) -> Result<u64, OpfsError> {
        let file = self
            .open_files
            .get(file_id)
            .ok_or_else(|| OpfsError::NotFound(format!("File handle {file_id}")))?;
        let handle = self.backend.get_file(&file.path)?;
        Ok(handle.size())
    }

    /// `xLock`: Escalates the file lock level.
    pub fn lock(&mut self, file_id: usize, target_lock: SqliteLockLevel) -> Result<(), OpfsError> {
        let file = self
            .open_files
            .get_mut(file_id)
            .ok_or_else(|| OpfsError::NotFound(format!("File handle {file_id}")))?;

        if target_lock < file.lock {
            return Err(OpfsError::Locked(
                "Cannot escalate lock to lower level".into(),
            ));
        }
        file.lock = target_lock;
        Ok(())
    }

    /// `xUnlock`: De-escalates the file lock level.
    pub fn unlock(
        &mut self,
        file_id: usize,
        target_lock: SqliteLockLevel,
    ) -> Result<(), OpfsError> {
        let file = self
            .open_files
            .get_mut(file_id)
            .ok_or_else(|| OpfsError::NotFound(format!("File handle {file_id}")))?;

        if target_lock > file.lock {
            return Err(OpfsError::Locked(
                "Cannot de-escalate lock to higher level".into(),
            ));
        }
        file.lock = target_lock;
        Ok(())
    }

    /// `xClose`: Closes the file handle, flushing any dirty writes.
    pub fn close(&mut self, file_id: usize) -> Result<(), OpfsError> {
        self.sync(file_id)?;
        self.unlock(file_id, SqliteLockLevel::None)?;
        Ok(())
    }

    /// Access underlying OPFS backend.
    pub fn backend(&self) -> &OpfsStorageBackend {
        &self.backend
    }

    /// Mutable access to underlying OPFS backend.
    pub fn backend_mut(&mut self) -> &mut OpfsStorageBackend {
        &mut self.backend
    }
}
