//! `actual-server` is a 100% Rust self-hosted sync server for Actual Budget,
//! providing CRDT synchronization, user file backups, and zero-knowledge encryption key exchange.

pub mod app;
pub mod auth;
pub mod error;
pub mod routes;
pub mod state;

pub use app::create_app;
pub use error::ServerError;
pub use state::ServerState;
