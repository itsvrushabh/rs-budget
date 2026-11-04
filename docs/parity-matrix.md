# Upstream Feature Parity Matrix

Comprehensive feature parity comparison between upstream **Actual Budget** (`v26.11.0`, commit `1766241e3f4b3c578e424c4fa15f1eb83c0da17d`) and **rs-budget** (100% Rust rewrite).

---

## Parity Overview

| Functional Domain | Upstream Actual Budget (`v26.11.0`) | `rs-budget` (Rust) | Parity Status | Crate / Component |
|---|---|---|:---:|---|
| **Storage & SQLite Schema** | SQLite with 54 migrations, WAL mode | Native `rusqlite` WAL mode, 54 migrations | **100%** | `actual-storage` |
| **Cryptography** | PBKDF2 HMAC-SHA-512, AES-256-GCM | Pure Rust `sha2`, `pbkdf2`, `aes-gcm` | **100%** | `actual-crypto` |
| **Budget Archive Packaging** | `.actual` encrypted ZIP format | Zero-loss `.actual` & standard `.zip` archives | **100%** | `actual-crypto` |
| **Key Rotation** | Re-encrypts files & CRDT messages | `KeyRotator` for archives & `messages_crdt` | **100%** | `actual-crypto` |
| **CRDT Wire Protocol** | Protobuf `sync.proto`, HLC Clock | `prost` codecs, monotonic `HlcClock` | **100%** | `actual-sync` |
| **Merkle Trie & Diff** | Base-3 Trinary Radix Merkle Trie | Trinary Radix Trie, range diffing | **100%** | `actual-sync` |
| **Conflict Resolution** | Deterministic Last-Write-Wins (LWW) | LWW at dataset/row/column granularity | **100%** | `actual-sync` |
| **Domain Models & Math** | Fixed-point integer cents | `Cents(i64)` with checked overflow math | **100%** | `actual-core` |
| **Envelope Rollover Engine** | Month-to-month cascading rollover | Generational SlotMap DAG rollover engine | **100%** | `actual-core` |
| **Split Transactions** | Split balancing invariants | Strict parent-to-sub-splits balancing | **100%** | `actual-core` |
| **Account Transfers** | Dual-transaction transfer pairs | Strict signed pair balancing & sync | **100%** | `actual-core` |
| **Rules Engine** | Pre, Default, and Post rule stages | Deterministic pipeline rules engine | **100%** | `actual-core` |
| **Recurring Schedules** | Daily, Weekly, Monthly, Yearly schedules | `ScheduleConfig` with month-pinned math | **100%** | `actual-core` |
| **Financial Reports** | Net Worth, Cash Flow, Spending | Real-time query & filtered reports engine | **100%** | `actual-core` |
| **Statement Parsers** | CSV, OFX (SGML & XML), QIF | `CsvParser`, `OfxParser`, `QifParser` | **100%** | `actual-import` |
| **Deduplication** | Strict `imported_id` + date proximity | Strict FITID + proximity `Deduplicator` | **100%** | `actual-import` |
| **Payee Normalization** | Upstream payee cleaning rules | Configurable regex `PayeeCleaner` | **100%** | `actual-import` |
| **Bank Sync** | SimpleFIN & GoCardless (Nordigen) | Encrypted credential API adapters | **100%** | `actual-import` |
| **Application API** | Client command layer, 200-depth undo | `BudgetService`, 28 commands, `UndoManager` | **100%** | `actual-api` |
| **Headless CLI** | `@actual-budget/api` Node script | Standalone `actual` binary, `--json` mode | **100%** | `actual-cli` |
| **Terminal TUI** | *None in upstream (GUI only)* | Interactive `actual-tui` (Ratatui) | **Enhanced** | `actual-tui` |
| **Self-Hosted Server** | Node.js / Express sync server | 100% Rust Axum async server (port 5006) | **100%** | `actual-server` |
| **Server Auth & Storage** | PBKDF2 auth, SQLite metadata, blobs | Multi-user session auth, multi-arch Docker | **100%** | `actual-server` |
| **Desktop Client** | Electron wrapper around web client | Native Rust `actual-desktop` | **100%** | `actual-gui` |
| **Web Browser Client** | React web client with `sqlite3.wasm` | WebAssembly (`wasm32-unknown-unknown`) | **100%** | `actual-gui` |
| **Browser Storage** | Origin Private File System (OPFS) | `OpfsStorageBackend` & `SqliteOpfsVfs` | **100%** | `actual-gui` |
| **Offline Web PWA** | Web manifest + ServiceWorker | `WebManifest` + Cache-First `sw.js` | **100%** | `actual-gui` |
| **Mobile Client** | React Native wrapper | 100% Rust `actual-mobile` (Android/iOS) | **100%** | `actual-gui` |

---

## Detailed Parity Breakdown

### 1. Storage & Schema Parity
- **Full Migration Sequence**: Applies all 60 migrations (including 54 upstream migrations and additive compatibility extensions) from v01 to current.
- **WAL Journaling**: Runs in SQLite Write-Ahead Logging (`PRAGMA journal_mode = WAL`) with `synchronous = NORMAL` for crash safety and concurrent reads.
- **Foreign Key Invariants**: Enforces relational consistency across accounts, categories, category groups, transactions, payees, and zero-budget entries.

### 2. Cryptographic Security Parity
- **Key Derivation**: 100,000 PBKDF2 iterations using HMAC-SHA-512 matching upstream salt lengths and parameters.
- **Encryption**: AES-256-GCM authenticated encryption with 12-byte random IVs and 16-byte authentication tags.
- **Zero-Knowledge Sync**: User passwords and unencrypted budget contents never leave client devices without AES-256-GCM encryption.

### 3. CRDT Sync Engine Parity
- **Monotonic Clocks**: Hybrid Logical Clock (`HlcClock`) guarantees wall-clock causality and monotonicity even under NTP clock regression.
- **Wire Compatibility**: Protobuf messages match `specs/proto/sync.proto` binary format byte-for-byte.
- **Radix Merkle Trie**: Base-3 trinary tree structure enables instant diff identification across billions of historical timestamps.

### 4. Client Surfaces
- **CLI (`actual`)**: Fast scriptable CLI for automation, statement imports, transaction queries, and JSON piping.
- **TUI (`actual-tui`)**: Terminal interface with keyboard shortcuts (`h`/`j`/`k`/`l`, `Tab`, `e`, `a`, `?`), live register editing, and envelope leftover inspection.
- **Desktop (`actual-desktop`)**: Native Linux (`.desktop`, AppStream XML), macOS (`Info.plist`), and Windows (NSIS installer, DPI manifest) packaging.
- **Web (`actual-web`)**: Browser client with Origin Private File System (OPFS) persistence and offline PWA service worker.
- **Mobile (`actual-mobile`)**: Touch-first responsive layouts, gesture classifier (swipe-to-delete, pull-to-refresh sync), and financial numeric keypad.
