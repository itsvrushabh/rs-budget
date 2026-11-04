# rs-budget Configuration Reference

Comprehensive reference guide for environment variables, server settings, CLI flags, desktop preferences, and automated bank sync credentials.

---

## 1. Sync Server Configuration (`actual-server`)

The self-hosted Axum server can be configured using environment variables or a `.env` file:

| Environment Variable | Default Value | Description |
|---|---|---|
| `ACTUAL_PORT` | `5006` | HTTP listening port for sync API endpoints. |
| `ACTUAL_HOST` | `0.0.0.0` | Network interface binding IP address. |
| `ACTUAL_DATA_DIR` | `./user-files` | Root directory for SQLite databases, blob storage, and user credentials. |
| `ACTUAL_SERVER_FILES_DIR` | `./server-files` | Storage directory for global server metadata and user authentication tables. |
| `ACTUAL_UPLOAD_LIMIT_MB` | `20` | Maximum file upload size in megabytes for `.actual` archives. |
| `ACTUAL_MULTIUSER` | `false` | Enable multi-user account registration (set to `true` to allow multiple accounts). |
| `ACTUAL_PASSWORD` | *None* | Initial master password for single-user bootstrap on first launch. |
| `RUST_LOG` | `info` | Logging verbosity filter (`error`, `warn`, `info`, `debug`, `trace`). |

### Docker Compose Example
```yaml
version: '3.8'

services:
  actual-server:
    image: itsvrushabh/rs-budget-server:latest
    container_name: rs-budget-server
    restart: unless-stopped
    ports:
      - "5006:5006"
    environment:
      - ACTUAL_PORT=5006
      - ACTUAL_DATA_DIR=/data
      - ACTUAL_MULTIUSER=true
      - RUST_LOG=info
    volumes:
      - ./budget-data:/data
```

---

## 2. CLI Configuration (`actual-cli` / `actual`)

The CLI utility supports global flags and command options:

```bash
actual [OPTIONS] <COMMAND>

Global Options:
  -d, --db <PATH>         Path to local SQLite budget file [default: ./budget.sqlite]
  -j, --json              Output machine-readable JSON for piping and scripting
  -h, --help              Print help information
  -V, --version           Print version information

Commands:
  account                 Manage accounts (create, list, rename, balance)
  transaction             Add, list, search, or delete transactions
  budget                  Inspect envelope balances, allocations, and leftovers
  import                  Import statement files (CSV, OFX, QIF)
  export                  Export budget to JSON or CSV
  sync                    Trigger server synchronization cycle
```

---

## 3. Terminal TUI Keybindings (`actual-tui`)

| Keybinding | Action |
|---|---|
| `Tab` | Switch between Budget View, Account Register, and Transactions list |
| `j` / `Down` | Move selection down |
| `k` / `Up` | Move selection up |
| `h` / `Left` | Navigate to previous month |
| `l` / `Right` | Navigate to next month |
| `e` | Edit selected budget category allocation |
| `a` | Add new transaction to active account |
| `c` | Toggle category carryover rollover |
| `/` | Search / filter payees and notes |
| `?` | Toggle Help modal overlay |
| `q` / `Esc` | Exit / close modal |

---

## 4. Bank Sync Credentials (`actual-import`)

Credentials for automated bank sync are encrypted with AES-256-GCM before storage:

### SimpleFIN Bridge
- **Access URL**: Configured with the base64 claim token provided by SimpleFIN Bridge.
- **Fetch Interval**: Recommended delta sync every 4 to 12 hours.

### GoCardless (Nordigen Open Banking)
- **Secret ID**: Open Banking developer API key.
- **Secret Key**: Open Banking developer API secret.
- **Institution ID**: Bank identifier (e.g. `SANDBOXFINANCE_SFIN0000`).
