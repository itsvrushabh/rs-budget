# Future Opportunities

These ideas are intentionally outside the initial Rust parity milestone. Reconsider them only after the Rust implementation can reliably open existing budgets, preserve data, and pass the parity and interoperability gates in the [roadmap](./roadmap.md).

Potential follow-up proposals:

- Multi-currency envelope budgeting, automatic FX conversion, and historical exchange rates.
- New bank integrations or financial data providers beyond upstream-supported integrations.
- Additional audit history, point-in-time reconstruction, or rollback experiences.
- Alternative storage engines (e.g. Turso / libSQL remote replication) or serverless/edge deployment modes.
- Deployment targets or mobile platforms not supported by the pinned upstream baseline.

Each proposal should explain the user need, impact on existing files and sync, security/privacy implications, maintenance cost, and migration/rollback strategy. Treat changes to stored data or wire protocols as versioned compatibility projects, not incidental parts of a feature.

The CLI and TUI are prioritized Tier-1 surfaces in the initial release. See the [feature plan](./features.md#initial-tui-scope) and [roadmap](./roadmap.md#phase-3--prioritized-cli-tui-and-import-workflows).
