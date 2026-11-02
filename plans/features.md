# Feature Parity Plan

The initial Rust release targets functional parity with the pinned Actual Budget upstream version. Confirm the exact feature set against upstream code, documentation, tests, and supported deployments before marking work complete.

## Budgeting and financial workflows

- Budgets, months, month rollover, and envelope-budget calculations.
- Accounts and account groups; transactions, splits, payees, and categories.
- Schedules, rules, reconciliation, reports, and undo.
- Monetary precision, date handling, carry-forward, and overspending behavior consistent with upstream.

## Data and integrations

- rs-budget stores active budget data directly in SQLite using `rusqlite`, maintaining 100% native file and schema compatibility with upstream Actual budgets.
- Versioned SQLite schema migrations and backup/recovery workflows compatible with upstream migration versions.
- Existing encryption and key-derivation formats.
- Upstream-supported financial imports and exports, preserving their behavior and edge cases.
- Existing application API, sync protocol, and multi-device behavior.

## Applications and deployment surfaces

Actual has shared budgeting functionality presented through multiple clients and a separate server. In rs-budget, all surfaces are built with **100% Rust**, with the CLI and TUI prioritized as early first-class operational interfaces:

| Surface | Priority | Role in rs-budget |
|---|---|---|
| CLI | High (Tier 1) | 100% Rust non-interactive command-line interface for scripting, automation, and direct inspection. |
| TUI | High (Tier 1) | 100% Rust full-screen interactive terminal client (`ratatui`) for fast, keyboard-first budgeting. |
| Server | High (Tier 1) | 100% Rust self-hosted backend service (`axum`) for sync, encryption key exchange, and multi-device coordination. |
| Desktop app | Tier 2 | 100% native Rust GUI (e.g. Slint / Dioxus) for Windows, macOS, and Linux, with full offline local-first support. |
| Web UI | Tier 2 | 100% Rust browser client compiled to WebAssembly (WASM). |
| Mobile app | Tier 2 | 100% Rust mobile client for Android and iOS (including iPad). |

Preserve upstream-supported interaction patterns, localization, and accessibility on existing clients. Define keyboard-first navigation and readable, accessible terminal output for the TUI.

## Initial TUI scope

The TUI should use the same domain behavior and API as the other clients, not duplicate budgeting calculations or bypass data-safety rules. Its first usable release should support:

- Connecting to the supported local/server budget workflow and selecting a budget.
- Navigating months and viewing budget/category balances.
- Viewing accounts and transactions; creating and editing transactions, including categorization.
- Editing category allocations and common account/category settings.
- Clear connection, sync, validation, and error states, with confirmation for destructive actions.

Defer advanced reports, full rule/schedule editors, and parity with every graphical workflow until the core terminal workflows are proven. Keep the scope explicit in the feature matrix.

## Definition of parity

For each feature, record:

1. User workflow and observable behavior.
2. Upstream package, API, and relevant test coverage.
3. Database/file-format and migration effects.
4. Sync and encryption effects.
5. Rust unit/integration coverage and end-to-end verification.

A feature is not complete solely because its screen or endpoint exists. Verify calculations and persisted results, failure/recovery behavior, and relevant interoperability. Document intentionally unsupported behavior and its impact on data migration and users.

## Compatibility requirements

- Test representative, anonymized fixtures from supported budget versions.
- Verify native SQLite database compatibility, schema migrations, encryption, imports/exports, and sync against upstream-compatible fixtures and official Actual releases.
- Define supported old/new client and server combinations during migration and sync negotiation.
- Use copied test budgets; verify failed operations leave existing SQLite budget files recoverable and untouched.
- Never include real user financial data in repository fixtures or CI artifacts.
