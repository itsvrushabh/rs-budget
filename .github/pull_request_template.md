## Associated Issue

<!-- MANDATORY: Every PR must link to a valid GitHub Issue ID. -->
<!-- Examples: Closes #123, Fixes #456, Relates to #789 -->
Closes #

## Summary of Changes

<!-- Provide a concise description of the changes introduced by this PR. -->

## Target Branch

<!-- Select one target branch: -->
- [ ] `dev` (Features, bugfixes, refactoring)
- [ ] `release` (Release candidate staging & automation)
- [ ] `main` (Production hotfix)
- [ ] `planning` (Architecture & planning documentation)
- [ ] `ci-workflow` (CI pipelines, automation, developer tooling)

## Verification & Quality Checklist

<!-- Ensure all applicable checks pass before submitting: -->
- [ ] Linked to a valid GitHub Issue ID above.
- [ ] Code formatted with `cargo fmt --check`.
- [ ] Clippy passes with zero warnings: `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- [ ] Unit and integration tests pass cleanly (`cargo test`).
- [ ] Upstream parity / differential tests verified (if modifying financial calculations or sync).
- [ ] SQLite compatibility and data safety verified against test budgets (if modifying storage).
- [ ] Documentation updated to reflect changes.
