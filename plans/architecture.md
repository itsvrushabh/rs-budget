# Architecture and Compatibility Plan

## Guiding decisions

- Begin by porting verified upstream responsibilities; do not invent new service boundaries or protocols in advance.
- Use Turso/libSQL as rs-budget's target storage engine instead of SQLite.
- Treat Actual's existing SQLite budget files as an import/compatibility format. Preserve the ability to import them; do not assume Turso can open them in place or that the schemas and runtime behaviors are interchangeable.
- Keep budget-file, encryption, and sync compatibility as explicit requirements independent of the database engine. Any conversion must be versioned, validated, recoverable, and must not silently alter or discard source data.
- Select the UI framework after a focused prototype against upstream platform and accessibility requirements.

## Candidate Rust workspace

```text
actual-rust/
└── crates/
    ├── actual-core/      # Domain types, calculations, rules, and reports
    ├── actual-storage/   # Turso/libSQL access, schema, migrations, and SQLite import
    ├── actual-sync/      # Existing CRDT semantics, protobuf, and sync behavior
    ├── actual-crypto/    # Existing encryption and key-derivation compatibility
    ├── actual-import/    # Upstream-supported financial import and export formats
    ├── actual-api/       # Typed application API and compatibility layer
    ├── actual-server/    # Local/self-hosted HTTP services, auth, and sync server
    ├── actual-ui/        # Shared UI/domain-facing client APIs and platform adapters
    ├── actual-tui/       # Interactive full-screen terminal client (new rs-budget surface)
    └── actual-cli/       # Non-interactive scripting and command-line workflows
```

These are candidate boundaries, not a mandate to create every crate immediately. Confirm module ownership and dependencies from the pinned upstream source and build the smallest useful vertical slice first.

## Client surfaces

Plan for the following distinct client/deployment targets:

- **Web:** browser UI, supporting server-backed operation and local-only use where upstream supports it.
- **Desktop:** Windows, macOS, and Linux app, including offline workflows where supported.
- **Mobile:** Android and iOS client; iOS includes iPad. Also account for the upstream-documented installable web app on mobile.
- **Server:** self-hosted backend for sync and server-dependent capabilities; separate from the client applications.
- **CLI:** non-interactive commands for scripts and automation.
- **TUI:** interactive, keyboard-driven terminal client, newly added for rs-budget.

The exact upstream implementation and support status for each target must be recorded in the baseline inventory. Do not equate an available build project with a generally supported/released product.

All clients should share domain logic and validation through the core/API boundary. The TUI must not implement independent money calculations or write around established storage, encryption, or sync paths. Keep interactive TUI navigation distinct from CLI commands and machine-readable CLI output.

## Compatibility work

Before implementation, document:

- Upstream SQLite schema, migration order, query/mutation semantics, and transaction boundaries.
- Turso/libSQL Rust client capabilities, supported platforms, local/offline behavior, remote connectivity, replication model, and operational requirements.
- The rs-budget Turso schema and migration/versioning policy, including which upstream SQLite fields and semantics need translation.
- Budget-file layout, amount/date encoding, encryption parameters, and key derivation.
- Protobuf definitions, CRDT behavior, API routes, and client/server sync negotiation.
- Import/export formats and upstream-supported desktop, browser, and server targets.

Implement Turso as the authoritative storage layer for rs-budget. Keep the upstream SQLite reader/importer as a compatibility boundary, and port observed application and sync contracts rather than assuming specific algorithms. In particular, verify whether and how upstream uses HLCs, Merkle structures, WebSockets, or other synchronization mechanisms before choosing Rust equivalents. Database replication is not a replacement for Actual's application-level sync protocol unless interoperability and conflict behavior are demonstrated.

## Turso deployment model

- Use the Turso/libSQL Rust client behind `actual-storage`; isolate vendor-specific APIs from core budgeting logic.
- Prototype local-first operation against the supported Turso/libSQL modes before committing to a deployment topology. Verify offline reads/writes, reconnect and replication behavior, transaction guarantees, conflict handling, and supported desktop/mobile targets.
- Define whether a user's local budget is a local database, an embedded replica, or another supported Turso mode, and how it relates to an optional self-hosted/remote Turso database. Do not promise seamless offline replication until tested.
- Keep credentials and remote connection configuration out of budget files and logs. Document backup, restore, export, and recovery procedures for both local and remote data.
- If a Turso mode cannot meet required platforms or local-first behavior, surface that as an architecture decision before implementation expands; do not silently fall back to a different engine.

## Data safety

- Add compatibility and known-answer fixtures before changing serialization or cryptographic code.
- Make Turso schema migrations versioned and recoverable from a backup.
- Build and test a one-way or explicitly documented SQLite-to-Turso import path on copied budgets; compare imported data and calculated results with upstream.
- Ensure failed imports leave the original SQLite budget untouched and the partial Turso database safe to discard or resume.
- Treat the SQLite-to-Turso conversion as an explicit storage migration within the port. Keep any additional file-format or sync-protocol changes separate, with explicit user communication and rollback guidance.
