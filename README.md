# rs-budget

[![Continuous Integration](https://github.com/itsvrushabh/rs-budget/actions/workflows/ci.yml/badge.svg)](https://github.com/itsvrushabh/rs-budget/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust: 2021](https://img.shields.io/badge/Rust-2021_Edition-orange.svg)](https://www.rust-lang.org/)
[![Upstream Parity](https://img.shields.io/badge/Upstream_Parity-100%25-brightgreen.svg)](docs/parity-matrix.md)

A local-first, 100% Rust rewrite of [Actual Budget](https://github.com/actualbudget/actual) (`v26.11.0`, commit `1766241e3f4b3c578e424c4fa15f1eb83c0da17d`).

`rs-budget` preserves Actual's envelope budgeting workflows, native SQLite budget files, end-to-end encryption, and CRDT synchronization, rewritten entirely in Rust for blistering performance, zero Node.js/Electron memory overhead, and a first-class terminal experience (CLI & interactive TUI) alongside native desktop, web (WASM/OPFS), and mobile clients.

---

## 📸 Demo Screens & Visual Previews

### 1. Interactive Terminal UI (`actual-tui`)

An interactive, responsive terminal UI built with [Ratatui](https://github.com/ratatui/ratatui) featuring full keyboard navigation, live envelope rollover calculations, and instant register editing:

```text
┌─ Actual Budget TUI ──────────────────────────────────── 2026-11 ─┐
│ Month: < 2026-11 >     To Budget: $450.00      Available: $3,250.00│
├───────────────────────────────────────────────────────────────────┤
│ Category Group / Name       Budgeted      Activity      Leftover  │
│ ▼ Living Expenses                                                 │
│   ● Groceries                $500.00      -$345.50       $154.50  │
│   ● Rent                   $1,200.00    -$1,200.00         $0.00  │
│   ● Electricity              $120.00       -$85.00        $35.00  │
│   ● Internet                  $70.00       -$70.00         $0.00  │
│ ▼ Discretionary                                                   │
│   ● Dining Out               $150.00      -$182.00       -$32.00  │
│   ● Subscriptions             $45.00       -$45.00         $0.00  │
├───────────────────────────────────────────────────────────────────┤
│ [Tab] Switch View  [h/l] Month  [j/k] Select  [e] Edit  [a] Add   │
│ [c] Carryover      [/] Filter   [?] Help      [q] Quit            │
└───────────────────────────────────────────────────────────────────┘
```

### 2. Fast Scriptable CLI (`actual`)

Inspect envelopes, add transactions, and query financial reports with human-readable tables or machine-readable JSON for scripting:

```bash
$ actual --db budget.sqlite budget inspect --month 2026-11
┌──────────────────┬─────────────┬─────────────┬─────────────┐
│ Category         │    Budgeted │    Activity │    Leftover │
├──────────────────┼─────────────┼─────────────┼─────────────┤
│ Groceries        │     $500.00 │    -$345.50 │     $154.50 │
│ Rent             │   $1,200.00 │  -$1,200.00 │       $0.00 │
│ Electricity      │     $120.00 │     -$85.00 │      $35.00 │
│ Dining Out       │     $150.00 │    -$182.00 │     -$32.00 │
└──────────────────┴─────────────┴─────────────┴─────────────┘
To Budget: $450.00 | Total Available: $3,250.00

$ actual --db budget.sqlite transaction list --json
[
  {
    "id": "tx-101",
    "account": "Checking",
    "date": "2026-11-04",
    "amount": -54.20,
    "payee": "Supermarket Express",
    "category": "Groceries",
    "cleared": true
  }
]
```

---

## 🚀 Quick Start & How to Use

### Prerequisites & Building from Source

Ensure you have a modern Rust toolchain installed (Rust 1.80+):

```bash
# Clone the repository
git clone https://github.com/itsvrushabh/rs-budget.git
cd rs-budget

# Build all release binaries across the workspace
cargo build --release
```

All binaries will be available in `./target/release/`:
- `actual-tui` — Interactive Ratatui terminal user interface.
- `actual` — Non-interactive scriptable CLI.
- `actual-desktop` — Native graphical desktop client.
- `actual-web` — WebAssembly PWA generator and manager.
- `actual-server` — Self-hosted Axum CRDT synchronization server.

---

### 1. Running the Interactive Terminal UI (`actual-tui`)

```bash
# Open an existing SQLite budget or create a new one:
cargo run --release -p actual-tui -- /path/to/my-budget.sqlite

# Or run the compiled binary directly:
./target/release/actual-tui ./budget.sqlite
```

#### TUI Keyboard Shortcuts

| Shortcut | Description |
|---|---|
| `Tab` | Switch between Budget View, Account Register, and Transactions list |
| `j` / `Down` | Move cursor down |
| `k` / `Up` | Move cursor up |
| `h` / `Left` | Navigate to previous month |
| `l` / `Right` | Navigate to next month |
| `e` | Edit budgeted amount for the selected category |
| `a` | Add a new transaction |
| `c` | Toggle carryover rollover flag for the selected category |
| `/` | Search / filter payees and notes |
| `?` | Display Help & Keybinding cheat sheet modal |
| `q` / `Esc` | Quit TUI or close modal |

---

### 2. Using the Headless CLI (`actual`)

```bash
# Display general help and subcommands
./target/release/actual --help

# Create a new checking account
./target/release/actual --db budget.sqlite account create --name "Checking"

# List all accounts in JSON format
./target/release/actual --db budget.sqlite account list --json

# Add an expense transaction
./target/release/actual --db budget.sqlite transaction add \
  --account "Checking" \
  --date 2026-11-04 \
  --amount -35.50 \
  --payee "Supermarket" \
  --category "Groceries"

# Inspect current envelope allocations
./target/release/actual --db budget.sqlite budget inspect --month 2026-11

# Import bank statements (CSV, OFX, or QIF)
./target/release/actual --db budget.sqlite import \
  --account "Checking" \
  --file statement.csv
```

---

### 3. Running the Native Desktop Client (`actual-desktop`)

```bash
# Launch native desktop application:
./target/release/actual-desktop

# Launch with a specific budget file:
./target/release/actual-desktop --budget-file ./budget.sqlite

# Run self-diagnostic check:
./target/release/actual-desktop --headless-check
```

---

### 4. Serving the Web Client (`actual-web`)

The browser client runs 100% in WebAssembly using the browser's **Origin Private File System (OPFS)** for local-first SQLite persistence and offline PWA service worker caching:

```bash
# Export static distribution assets (index.html, manifest.webmanifest, sw.js):
./target/release/actual-web export-dist --output ./dist

# Validate PWA manifest and Service Worker compliance:
./target/release/actual-web verify-pwa
```

---

### 5. Running the Self-Hosted Sync Server (`actual-server`)

The 100% Rust self-hosted synchronization server runs on **port 5006** using Axum:

```bash
# Run via Cargo:
cargo run --release -p actual-server

# Run via Docker Compose (all-in-one sync server + web client):
docker compose up -d

# Or with Docker run:
docker run -d \
  --name actual-server \
  -p 5006:5006 \
  -v ./budget-data:/data \
  ghcr.io/itsvrushabh/rs-budget-server:latest
```

For advanced deployments (Caddy, Traefik, Nginx, Podman Quadlet, Kubernetes Helm, Umbrel, CasaOS, Unraid, and Coolify), refer to the comprehensive [Self-Hosting Guide](docs/self-hosting.md).

---

## 💡 Common Workflows

### 1. Migrating an Existing Budget
`rs-budget` uses the exact SQLite schema and migration sequence as upstream Actual Budget. You can open your existing `db.sqlite` directly:
```bash
actual-tui ~/.config/Actual/my-budget/db.sqlite
```
For encrypted budgets (`.actual` archives) and server cutover instructions, see the [Migration Guide](docs/migration-guide.md).

### 2. Automated Bank Sync
Configure automated delta synchronization via SimpleFIN Bridge or GoCardless (Nordigen Open Banking) with AES-256-GCM encrypted credentials:
```bash
actual --db budget.sqlite sync bank --provider simplefin
```

### 3. Multi-Device End-to-End Encrypted Sync
Sync changes in real-time across your desktop, laptop, terminal, and mobile clients with deterministic Last-Write-Wins (LWW) conflict resolution:
```bash
actual --db budget.sqlite sync --server http://localhost:5006 --token "secret"
```

---

## 📦 Workspace Crates

| Crate | Description |
|---|---|
| [`actual-core`](crates/actual-core) | Domain entities, fixed-point integer `Cents`, rules engine, schedules, and reports. |
| [`actual-storage`](crates/actual-storage) | SQLite storage engine, WAL pragmas, and all 54 upstream migrations. |
| [`actual-crypto`](crates/actual-crypto) | PBKDF2 HMAC-SHA-512, AES-256-GCM cipher, `.actual` ZIP archives, and key rotation. |
| [`actual-sync`](crates/actual-sync) | Protobuf codecs, monotonic HLC clock, trinary Radix Merkle trie, and CRDT tracker. |
| [`actual-import`](crates/actual-import) | Statement parsers (CSV, OFX, QIF), payee cleaner, deduplication, and bank sync. |
| [`actual-api`](crates/actual-api) | Typed application service, 28 command handlers, and 200-depth undo/redo manager. |
| [`actual-cli`](crates/actual-cli) | Fast scriptable CLI (`actual`) with human and `--json` outputs. |
| [`actual-tui`](crates/actual-tui) | Full-featured interactive terminal user interface built with Ratatui. |
| [`actual-server`](crates/actual-server) | 100% Rust Axum sync server on port 5006 with multi-user auth and Dockerfile. |
| [`actual-gui`](crates/actual-gui) | Cross-platform graphical components: Desktop, Web (WASM/OPFS), and Mobile. |

---

## 📚 Documentation & Guides

- [Upstream Parity Matrix](docs/parity-matrix.md) — 100% upstream feature parity breakdown across all 6 phases.
- [Self-Hosting Guide](docs/self-hosting.md) — Complete guide for Docker Compose, Podman, Kubernetes Helm, Umbrel, CasaOS, and reverse proxies.
- [Migration Guide](docs/migration-guide.md) — Instructions for migrating existing budgets and sync servers.
- [Configuration Reference](docs/configuration.md) — Server environment variables, CLI commands, and TUI keybindings.
- [Release Notes (v0.1.0-preview)](docs/release-notes-v0.1.0-preview.md) — Public preview release details.

---

## 🗺️ Planning & Architecture

All architectural specifications, feature matrices, implementation roadmaps, and CI/release workflows are maintained on the dedicated [`planning`](https://github.com/itsvrushabh/rs-budget/tree/planning) branch:

- [Planning Index (`plan.md`)](https://github.com/itsvrushabh/rs-budget/blob/planning/plan.md)
- [Feature Parity Plan](https://github.com/itsvrushabh/rs-budget/blob/planning/plans/features.md)
- [Architecture & Compatibility Plan](https://github.com/itsvrushabh/rs-budget/blob/planning/plans/architecture.md)
- [Implementation Roadmap](https://github.com/itsvrushabh/rs-budget/blob/planning/plans/roadmap.md)
- [CI, PR Review & Release Plan](https://github.com/itsvrushabh/rs-budget/blob/planning/plans/ci-release.md)
- [Future Opportunities](https://github.com/itsvrushabh/rs-budget/blob/planning/plans/future.md)

---

## 🌿 Repository Branches

- `main` — Production-ready codebase and preview releases.
- `dev` — Primary integration and active development branch.
- `release` — Automated release pipeline branch for GitHub Actions.
- `planning` — Architecture, technical specifications, and planning documents.
- `ci-workflow` — CI pipelines, workflows, build scripts, and developer tooling.

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).
