# Implementation Roadmap

Track each checklist item in issues or pull requests as work begins. Update this document when the upstream baseline or verified scope changes.

## Phase 0 — Upstream inventory and baseline

- [x] Pin upstream commit/release, map packages, and generate test fixtures ([#5](https://github.com/itsvrushabh/rs-budget/issues/5))
  - [x] Pin the upstream commit/release to port in documentation (`v26.11.0` commit `1766241e3f4b3c578e424c4fa15f1eb83c0da17d`).
  - [x] Map upstream packages—including `loot-core`, CRDT, API, server, and web/desktop clients—to Rust responsibilities.
  - [x] Extract upstream SQLite DDL, migration order, protobuf definitions (`sync.proto`), and encryption parameters.
  - [x] Create a feature and workflow matrix from upstream source, docs, tests, and CI.
  - [x] Prepare representative anonymized fixtures and expected results for calculations, migrations, imports, encryption, and sync.

## Phase 1 — Storage, crypto, and wire primitives

- [x] Implement `actual-storage` using `rusqlite` with WAL mode and migration runner ([#6](https://github.com/itsvrushabh/rs-budget/issues/6))
  - [x] Match upstream table schemas and migration versions.
  - [x] Validate direct opening, querying, and updating of existing upstream SQLite budget files with zero data transformation loss.
  - [x] Verify database transaction safety, crash recovery, and backup/restore behavior on copied test budgets.
- [x] Port cryptographic algorithms in `actual-crypto` ([#7](https://github.com/itsvrushabh/rs-budget/issues/7))
  - [x] Port PBKDF2 key derivation and AES-GCM encryption/decryption.
  - [x] Add cross-implementation known-answer tests against upstream Node.js output.
- [x] Implement wire primitives and CRDT structures in `actual-sync` ([#8](https://github.com/itsvrushabh/rs-budget/issues/8))
  - [x] Generate Protobuf codecs via `prost` from `sync.proto`.
  - [x] Implement Hybrid Logical Clock (HLC) timestamps and Merkle tree state comparison structures.

## Phase 2 — Core budgeting domain & calculation engine

- [x] Port domain models in `actual-core` ([#9](https://github.com/itsvrushabh/rs-budget/issues/9))
  - [x] Port Accounts, Payees, Categories, Category Groups, Transactions, and Splits.
  - [x] Implement integer cents monetary precision and invariant validations.
- [x] Implement reactive spreadsheet calculation DAG in `actual-core` ([#10](https://github.com/itsvrushabh/rs-budget/issues/10))
  - [x] Build dependency graph with a generational arena for cycle-free incremental envelope calculations.
  - [x] Implement month rollover, carry-forward, and overspending calculation logic.
- [x] Port rules evaluation engine and recurring schedules in `actual-core` ([#11](https://github.com/itsvrushabh/rs-budget/issues/11))
  - [x] Implement rule conditions and actions evaluated during transaction creation/mutation.
  - [x] Implement recurring schedule execution and upcoming transaction generation.
- [ ] Build differential testing harness comparing rs-budget against Node.js Actual ([#12](https://github.com/itsvrushabh/rs-budget/issues/12))
  - [ ] Run identical transaction streams through upstream Node.js Actual and Rust `actual-core`.
  - [ ] Assert identical balances, envelope numbers, and reports.
- [ ] Expose query, mutation, and undo behavior behind typed APIs in `actual-api` ([#13](https://github.com/itsvrushabh/rs-budget/issues/13))
  - [ ] Implement command/mutation interfaces and undo/redo state restoration.
- [ ] Implement financial reporting, net worth, and cash flow analytics engine in `actual-core` ([#37](https://github.com/itsvrushabh/rs-budget/issues/37))
  - [ ] Calculate net worth progression (assets vs liabilities) over configurable timeframes.
  - [ ] Generate monthly cash flow summaries, spending breakdowns by category group, and custom filtered analytics.

## Phase 3 — Prioritized CLI, TUI, and import workflows

- [ ] Implement financial format parsers in `actual-import` ([#14](https://github.com/itsvrushabh/rs-budget/issues/14))
  - [ ] Port QIF, OFX, and CSV workflows verified against upstream format fixtures.
  - [ ] Implement transaction deduplication and payee matching.
- [ ] Build non-interactive CLI in `actual-cli` ([#15](https://github.com/itsvrushabh/rs-budget/issues/15))
  - [ ] Scripting commands, batch imports, account inspection, and automated maintenance.
  - [ ] Machine-readable JSON output mode (`--json`).
- [ ] Build interactive Ratatui-based terminal client in `actual-tui` ([#16](https://github.com/itsvrushabh/rs-budget/issues/16))
  - [ ] Connect to local SQLite budget files or remote servers.
  - [ ] Navigate months and view category group and envelope balances.
  - [ ] Browse accounts and transactions with search and filter capabilities.
  - [ ] Inline transaction creation, editing, split entry, and categorization.
  - [ ] Edit category allocations and envelope settings with immediate calculation feedback.
  - [ ] Keyboard navigation, terminal resizing, error reporting, and destructive action confirmation.
- [ ] Implement automated bank synchronization integration in `actual-import` ([#38](https://github.com/itsvrushabh/rs-budget/issues/38))
  - [ ] Build adapters for SimpleFIN and GoCardless (Nordigen Open Banking) APIs.
  - [ ] Securely store bank credentials and support incremental delta synchronization.

## Phase 4 — Sync engine & 100% Rust self-hosted server

- [ ] Implement complete client CRDT sync logic in `actual-sync` ([#17](https://github.com/itsvrushabh/rs-budget/issues/17))
  - [ ] Message log management, Merkle tree range diffing, and conflict resolution.
  - [ ] Offline queueing, exponential backoff, and network reconnect logic.
- [ ] Implement full budget file encryption packaging, backup archives, and key rotation ([#39](https://github.com/itsvrushabh/rs-budget/issues/39))
  - [ ] Encrypt raw SQLite budget files into `.actual` transportable archives using AES-256-GCM.
  - [ ] Support re-encrypting budget databases and CRDT logs under an updated user passphrase.
- [ ] Build 100% Rust self-hosted sync server with Axum in `actual-server` ([#18](https://github.com/itsvrushabh/rs-budget/issues/18))
  - [ ] User authentication, encryption key exchange, budget metadata, and sync endpoints (`/sync/sync`).
  - [ ] Multi-architecture Docker container images (`linux/amd64`, `linux/arm64`).
- [ ] Cross-client sync interoperability tests with official Actual clients and server ([#19](https://github.com/itsvrushabh/rs-budget/issues/19))
  - [ ] End-to-end multi-device sync, conflict scenarios, and offline reconnects.

## Phase 5 — 100% Rust native graphical clients

- [ ] Prototype and evaluate pure Rust cross-platform GUI toolkit in `actual-gui` ([#20](https://github.com/itsvrushabh/rs-budget/issues/20))
  - [ ] Evaluate Slint / Dioxus against table editing, keyboard navigation, and accessibility.
- [ ] Build native Desktop client for Linux, macOS, and Windows ([#21](https://github.com/itsvrushabh/rs-budget/issues/21))
  - [ ] Offline-first SQLite support and native OS packaging.
- [ ] Build Web client compiled to WebAssembly (WASM with OPFS) ([#22](https://github.com/itsvrushabh/rs-budget/issues/22))
  - [ ] Browser SQLite persistence via Origin Private File System (OPFS).
- [ ] Build Mobile client for Android and iOS ([#23](https://github.com/itsvrushabh/rs-budget/issues/23))
  - [ ] Touch gestures, mobile layouts, Android APK/AAB, and iOS IPA.

## Phase 6 — Parity, migration, and cutover

- [ ] Parity validation, migration testing, and public preview release ([#24](https://github.com/itsvrushabh/rs-budget/issues/24))
  - [ ] Pass full feature matrix and integration verification across CLI, TUI, Server, and Graphical clients.
  - [ ] Test cross-version upgrades, automated backups, and recovery on real-world anonymized budgets.
  - [ ] Ship an opt-in preview release for community validation and feedback.
  - [ ] Finalize documentation, user migration guides, and production release pipeline.
