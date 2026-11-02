# CI, PR Review, and Release Plan

## Branching strategy and lifecycle

The repository uses an organized branching model tailored for safety, structured integration, and automated CI/CD workflows:

| Branch | Purpose & Lifecycle | Protection & Merge Policy |
|---|---|---|
| `main` | Production-ready, stable codebase. Tagged releases (`v*.*.*`) originate here. | Protected. Requires passing CI, human review approval, and linear history. No direct commits. |
| `dev` | Primary integration branch for active development. Feature and bugfix PRs merge here. | Protected. PR required, must pass all CI checks (fmt, clippy, unit/integration tests). |
| `release` | Release staging and stabilization branch. Used for pre-release validation, version bumping, changelog preparation, and final verification before merging to `main`. | Protected. Only release preparation commits and cherry-picked critical fixes. Merges to `main` and back-merges to `dev`. |
| `planning` | Dedicated branch for architecture, roadmap updates, technical specifications, and project planning documentation. | Dedicated to planning iterations; merged into `dev`/`main` as specifications are finalized. |
| `ci-workflow` | Dedicated branch for CI pipelines, GitHub Actions workflows, build/test scripts, and developer tooling infrastructure. | Isolated branch for developing and testing workflow automation without disrupting core code. |

### Working branch naming conventions

- `feat/<scope>`: New features (e.g. `feat/tui-envelope-nav`, `feat/sqlite-schema`). Branched from `dev`, merged into `dev`.
- `fix/<issue>`: Bugfixes for active development code. Branched from `dev`, merged into `dev`.
- `hotfix/<version>`: Urgent production fixes branched directly from `main`. Merged into `main`, `release`, and back into `dev`.
- `chore/<task>`: Tooling, dependency bumps, or maintenance tasks.
- `docs/<topic>`: Documentation and non-code guide updates.

## Pull request checks

Run required checks on pull requests and pushes to `main`:

- `cargo fmt --check`.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- Workspace unit and integration tests on supported operating systems.
- Native target builds and `wasm32-unknown-unknown` checks only for browser-facing Rust code.
- Dependency/security checks, plus required data-format, sync interoperability, and differential parity checks for relevant changes.
- SQLite database integration tests and schema migration regression tests, including crash recovery and WAL consistency.
- Differential test suites verifying identical envelope calculation and rollover results between rs-budget and upstream Node.js Actual.
- Bounded parser and crypto-decoder fuzz smoke tests; run longer fuzz campaigns on a schedule.

Keep platform-specific tests explicit. Required status checks should report a result even when optional path-filtered work is skipped. Cancel superseded PR runs, retain useful logs, and identify failing targets in check summaries.

## Automated PR review

- Review PR open, reopen, and update events against the latest commit; avoid repeating unchanged findings.
- Focus on correctness, error handling, unsafe code, concurrency, data integrity, cryptography, compatibility, tests, and documentation.
- Report actionable findings with file/line references, severity, and rationale; distinguish bugs from optional suggestions.
- Keep the review advisory. Require human approval and all required CI checks; automated review must not approve or merge its own PR.
- Pin third-party actions to immutable revisions, use least-privilege permissions, and do not expose secrets to forked PRs. Restrict write access to what is needed to post feedback.

## Release workflow

A `v*.*.*` tag starts a release workflow after validating SemVer and the project's recorded release version.

1. Re-run formatting, Clippy, tests, parity/compatibility checks, supported target builds, and dependency/security checks from the tagged commit.
2. Build and smoke-test artifacts for upstream-supported desktop and self-hosted server surfaces.
3. Publish validated multi-architecture container images with immutable version tags; move major/minor tags only after successful validation.
4. Generate checksums and provenance; sign artifacts and images with the configured project identity.
5. Create release notes from merged Conventional Commits and attach artifacts, checksums, and provenance to the GitHub Release.
6. Publish Rust crates only if the project explicitly commits to a public crate API; otherwise keep workspace crates internal.
7. Make publication idempotent and visibly fail on partial releases. Document safe retry/withdrawal procedures; never silently replace an existing version or artifact.

Keep release credentials out of PR workflows. Use platform and artifact targets verified in the upstream inventory rather than assuming installer formats or architectures.
