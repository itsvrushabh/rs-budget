# Implementation Roadmap

Track each checklist item in issues or pull requests as work begins. Update this document when the upstream baseline or verified scope changes.

## Phase 0 — Upstream inventory and baseline

- [ ] Pin the upstream commit/release to port.
- [ ] Map upstream packages—including core, CRDT, API, web/desktop/mobile clients, server, CLI, and docs—to Rust responsibilities; distinguish released/supported surfaces from build targets.
- [ ] Create a feature/workflow matrix from upstream source, docs, tests, and CI; identify supported platforms and release artifacts.
- [ ] Record build/test commands, database and file formats, protobuf definitions, API routes, external integrations, and known unsupported/experimental features.
- [ ] Prepare representative anonymized fixtures and expected results for calculations, migrations, imports, encryption, and sync.

## Phase 1 — Data, crypto, and protocol compatibility

- [ ] Document upstream SQLite schema and migration ordering, amount/date encoding, budget-file layout, encryption, key derivation, and supported import/export formats.
- [ ] Select and prototype the Turso/libSQL Rust client and required local/remote mode; verify supported targets, offline behavior, replication/conflicts, transaction guarantees, and recovery.
- [ ] Define the rs-budget Turso schema and versioned migration process.
- [ ] Implement a SQLite-to-Turso importer; compare row counts, relationships, balances, and representative calculated results against the source budget.
- [ ] Test import interruption and failure on copied budgets; ensure the SQLite source remains unchanged and recoverable.
- [ ] Port cryptographic algorithms, parameters, and serialized formats exactly; add known-answer and cross-implementation tests.
- [ ] Port observed protobuf and sync semantics; verify interoperability with upstream fixtures and supported client/server combinations.

## Phase 2 — Core budgeting

- [ ] Port domain models and deterministic calculations for accounts, transactions, splits, categories, schedules, rules, month rollover, carry-forward, overspending, reports, and reconciliation.
- [ ] Differential-test representative inputs and edge cases against the pinned upstream implementation.
- [ ] Port query and mutation behavior behind typed Rust APIs; preserve undo and transaction consistency.

## Phase 3 — Imports, API, and server

- [ ] Port every upstream-supported import/export workflow and validate it with format fixtures and regressions.
- [ ] Implement verified application API, local/self-hosted server, authentication, budget access, encryption, and sync behavior.
- [ ] Test Turso local-only operation, restart/recovery, remote connectivity/replication where selected, multi-client sync, conflicts, retries, and older supported budget versions.

## Phase 4 — Web, desktop, and mobile clients

- [ ] Prototype and select a Rust UI framework against upstream interaction and accessibility requirements.
- [ ] Port the browser and desktop workflows for Windows, macOS, and Linux, including supported offline behavior.
- [ ] Port the mobile workflows for Android and iOS, including iPad support; evaluate the installable mobile web app separately from the native package.
- [ ] Preserve shared components, keyboard behavior, localization, themes, and accessibility.
- [ ] Package and test supported client/server deployments; isolate platform-specific behavior behind explicit adapters.

## Phase 5 — CLI and TUI

- [ ] Provide a non-interactive CLI for upstream-equivalent commands and scripting; document its server/local connection requirements.
- [ ] Build an interactive TUI as an additional rs-budget client, sharing the core/API behavior instead of duplicating business logic.
- [ ] Deliver the initial TUI workflows: budget/month navigation, category balances and allocations, account/transaction browsing, and common transaction edits.
- [ ] Test keyboard navigation, terminal resizing, narrow layouts, accessibility/readability, error states, and safe handling of destructive actions.
- [ ] Expand TUI coverage to reports, schedules, and rules only after the initial workflows are stable.

## Phase 6 — Parity, migration, and cutover

- [ ] Pass the upstream feature matrix and end-to-end tests on supported platforms; track TUI as an additional rs-budget surface, not an upstream parity blocker.
- [ ] Test upgrade, backup/restore, recovery, and old/new interoperability on copied fixtures; publish migration instructions and limitations.
- [ ] Ship an opt-in preview before broad release.
- [ ] Retire upstream runtime components only after parity, migration safety, and operational recovery criteria are met.
