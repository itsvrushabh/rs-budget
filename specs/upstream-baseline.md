# Upstream Baseline Specification

## 1. Pinned Baseline Metadata

- **Upstream Repository:** [actualbudget/actual](https://github.com/actualbudget/actual)
- **Pinned Tag / Release:** `v26.11.0`
- **Pinned Commit Hash:** `1766241e3f4b3c578e424c4fa15f1eb83c0da17d`
- **Release Date:** 2026-11-02
- **License:** MIT

All behavioral expectations, data formats, cryptographic parameters, calculation rules, and sync protocols in `rs-budget` are verified against this pinned baseline.

---

## 2. Upstream Architecture & Rust Crate Mapping

Actual Budget is organized as a TypeScript/JavaScript monorepo. The table below maps upstream responsibilities to the 100% Rust workspace crates:

| Upstream Package | Primary Upstream Responsibilities | Target Rust Crate in `rs-budget` |
|---|---|---|
| `packages/loot-core` | Spreadsheet calculation graph, envelope rollover, transactions, accounts, rules, schedules, SQLite persistence | `crates/actual-core` (domain logic & calculation DAG)<br>`crates/actual-storage` (SQLite & migrations) |
| `packages/crdt` | Hybrid Logical Clocks (HLC), Merkle trie comparison, sync protobuf definitions | `crates/actual-sync` |
| `packages/loot-core/src/server/encryption` | PBKDF2 key derivation, AES-256-GCM encryption, key management | `crates/actual-crypto` |
| `packages/loot-core/src/server/importers` | QIF, OFX, and CSV statement parsers, transaction deduplication | `crates/actual-import` |
| `packages/api` | Typed consumer API, query engine, undo/redo stack | `crates/actual-api` |
| `packages/sync-server` | Self-hosted sync backend, user authentication, file management, sync delta dispatch | `crates/actual-server` |
| `packages/cli` | Non-interactive command-line interface for scripts and budget inspection | `crates/actual-cli` (Tier 1) |
| *(New in rs-budget)* | Full-screen interactive terminal client via `ratatui` | `crates/actual-tui` (Tier 1) |
| `packages/desktop-client`<br>`packages/desktop-electron`<br>`packages/mobile-client` | Web client, native desktop wrappers (Electron), mobile client (Capacitor/PWA) | `crates/actual-gui` (Tier 2, 100% native Rust GUI e.g. Slint / Dioxus) |

---

## 3. Cryptographic Specification

Verified from upstream `packages/loot-core/src/server/encryption/encryption-internals.ts`:

- **Key Derivation (PBKDF2):**
  - Algorithm: PBKDF2
  - Hash Function: `SHA-512`
  - Salt: Unique user/file salt buffer (UTF-8 bytes)
  - Iteration Count: `10,000`
  - Target Key: 256-bit AES key (`length: 256`)
- **Symmetric Encryption (AES-GCM):**
  - Cipher: `aes-256-gcm`
  - IV Length: `12` bytes (96 bits) randomly generated per message
  - Auth Tag Length: `16` bytes (128 bits) appended to ciphertext / serialized in metadata
  - Format: Raw ciphertext with detached 128-bit authentication tag and 96-bit initialization vector

---

## 4. Protobuf Wire Protocol

Extracted from upstream `packages/crdt/src/proto/sync.proto`:

- **`EncryptedData`**: Transports AES-GCM encrypted payloads with detached IV (`bytes iv = 1`), authentication tag (`bytes authTag = 2`), and ciphertext (`bytes data = 3`).
- **`Message`**: Atomic CRDT change entry consisting of `dataset` (table name), `row` (record ID), `column` (field name), and `value` (serialized value).
- **`MessageEnvelope`**: Container with monotonically increasing HLC `timestamp`, encryption flag `isEncrypted`, and payload `content`.
- **`SyncRequest`**: Client-to-server payload containing batch of `MessageEnvelope`, `fileId`, `groupId`, `keyId`, and `since` timestamp.
- **`SyncResponse`**: Server-to-client response containing new `messages` and the updated server Merkle trie string representation (`string merkle = 2`).

---

## 5. Sync Server HTTP API Contract

Verified from upstream `packages/sync-server/src/app-sync.ts`:

| Method | Endpoint | Description |
|---|---|---|
| `POST` | `/sync` | Submit sync messages and receive remote deltas |
| `POST` | `/user-get-key` | Retrieve encryption key metadata for current budget |
| `POST` | `/user-create-key` | Register new encryption key metadata |
| `POST` | `/reset-user-file` | Reset cloud copy of budget file |
| `POST` | `/upload-user-file` | Upload complete encrypted SQLite budget database |
| `GET` | `/download-user-file` | Download complete encrypted SQLite budget database |
| `POST` | `/update-user-filename` | Rename budget file |
| `GET` | `/list-user-files` | List all available budgets for authenticated user |
| `GET` | `/get-user-file-info` | Retrieve metadata (size, last modified) for specific budget |
| `POST` | `/delete-user-file` | Delete cloud budget file |

---

## 6. Authoritative Database Schema

Upstream SQLite storage initializes via `init.sql` and applies 54 sequential migrations.
- Base tables include: `accounts`, `categories`, `category_groups`, `transactions`, `created_budgets`, `spreadsheet_cells`, `banks`, `messages_crdt`, `messages_clock`, `category_mapping`, and `__migrations__`.
- All migrations after version `1780606215001` are additive-only, ensuring forward and backward schema compatibility.
