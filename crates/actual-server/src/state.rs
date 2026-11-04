//! Server shared state, SQLite database schema initialization, and configuration.

use crate::error::ServerError;
use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Global shared state accessible across Axum request handlers.
#[derive(Clone)]
pub struct ServerState {
    /// Mutex-wrapped SQLite connection for server metadata and message logs.
    pub db: Arc<Mutex<Connection>>,
    /// Filesystem directory where uploaded `.actual` user backup archives are stored.
    pub data_dir: PathBuf,
    /// Master server password configured via environment or bootstrap.
    pub server_password: Arc<Mutex<Option<String>>>,
}

impl ServerState {
    /// Creates a new `ServerState` backed by a file-based SQLite database in `data_dir`.
    pub fn new(
        data_dir: impl AsRef<Path>,
        initial_password: Option<String>,
    ) -> Result<Self, ServerError> {
        let data_dir = data_dir.as_ref().to_path_buf();
        fs::create_dir_all(&data_dir)?;
        fs::create_dir_all(data_dir.join("user-files"))?;

        let db_path = data_dir.join("server.sqlite");
        let conn = Connection::open(&db_path)?;
        Self::init_schema(&conn)?;

        // If an initial password is provided and no password yet exists in DB, bootstrap it
        let state = Self {
            db: Arc::new(Mutex::new(conn)),
            data_dir,
            server_password: Arc::new(Mutex::new(initial_password)),
        };

        Ok(state)
    }

    /// Creates an in-memory `ServerState` (ideal for unit and integration testing).
    pub fn open_in_memory() -> Result<Self, ServerError> {
        let conn = Connection::open_in_memory()?;
        Self::init_schema(&conn)?;

        let temp_dir = std::env::temp_dir().join(format!("actual-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&temp_dir)?;
        fs::create_dir_all(temp_dir.join("user-files"))?;

        Ok(Self {
            db: Arc::new(Mutex::new(conn)),
            data_dir: temp_dir,
            server_password: Arc::new(Mutex::new(None)),
        })
    }

    /// Initializes server tables if they do not already exist.
    pub fn init_schema(conn: &Connection) -> Result<(), ServerError> {
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;

            CREATE TABLE IF NOT EXISTS users (
                id TEXT PRIMARY KEY,
                password_hash TEXT NOT NULL,
                salt TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS sessions (
                token TEXT PRIMARY KEY,
                user_id TEXT NOT NULL,
                created_at TEXT NOT NULL,
                expires_at INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS files (
                file_id TEXT PRIMARY KEY,
                group_id TEXT NOT NULL,
                name TEXT NOT NULL,
                encrypt_meta TEXT,
                key_id TEXT,
                key_salt TEXT,
                deleted INTEGER DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_id TEXT NOT NULL,
                group_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                is_encrypted INTEGER NOT NULL,
                content BLOB NOT NULL,
                UNIQUE(file_id, group_id, timestamp)
            );

            CREATE INDEX IF NOT EXISTS idx_messages_search
                ON messages(file_id, group_id, timestamp);

            CREATE TABLE IF NOT EXISTS merkle_tries (
                file_id TEXT NOT NULL,
                group_id TEXT NOT NULL,
                trie_json TEXT NOT NULL,
                PRIMARY KEY (file_id, group_id)
            );
            "#,
        )?;
        Ok(())
    }
}
