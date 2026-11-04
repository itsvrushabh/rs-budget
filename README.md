# rs-budget

A local-first, 100% Rust rewrite of [Actual Budget](https://github.com/actualbudget/actual).

## Project Overview

`rs-budget` preserves Actual's local-first envelope budgeting workflows, native SQLite budget files, end-to-end encryption, and CRDT synchronization, rewritten entirely in Rust with high performance, safety, and a first-class terminal experience (CLI & TUI).

## Documentation & Guides

- [Upstream Parity Matrix](docs/parity-matrix.md) — 100% upstream feature parity breakdown across all 6 phases.
- [Migration Guide](docs/migration-guide.md) — Instructions for migrating existing budgets and sync servers.
- [Configuration Reference](docs/configuration.md) — Server environment variables, CLI commands, and TUI keybindings.
- [Release Notes (v0.1.0-preview)](docs/release-notes-v0.1.0-preview.md) — Public preview release details.

## Planning & Architecture

All architectural specifications, feature matrices, implementation roadmaps, and CI/release workflows are maintained on the dedicated [`planning`](https://github.com/itsvrushabh/rs-budget/tree/planning) branch:

- [Planning Index (`plan.md`)](https://github.com/itsvrushabh/rs-budget/blob/planning/plan.md)
- [Feature Parity Plan](https://github.com/itsvrushabh/rs-budget/blob/planning/plans/features.md)
- [Architecture & Compatibility Plan](https://github.com/itsvrushabh/rs-budget/blob/planning/plans/architecture.md)
- [Implementation Roadmap](https://github.com/itsvrushabh/rs-budget/blob/planning/plans/roadmap.md)
- [CI, PR Review & Release Plan](https://github.com/itsvrushabh/rs-budget/blob/planning/plans/ci-release.md)
- [Future Opportunities](https://github.com/itsvrushabh/rs-budget/blob/planning/plans/future.md)

## Repository Branches

- `main` — Production-ready codebase and releases.
- `dev` — Primary integration and development branch.
- `release` — Automated release pipeline branch for GitHub Actions (Android, iOS, Cargo crates, Docker web server images).
- `planning` — Architecture, technical specifications, and planning documents.
- `ci-workflow` — CI pipelines, workflows, build scripts, and developer tooling.
