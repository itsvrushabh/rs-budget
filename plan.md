# Actual Budget Rust Rewrite

This is the planning index for rewriting [Actual Budget](https://github.com/actualbudget/actual) in Rust. The goal is to preserve Actual's local-first envelope budgeting workflows, existing budget data, and sync compatibility—not to design a new budgeting product.

## Plan documents

- [Feature parity](./plans/features.md) — existing product capabilities and what “parity” means.
- [Architecture and compatibility](./plans/architecture.md) — proposed Rust workspace boundaries and data/protocol constraints.
- [Implementation roadmap](./plans/roadmap.md) — staged work from upstream inventory through release cutover.
- [CI and release plan](./plans/ci-release.md) — verification, automated PR review, and production releases.
- [Future opportunities](./plans/future.md) — enhancements intentionally deferred until parity is established.

## Project guardrails

1. Pin and document the upstream commit or release used as the behavior baseline.
2. Verify formats, protocols, and platform support in upstream source and tests; do not assume their implementation.
3. Use Turso/libSQL for rs-budget storage while preserving an explicit, tested import path for Actual's SQLite budgets; retain budget-file, encryption, and sync compatibility unless a separately reviewed migration is necessary.
4. Treat data safety and interoperability as release gates. Never silently discard, rewrite, or strand an existing budget.
5. Mark a feature complete only when its behavior is covered by Rust tests and a representative end-to-end workflow.

These are planning documents, not a claim that the listed architecture or milestones have already been implemented. Refresh the upstream baseline and feature inventory as the port progresses.
