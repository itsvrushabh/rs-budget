# Actual Budget Rust Rewrite

This is the planning index for rewriting [Actual Budget](https://github.com/actualbudget/actual) in Rust. The goal is to preserve Actual's local-first envelope budgeting workflows, existing budget data, and sync compatibility—not to design a new budgeting product.

## Plan documents

- [Feature parity](./plans/features.md) — existing product capabilities and what “parity” means.
- [Architecture and compatibility](./plans/architecture.md) — proposed Rust workspace boundaries and data/protocol constraints.
- [Implementation roadmap](./plans/roadmap.md) — staged work from upstream inventory through release cutover.
- [CI and release plan](./plans/ci-release.md) — branching strategy, PR submission guidelines & mandatory issue linking, verification, and release pipeline.
- [Future opportunities](./plans/future.md) — enhancements intentionally deferred until parity is established.

## Project guardrails

1. Pin and document the upstream commit or release used as the behavior baseline.
2. Verify formats, protocols, and platform support in upstream source and tests; do not assume their implementation.
3. Use SQLite (via `rusqlite`) as rs-budget's default authoritative storage engine, ensuring 1:1 file and sync compatibility with upstream Actual budgets without requiring lossy conversion or mandatory database migration.
4. Commit to 100% Rust across the entire stack—from domain logic, storage, and server to the CLI, TUI, and native graphical client interfaces.
5. Prioritize the CLI and interactive TUI as early first-class operational surfaces to validate core budgeting workflows before native graphical UI delivery.
6. Treat data safety and interoperability as release gates. Never silently discard, rewrite, or strand an existing budget.
7. Mark a feature complete only when its behavior is covered by Rust tests and a representative end-to-end workflow.
8. Require all code and documentation contributions to be submitted via Pull Requests linked to an explicit GitHub Issue ID; unlinked PRs or direct commits to protected branches are rejected.

These are planning documents, not a claim that the listed architecture or milestones have already been implemented. Refresh the upstream baseline and feature inventory as the port progresses.
