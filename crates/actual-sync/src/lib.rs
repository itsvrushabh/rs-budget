//! `actual-sync` implements CRDT synchronization, Hybrid Logical Clocks (HLC),
//! Merkle trees, and protobuf wire protocols matching upstream Actual Budget.

pub mod error;
pub mod hash;
pub mod merkle;
pub mod proto;
pub mod timestamp;

pub use error::{SyncError, TimestampError};
pub use hash::murmur3_32;
pub use merkle::{
    build as build_merkle, diff as diff_merkle, from_base3, key_to_timestamp, to_base3, TrieNode,
};
pub use proto::{EncryptedData, Message, MessageEnvelope, SyncRequest, SyncResponse};
pub use timestamp::{
    make_client_id, HlcClock, Timestamp, DEFAULT_MAX_DRIFT, MAX_COUNTER, MAX_MILLIS,
};
