//! `actual-sync` implements CRDT synchronization, Hybrid Logical Clocks (HLC),
//! Radix Merkle tries, change tracking, Last-Write-Wins conflict resolution,
//! offline queues, and protobuf wire protocols matching upstream Actual Budget.

#[cfg(not(target_arch = "wasm32"))]
pub mod change_tracker;
#[cfg(not(target_arch = "wasm32"))]
pub mod client;
pub mod crdt;
pub mod error;
pub mod hash;
pub mod merkle;
pub mod proto;
#[cfg(not(target_arch = "wasm32"))]
pub mod queue;
pub mod timestamp;

#[cfg(not(target_arch = "wasm32"))]
pub use change_tracker::{ApplyResult, CrdtTracker};
#[cfg(not(target_arch = "wasm32"))]
pub use client::{SyncClient, SyncCycleResult};
pub use crdt::CrdtMessage;
pub use error::{SyncError, TimestampError};
pub use hash::murmur3_32;
pub use merkle::{
    build as build_merkle, diff as diff_merkle, from_base3, key_to_timestamp, to_base3, TrieNode,
};
pub use proto::{EncryptedData, Message, MessageEnvelope, SyncRequest, SyncResponse};
#[cfg(not(target_arch = "wasm32"))]
pub use queue::{BackoffStrategy, OfflineQueue, SyncStatus};
pub use timestamp::{
    make_client_id, HlcClock, Timestamp, DEFAULT_MAX_DRIFT, MAX_COUNTER, MAX_MILLIS,
};
