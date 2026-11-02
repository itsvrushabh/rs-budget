# Implementation Roadmap

Track each checklist item in issues or pull requests as work begins. Update this document when the upstream baseline or verified scope changes.

## Phase 0 — Upstream inventory and baseline

- [ ] Pin the upstream commit/release to port.
- [ ] Map upstream packages—including `loot-core`, CRDT, API, server, and web/desktop clients—to Rust responsibilities.
- [ ] Extract upstream SQLite DDL, migration order, protobuf definitions (`sync.proto`), and encryption parameters.
- [ ] Create a feature and workflow matrix from upstream source, docs, tests, and CI.
- [ ] Prepare representative anonymized fixtures and expected results for calculations, migrations, imports, encryption, and sync.

## Phase 1 — Storage, crypto, and wire primitives

- [ ] Implement `actual-storage` using `rusqlite` with WAL mode, matching upstream table schemas and migration versions.
- [ ] Validate direct opening, querying, and updating of existing upstream SQLite budget files with zero data transformation loss.
- [ ] Port cryptographic algorithms (`actual-crypto`): PBKDF2 key derivation and AES-GCM encryption/decryption with cross-implementation known-answer tests.
- [ ] Implement wire primitives in `actual-sync`: Protobuf codecs (via `prost`), Hybrid Logical Clock (HLC) timestamps, and Merkle tree state comparison structures.
- [ ] Verify database transaction safety, crash recovery, and backup/restore behavior on copied test budgets.

## Phase 2 — Core budgeting domain & calculation engine

- [ ] Port domain models (`actual-core`): Accounts, Payees, Categories, Category Groups, Transactions, and Splits.
- [ ] Implement reactive spreadsheet calculation DAG in `actual-core` using a generational arena for cycle-free, incremental envelope calculations, month rollover, carry-forward, and overspending.
- [ ] Port rules evaluation engine (conditions and actions applied during transaction creation/mutation) and schedules.
- [ ] Build a differential test harness: run identical transaction streams through upstream Node.js Actual and Rust `actual-core` to verify identical balances, envelope numbers, and reports.
- [ ] Expose query, mutation, and undo behavior behind typed Rust APIs in `actual-api`.

## Phase 3 — Prioritized CLI, TUI, and import workflows

- [ ] Implement financial format parsers in `actual-import`: QIF, OFX, and CSV workflows verified against upstream format fixtures.
- [ ] Build `actual-cli`: non-interactive commands for scripting, batch imports, account inspection, and automated budget maintenance.
- [ ] Build `actual-tui` as a first-class interactive client using `ratatui`:
  - Connect to local SQLite budget files or remote servers.
  - Navigate months and view category group and envelope balances.
  - Browse accounts and transactions with search and filter capabilities.
  - Inline transaction creation, editing, split entry, and categorization.
  - Edit category allocations and envelope settings with immediate calculation feedback.
- [ ] Test TUI keyboard navigation, terminal resizing, narrow layouts, error reporting, and confirmation for destructive actions.

## Phase 4 — Sync engine & 100% Rust self-hosted server

- [ ] Implement complete client CRDT sync logic in `actual-sync`: message log management, Merkle tree range diffing, and conflict resolution.
- [ ] Implement `actual-server` in 100% Rust using `axum`: user authentication, encryption key exchange, budget metadata, and sync endpoints (`/sync/sync`).
- [ ] Test end-to-end sync interoperability between rs-budget (TUI/CLI) and upstream official Actual clients and servers.
- [ ] Test multi-device concurrency, offline reconnects, conflict resolution, and network retry logic.

## Phase 5 — 100% Rust native graphical clients

- [ ] Prototype and select a pure Rust cross-platform GUI framework (e.g. Slint or Dioxus) evaluated against keyboard navigation, table editing, and accessibility.
- [ ] Build the native Desktop client for Linux, macOS, and Windows with offline-first SQLite support.
- [ ] Build the Web client compiled to WebAssembly (WASM), using `rusqlite` backed by OPFS (Origin Private File System) for browser-local storage.
- [ ] Build the Mobile client for Android and iOS (including iPad).
- [ ] Isolate platform-specific adapters behind `actual-gui` while reusing `actual-core` and `actual-api`.

## Phase 6 — Parity, migration, and cutover

- [ ] Pass full feature matrix and integration verification across CLI, TUI, Server, and Graphical clients.
- [ ] Test cross-version upgrades, automated backups, and recovery on real-world anonymized budgets.
- [ ] Ship an opt-in preview release for community validation and feedback.
- [ ] Finalize documentation, user migration guides, and production release pipeline.
