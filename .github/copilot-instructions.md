# GitHub Copilot Code Review Instructions for `rs-budget`

You are reviewing pull requests for `rs-budget`, a 100% Rust rewrite of Actual Budget preserving local-first envelope budgeting, SQLite budget files, and CRDT synchronization.

## Critical Review Guidelines

### 1. Parity and Data Safety
- **Behavior Baseline:** Actual Budget `v26.11.0` (commit `1766241e3f4b3c578e424c4fa15f1eb83c0da17d`).
- **Storage:** Authoritative local storage is SQLite via `rusqlite` with WAL mode enabled.
- **Data Safety:** Never allow unversioned schema changes, silent data mutations, or data loss. All SQLite operations modifying schema must match upstream migration numbers.
- **Monetary Precision:** All currency amounts must be represented in integer cents (`i64` / `Cents`), never floating-point numbers.

### 2. Rust Code Quality
- Zero warnings: code must pass `cargo clippy --workspace --all-targets --all-features -- -D warnings` and `cargo fmt --check`.
- Proper error handling: use `thiserror` in library crates (`actual-core`, `actual-storage`, etc.) and avoid panics (`unwrap()`, `expect()`) in runtime code paths unless strictly proven invariant in tests.
- Audit any `unsafe` code blocks: require explicit safety comments.

### 3. Repository Standards
- Every Pull Request must link to an associated GitHub Issue ID (`Closes #<id>` or `Fixes #<id>`).
- Unit and integration tests must accompany new logic.
