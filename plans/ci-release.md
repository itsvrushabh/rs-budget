# CI, PR Review, and Release Plan

## Branching strategy and lifecycle

The repository uses an organized branching model tailored for safety, structured integration, and automated CI/CD workflows:

| Branch | Purpose & Lifecycle | Protection & Merge Policy |
|---|---|---|
| `main` | Production-ready, stable codebase. Tagged releases (`v*.*.*`) originate here. | Protected. Requires passing CI, human review approval, and linear history. No direct commits. |
| `dev` | Primary integration branch for active development. Feature and bugfix PRs merge here. | Protected. PR required, must pass all CI checks (fmt, clippy, unit/integration tests). |
| `release` | Automated release pipeline branch. Used strictly by GitHub Actions release workflows to package and release artifacts: Android apps (APK/AAB), iOS apps, Cargo crates/modules, and multi-architecture Docker container images for the self-hosted web server. | Locked. Direct manual commits and pushes are blocked. Changes are only pushed/merged by automated GitHub Actions workflows (or via strictly validated release PRs). |
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
2. Build and publish release artifacts via the automated `release` branch workflow:
   - **Android App:** Build signed APK and Google Play bundle (AAB).
   - **iOS App:** Build and package iOS/iPad application (IPA / TestFlight).
   - **Cargo Libraries / Modules:** Validate, package, and publish public Rust crates (`actual-core`, `actual-api`, `actual-storage`, etc.) to crates.io.
   - **Self-Hosted Web Server:** Build and publish multi-architecture container images (Docker/OCI `linux/amd64`, `linux/arm64`) with version tags to GitHub Container Registry (ghcr.io).
   - **Desktop & CLI/TUI Binaries:** Compile native standalone binaries for Linux, macOS, and Windows.
3. Generate SHA-256 checksums and provenance; sign release artifacts and container images with the configured project identity.
4. Create release notes from merged Conventional Commits and attach artifacts, checksums, and provenance to the GitHub Release.
5. Make publication idempotent and visibly fail on partial releases. Document safe retry/withdrawal procedures; never silently replace an existing version or artifact.

Keep release credentials out of PR workflows. Use platform and artifact targets verified in the upstream inventory rather than assuming installer formats or architectures.
