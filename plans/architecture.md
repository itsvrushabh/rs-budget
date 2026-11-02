# Architecture and Compatibility Plan

## Guiding decisions

- Begin by porting verified upstream responsibilities; do not invent new service boundaries or protocols in advance.
- Commit to **100% Rust** across the entire stack: domain logic, storage, synchronization, server, CLI, TUI, and graphical interfaces.
- Use **SQLite** (via `rusqlite`) as rs-budget's default authoritative storage engine, ensuring 1:1 binary file, schema, and query compatibility with upstream Actual budgets without requiring data migration.
- Abstract storage access behind a clean `BudgetStore` trait in `actual-storage` to cleanly separate persistence from core budgeting domain logic (and allow optional future backends if needed).
- Prioritize the **CLI** and interactive **TUI** as early Tier-1 operational surfaces to establish and validate end-to-end user workflows before tackling native graphical clients.
- Maintain strict budget-file, encryption, and CRDT sync compatibility with upstream Actual Budget.
- For graphical interfaces, select a 100% Rust UI framework (such as Slint or Dioxus) after focused prototyping against platform responsiveness, keyboard navigation, and accessibility requirements.

## Candidate Rust workspace

```text
actual-rust/
└── crates/
    ├── actual-core/      # Domain types, envelope calculations, rules, spreadsheet DAG, and reports
    ├── actual-storage/   # SQLite (rusqlite) access, schema DDL, versioned migrations, and queries
    ├── actual-sync/      # Application CRDT (HLC, Merkle tree), protobuf wire formats, sync engine
    ├── actual-crypto/    # PBKDF2, AES-GCM encryption and key-derivation compatibility
    ├── actual-import/    # Upstream-supported financial import and export formats (QIF, OFX, CSV)
    ├── actual-api/       # Typed application API and client-service boundary
    ├── actual-server/    # 100% Rust self-hosted HTTP server (Axum), auth, and sync endpoints
    ├── actual-cli/       # Non-interactive CLI for scripting, automation, and inspection (Tier 1)
    ├── actual-tui/       # Full-screen interactive terminal client via Ratatui (Tier 1)
    └── actual-gui/       # 100% native Rust cross-platform GUI (Tier 2: Desktop, Web WASM, Mobile)
```

These crate boundaries maintain clean dependency layering and clear ownership. Build the smallest useful vertical slice first (`actual-storage` + `actual-core` + `actual-cli`/`actual-tui`).

## Client surfaces & prioritization

Plan for the following distinct client/deployment targets:

### Tier 1: Fast-Validation and Power-User Surfaces
- **CLI (`actual-cli`):** 100% Rust non-interactive commands for scripting, batch transaction imports, account inspection, and automated maintenance.
- **TUI (`actual-tui`):** 100% Rust interactive, keyboard-driven full-screen terminal client built with `ratatui`. Delivers early, complete budgeting workflows (envelope allocations, transaction entry, month rollover navigation) directly against `actual-core`.
- **Server (`actual-server`):** 100% Rust self-hosted sync and coordination backend built with `axum`. Implements Actual's sync endpoints, auth, and key management without any Node.js dependencies.

### Tier 2: 100% Rust Native Graphical Surfaces
- **Desktop:** 100% native Rust GUI (e.g. Slint / Dioxus) for Linux, macOS, and Windows with offline-first SQLite support.
- **Web:** 100% Rust browser UI compiled to WebAssembly (WASM), using SQLite over OPFS (Origin Private File System) for local-first browser storage.
- **Mobile:** 100% Rust mobile builds for Android and iOS (including iPad).

All clients share domain logic, validation, and calculations through `actual-core`. Neither the TUI nor the GUI duplicate budgeting calculations or bypass established storage and encryption paths.

## Core engine & reactive calculation architecture

Upstream Actual relies on a reactive spreadsheet calculation graph (`loot-core`) where budget envelope cells, category balances, and month rollovers recompute incrementally upon transaction or allocation changes:

- **Spreadsheet Dependency Graph:** Implement the reactive calculation graph in `actual-core` using a directed acyclic graph (DAG) backed by a generational arena/slotmap. This avoids Rust borrow-checker conflicts while enabling deterministic, cycle-free topological recomputations.
- **Rules Engine:** Port upstream transaction rules (conditions and actions applied during transaction creation/sync) as a deterministic rule-evaluation pipeline.
- **Deterministic Precision:** Use fixed-point integer cents/milli-units for all financial balances and currency math, matching upstream Actual's exact monetary encoding.

## Sync and protocol compatibility

Upstream Actual synchronizes data across clients and servers using an application-level CRDT:
- **Hybrid Logical Clocks (HLC):** Generate monotonically increasing timestamps across distributed clients.
- **Merkle Tree State Comparison:** Efficiently detect divergence between client and server message logs.
- **Protobuf Wire Codecs:** Encode synchronization deltas (`SyncRequest`, `SyncResponse`) using `prost` matching upstream protobuf definitions.
- **End-to-End Encryption:** Match upstream PBKDF2 key derivation and AES-GCM payload encryption for synced data.

## SQLite storage model

- Use `rusqlite` with WAL (Write-Ahead Logging) mode enabled for high concurrency and crash resilience.
- Maintain identical table schemas (`accounts`, `categories`, `category_groups`, `transactions`, `rules`, `schedules`, `messages_merkletree`, `messages`) and schema migration versions to allow opening existing Actual budgets directly without conversion.
- For Web/WASM targets, utilize an OPFS or IndexedDB VFS for `rusqlite` to maintain local-first database persistence in the browser.
- Encapsulate connection management, transactions, and statements behind `actual-storage` so that domain crates do not execute raw SQL directly.

## Data safety

- Add compatibility and known-answer test fixtures before changing serialization or cryptographic code.
- Test schema migrations against authentic, anonymized budget snapshots across historical upstream versions.
- Ensure failed operations roll back cleanly within SQLite transactions, leaving source files untouched and intact.
- Differential test calculation outputs (envelope balances, rollover, overspending) against Node.js Actual on identical budget inputs.
