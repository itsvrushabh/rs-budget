//! Error types for CRDT synchronization and wire primitives.

use thiserror::Error;

/// Errors associated with timestamp generation, synchronization, and parsing.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum TimestampError {
    /// Maximum clock drift exceeded between physical time and logical time.
    #[error("maximum clock drift exceeded: logical {logical}, physical {physical}, max drift {max_drift}")]
    ClockDrift {
        logical: u64,
        physical: u64,
        max_drift: u64,
    },

    /// Counter overflow (exceeded 0xFFFF / 65535).
    #[error("timestamp counter overflow")]
    Overflow,

    /// Invalid timestamp format or value.
    #[error("timestamp is not valid: {0}")]
    Invalid(String),

    /// Duplicate node identifier detected during receive.
    #[error("duplicate node identifier: {0}")]
    DuplicateNode(String),
}

/// Errors occurring during sync operations, codec transformations, or Merkle tree diffing.
#[derive(Debug, Error)]
pub enum SyncError {
    /// Timestamp error.
    #[error("timestamp error: {0}")]
    Timestamp(#[from] TimestampError),

    /// Protobuf decoding error.
    #[error("protobuf decode error: {0}")]
    ProtobufDecode(#[from] prost::DecodeError),

    /// Protobuf encoding error.
    #[error("protobuf encode error: {0}")]
    ProtobufEncode(#[from] prost::EncodeError),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// SQLite database error.
    #[cfg(not(target_arch = "wasm32"))]
    #[error("SQLite database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// Cryptography error.
    #[error("cryptography error: {0}")]
    Crypto(#[from] actual_crypto::CryptoError),

    /// Network error during sync transmission.
    #[error("network sync error: {0}")]
    Network(String),

    /// Invalid Merkle tree key or base-3 structure.
    #[error("invalid Merkle tree key: {0}")]
    InvalidMerkleKey(String),

    /// Conflict resolution or data invariant failure.
    #[error("sync conflict error: {0}")]
    Conflict(String),
}
