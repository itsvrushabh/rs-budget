use crate::error::StorageError;
use crate::migrations::{get_applied_migrations, initialize_database, migrate};
use rusqlite::Connection;
use std::path::Path;

pub struct SqliteStore {
    conn: Connection,
}

impl SqliteStore {
    /// Open a SQLite database at the specified path and apply pragmas and migrations.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        let conn = Connection::open(path)?;
        let mut store = Self::from_connection(conn)?;
        store.configure_pragmas(false)?;
        store.init_and_migrate()?;
        Ok(store)
    }

    /// Open an in-memory SQLite database and apply pragmas and migrations.
    pub fn open_in_memory() -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()?;
        let mut store = Self::from_connection(conn)?;
        store.configure_pragmas(true)?;
        store.init_and_migrate()?;
        Ok(store)
    }

    /// Construct a `SqliteStore` from an existing `rusqlite::Connection`.
    pub fn from_connection(conn: Connection) -> Result<Self, StorageError> {
        Ok(Self { conn })
    }

    /// Configure performance and safety PRAGMAs.
    pub fn configure_pragmas(&mut self, is_in_memory: bool) -> Result<(), StorageError> {
        if !is_in_memory {
            let _: String = self
                .conn
                .query_row("PRAGMA journal_mode = WAL;", [], |row| row.get(0))?;
        }
        self.conn.execute_batch(
            r#"
            PRAGMA synchronous = NORMAL;
            PRAGMA foreign_keys = ON;
            PRAGMA busy_timeout = 5000;
            PRAGMA temp_store = MEMORY;
            PRAGMA cache_size = -20000;
            "#,
        )?;
        Ok(())
    }

    /// Initialize the database schema with `init.sql` (if new) and run all pending migrations.
    pub fn init_and_migrate(&mut self) -> Result<Vec<i64>, StorageError> {
        initialize_database(&mut self.conn)?;
        migrate(&mut self.conn)
    }

    /// Access the underlying SQLite connection.
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    /// Access the underlying mutable SQLite connection.
    pub fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    /// Retrieve the list of applied migration IDs.
    pub fn applied_migrations(&self) -> Result<Vec<i64>, StorageError> {
        get_applied_migrations(&self.conn)
    }

    /// Execute a closure inside a database transaction, rolling back automatically on error.
    pub fn with_transaction<T, F>(&mut self, f: F) -> Result<T, StorageError>
    where
        F: FnOnce(&rusqlite::Transaction) -> Result<T, StorageError>,
    {
        let tx = self.conn.transaction()?;
        let res = f(&tx)?;
        tx.commit()?;
        Ok(res)
    }
}
