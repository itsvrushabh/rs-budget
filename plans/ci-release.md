# CI, PR Review, and Release Plan

## Pull request checks

Run required checks on pull requests and pushes to `main`:

- `cargo fmt --check`.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- Workspace unit and integration tests on supported operating systems.
- Native target builds and `wasm32-unknown-unknown` checks only for browser-facing Rust code.
- Dependency/security checks, plus required data-format, sync interoperability, and differential parity checks for relevant changes.
- Turso integration tests and SQLite-to-Turso import regression tests, including migration and recovery cases.
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
