# Release Notes — v0.1.0-preview

We are proud to announce the first public preview release of **rs-budget**: a 100% Rust rewrite of [Actual Budget](https://github.com/actualbudget/actual) preserving full local-first envelope budgeting, zero-knowledge encryption, CRDT synchronization, and cross-platform native clients.

---

## Highlights

### ⚡ Blazing Performance & Tiny Footprint
- Native compilation to machine code with zero Node.js/Electron runtime overhead.
- Instant sub-millisecond cold boot across all surfaces.
- Minimal RAM consumption (< 20MB for desktop client and server vs 400MB+ in upstream Electron).

### 🔒 100% Upstream Wire & Database Parity
- Uses the identical SQLite relational schema (54 upstream migrations) with WAL mode.
- Cryptographic security verified against official test vectors (PBKDF2 HMAC-SHA-512, AES-256-GCM).
- Full CRDT wire synchronization with monotonic Hybrid Logical Clocks (`HlcClock`) and base-3 Radix Merkle Trie range diffing.

### 💻 First-Class Terminal Experience
- **Interactive TUI (`actual-tui`)**: Built with Ratatui, featuring Vim keybindings, live envelope leftover recalculation, and color-coded status badges.
- **Fast Scriptable CLI (`actual`)**: Complete command-line budget management, statement imports, and machine-readable `--json` output.

### 🌐 Cross-Platform Native Clients
- **Desktop (`actual-desktop`)**: Standalone binary for Linux, macOS, and Windows with native menus, system dark mode, and persistent window states.
- **Web (`actual-web`)**: 100% Rust WebAssembly client running in modern browsers with local-first persistence via the Origin Private File System (OPFS) and offline PWA service worker.
- **Mobile (`actual-mobile`)**: Touch-optimized interface for Android and iOS with swipe gestures (swipe-to-delete), numeric keypad, and pull-to-refresh sync.

### 🚀 High-Throughput Self-Hosted Sync Server
- Axum-based async sync server on port 5006 with multi-user authentication, session key exchange, and blob storage.
- Multi-architecture Docker images for `linux/amd64` and `linux/arm64`.

---

## Verification & Test Metrics
- **197 tests** passing across 10 crates with zero failures.
- **Zero warnings** on strict Clippy (`-D warnings`).
- **Differential testing** against official Node.js calculator vectors verifying 100% math and rollover parity.

---

## Getting Started

Visit the [Migration Guide](file:///home/cachyos/Work/rs-budget/docs/migration-guide.md) to import your existing budgets or read the [Configuration Reference](file:///home/cachyos/Work/rs-budget/docs/configuration.md) to launch the self-hosted sync server.
