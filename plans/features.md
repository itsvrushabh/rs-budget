# Feature Parity Plan

The initial Rust release targets functional parity with the pinned Actual Budget upstream version. Confirm the exact feature set against upstream code, documentation, tests, and supported deployments before marking work complete.

## Budgeting and financial workflows

- Budgets, months, month rollover, and envelope-budget calculations.
- Accounts and account groups; transactions, splits, payees, and categories.
- Schedules, rules, reconciliation, reports, and undo.
- Monetary precision, date handling, carry-forward, and overspending behavior consistent with upstream.

## Data and integrations

- Existing Actual SQLite budget files remain importable; rs-budget stores active data in Turso/libSQL.
- Versioned Turso schema migrations and backup/recovery workflows.
- Existing encryption and key-derivation formats.
- Upstream-supported financial imports and exports, preserving their behavior and edge cases.
- Existing application API, sync protocol, and multi-device behavior.

## Applications and deployment surfaces

Actual has shared budgeting functionality presented through multiple clients and a separate server. In the Rust rewrite, keep these roles distinct:

| Surface | Role in rs-budget |
|---|---|
| Web UI | Browser client. Preserve both server-backed use and any supported local-only browser workflow. |
| Desktop app | Desktop client for Windows, macOS, and Linux. Preserve offline use and server connection where supported. |
| Mobile app | Mobile client for Android and iOS; the iOS app should support iPad as well as iPhone. Upstream also documents installing the web app on mobile. |
| Server | Self-hosted service for sync and server-dependent features. It is a backend component, not another client UI. |
| CLI | Non-interactive command-line interface for scripting, automation, and direct commands. |
| TUI | New rs-budget full-screen terminal client. This is an explicit addition, not an upstream parity requirement. |

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
- Verify SQLite-to-Turso import results, migration safety, encryption, imports/exports, and sync against upstream-compatible fixtures or implementations.
- Define supported old/new client and server combinations during migration.
- Use copied test budgets; verify failed imports leave the original SQLite budget recoverable and untouched.
- Never include real user financial data in repository fixtures or CI artifacts.
