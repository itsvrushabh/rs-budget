# Upstream to rs-budget Migration Guide

Step-by-step instructions for seamlessly migrating your existing budgets and synchronization workflows from upstream **Actual Budget** (`v26.11.0`) to **rs-budget**.

---

## Overview

`rs-budget` maintains strict 100% binary and schema compatibility with Actual Budget:
- **Zero Schema Conversion Needed**: Existing SQLite `.sqlite` files and `.actual` archives can be opened directly without data loss.
- **Identical CRDT Sync Protocol**: The Axum server in `rs-budget` speaks the exact same protobuf sync wire protocol as the upstream Node.js sync server.
- **Multi-Client Support**: You can mix and match CLI, TUI, Desktop, Web, and Mobile clients seamlessly.

---

## Migration Options

### Option A: Direct SQLite File Migration (Recommended for Local-First Users)

If you have a local `.sqlite` database file from upstream Actual Budget:

1. **Locate your existing database file**:
   - Upstream Actual Desktop stores databases under:
     - Linux: `~/.config/Actual/` or `~/.local/share/Actual/`
     - macOS: `~/Library/Application Support/Actual/`
     - Windows: `%APPDATA%\Actual\`
2. **Open directly with `rs-budget` CLI or TUI**:
   ```bash
   # Open with interactive TUI:
   actual-tui /path/to/your/db.sqlite

   # Or inspect accounts via CLI:
   actual --db /path/to/your/db.sqlite account list
   ```
3. **Verify budget integrity**:
   - Check that all account balances, category leftover amounts, and payee histories match your previous installation.

---

### Option B: Export / Import via `.actual` Encrypted Archive

If your budget is encrypted with a master password or hosted on a remote server:

1. **Export your budget from upstream Actual**:
   - In upstream Actual GUI, navigate to `Settings` -> `Export`.
   - Download the `.zip` or `.actual` archive file.
2. **Import via CLI**:
   ```bash
   # Unpack and verify the encrypted archive
   actual-crypto unpack my-budget.actual --key "your-password" --out ./migrated-budget/
   ```
3. **Open with `actual-desktop` or `actual-tui`**:
   ```bash
   actual-desktop --budget-file ./migrated-budget/db.sqlite
   ```

---

### Option C: Cutover Self-Hosted Sync Server

If you self-host the upstream Node.js sync server (`actual-server`) and wish to upgrade to the high-performance 100% Rust `rs-budget` Axum server:

1. **Stop the old Node.js container**:
   ```bash
   docker stop actual-server
   ```
2. **Run the `rs-budget` Axum server**:
   ```bash
   docker run -d \
     --name rs-budget-server \
     -p 5006:5006 \
     -v /path/to/actual-data:/data \
     -e ACTUAL_PORT=5006 \
     -e ACTUAL_DATA_DIR=/data \
     itsvrushabh/rs-budget-server:latest
   ```
3. **Point your clients to the new sync server**:
   - All existing authentication tokens and Merkle trie logs continue synchronizing without a reset or re-sync requirement.

---

## Verification & Rollback Safeguards

1. **Automated Safety Backups**:
   - `rs-budget` automatically triggers a backup snapshot on first open.
2. **Rollback Procedure**:
   - Because `rs-budget` uses purely additive SQLite migrations, your SQLite file remains compatible with upstream Actual Budget `v26.11.0` if you ever need to roll back.
