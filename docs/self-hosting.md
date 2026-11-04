# Self-Hosting Guide for rs-budget

`rs-budget` is a 100% Rust rewrite of Actual Budget designed from the ground up for self-hosters and home labs. It provides lightweight resource usage (<15MB RAM idle), native SQLite WAL performance, end-to-end encrypted CRDT sync, and a single-port all-in-one container serving both the sync backend and the WebAssembly PWA interface.

---

## Table of Contents

1. [Architecture & Port Conventions](#1-architecture--port-conventions)
2. [Quick Start (Docker Run)](#2-quick-start-docker-run)
3. [Production Docker Compose](#3-production-docker-compose)
4. [Reverse Proxy Configuration](#4-reverse-proxy-configuration)
   - [Caddy (Automatic HTTPS)](#caddy-automatic-https)
   - [Traefik v3](#traefik-v3)
   - [Nginx](#nginx)
5. [Podman Quadlet (Rootless Systemd)](#5-podman-quadlet-rootless-systemd)
6. [Kubernetes (Helm Chart)](#6-kubernetes-helm-chart)
7. [Self-Hosted Ecosystem App Stores](#7-self-hosted-ecosystem-app-stores)
   - [Umbrel](#umbrel)
   - [CasaOS / ZimaOS](#casaos--zimaos)
   - [Unraid](#unraid)
   - [Coolify](#coolify)
8. [Configuration & Environment Variables](#8-configuration--environment-variables)
9. [Backup & Disaster Recovery](#9-backup--disaster-recovery)

---

## 1. Architecture & Port Conventions

| Component | Port | Path | Description |
|---|---|---|---|
| **Sync Server & API** | `5006` | `/sync/*`, `/account/*`, `/user/*` | Axum HTTP server handling authentication, key exchange, and CRDT sync |
| **Web Client (PWA)** | `5006` | `/`, `/index.html`, `/manifest.webmanifest` | WebAssembly client served directly by `actual-server` when `ACTUAL_WEB_DIR=/app/web` |
| **Data Directory** | - | `/data` | Persistent storage containing SQLite database (`account.sqlite`) and encrypted budget archives |
| **Process Security** | - | User `actual` (UID `1001`, GID `1001`) | Runs as an unprivileged user inside the container |

---

## 2. Quick Start (Docker Run)

To run the all-in-one container immediately:

```bash
docker run -d \
  --name actual-server \
  --restart unless-stopped \
  -p 5006:5006 \
  -v actual-data:/data \
  ghcr.io/itsvrushabh/rs-budget-server:latest
```

Open your browser to `http://localhost:5006` to complete the initial setup and create your master password.

---

## 3. Production Docker Compose

A production-ready [`docker-compose.yml`](file:///home/cachyos/Work/rs-budget/docker-compose.yml) is included at the root of the repository:

```yaml
services:
  actual-server:
    image: ghcr.io/itsvrushabh/rs-budget-server:latest
    container_name: actual-server
    restart: unless-stopped
    ports:
      - "${ACTUAL_PORT:-5006}:5006"
    environment:
      - PORT=5006
      - HOST=0.0.0.0
      - ACTUAL_DATA_DIR=/data
      - ACTUAL_WEB_DIR=/app/web
      - ACTUAL_SERVER_PASSWORD=${ACTUAL_SERVER_PASSWORD:-}
    volumes:
      - actual-data:/data
    healthcheck:
      test: ["CMD-SHELL", "curl -f http://localhost:5006/account/needs-bootstrap || exit 1"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 10s
    security_opt:
      - no-new-privileges:true

volumes:
  actual-data:
    name: actual-data
```

Start the stack:

```bash
docker compose up -d
```

---

## 4. Reverse Proxy Configuration

When hosting `rs-budget` on a public domain or internal LAN with TLS, use a reverse proxy. Ready-to-use recipes are provided in [`examples/compose/`](file:///home/cachyos/Work/rs-budget/examples/compose/):

### Caddy (Automatic HTTPS)

See [`examples/compose/caddy/`](file:///home/cachyos/Work/rs-budget/examples/compose/caddy/):

```caddyfile
budget.example.com {
    encode gzip zstd
    reverse_proxy actual-server:5006
}
```

### Traefik v3

See [`examples/compose/traefik/`](file:///home/cachyos/Work/rs-budget/examples/compose/traefik/):

```yaml
labels:
  - "traefik.enable=true"
  - "traefik.http.routers.actual.rule=Host(`budget.example.com`)"
  - "traefik.http.routers.actual.entrypoints=websecure"
  - "traefik.http.routers.actual.tls.certresolver=myresolver"
  - "traefik.http.services.actual.loadbalancer.server.port=5006"
```

### Nginx

See [`examples/compose/nginx/`](file:///home/cachyos/Work/rs-budget/examples/compose/nginx/):

Ensure `client_max_body_size 50M;` is configured to allow large encrypted budget backups.

---

## 5. Podman Quadlet (Rootless Systemd)

For Red Hat Enterprise Linux, Fedora, and Debian/Ubuntu rootless Podman users, a systemd Quadlet configuration is provided in [`deploy/podman/actual-server.container`](file:///home/cachyos/Work/rs-budget/deploy/podman/actual-server.container):

```bash
mkdir -p ~/.config/containers/systemd
cp deploy/podman/actual-server.container ~/.config/containers/systemd/
systemctl --user daemon-reload
systemctl --user start actual-server
systemctl --user enable actual-server
```

---

## 6. Kubernetes (Helm Chart)

A Helm chart is located in [`charts/rs-budget/`](file:///home/cachyos/Work/rs-budget/charts/rs-budget/):

```bash
# Install with default values
helm install rs-budget ./charts/rs-budget \
  --set persistence.size=10Gi \
  --set ingress.enabled=true \
  --set ingress.hosts[0].host=budget.mycluster.internal
```

---

## 7. Self-Hosted Ecosystem App Stores

- **Umbrel OS**: Installable via the Community Store using [`deploy/umbrel/umbrel-app.yml`](file:///home/cachyos/Work/rs-budget/deploy/umbrel/umbrel-app.yml).
- **CasaOS / ZimaOS**: Import [`deploy/casaos/rs-budget.json`](file:///home/cachyos/Work/rs-budget/deploy/casaos/rs-budget.json) directly into Custom Install.
- **Unraid**: Copy template from [`deploy/unraid/rs-budget.xml`](file:///home/cachyos/Work/rs-budget/deploy/unraid/rs-budget.xml) to `/boot/config/plugins/dockerMan/templates-user/`.
- **Coolify**: Deploy with 1-click using the compose file in [`deploy/coolify/docker-compose.yml`](file:///home/cachyos/Work/rs-budget/deploy/coolify/docker-compose.yml).

---

## 8. Configuration & Environment Variables

| Variable | Default | Description |
|---|---|---|
| `PORT` | `5006` | HTTP listening port |
| `HOST` | `0.0.0.0` | Bind IP address (`0.0.0.0` for all interfaces, `127.0.0.1` for local only) |
| `ACTUAL_DATA_DIR` | `/data` | Path to budget files and SQLite databases |
| `ACTUAL_WEB_DIR` | `/app/web` | Path to static WebAssembly/HTML assets for all-in-one serving |
| `ACTUAL_SERVER_PASSWORD` | `""` | Optional master password pre-seed (omit for interactive UI setup) |
| `RUST_LOG` | `actual_server=info` | Tracing log filter (`error`, `warn`, `info`, `debug`, `trace`) |

---

## 9. Backup & Disaster Recovery

`rs-budget` stores its data using SQLite in Write-Ahead Log (WAL) mode. Do not directly copy `account.sqlite` while the container is running without using SQLite's online backup API or safe snapshotting:

### Automated Backup Script

```bash
#!/usr/bin/env bash
set -euo pipefail

BACKUP_DIR="/backups/rs-budget"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
mkdir -p "${BACKUP_DIR}"

# 1. Safe SQLite snapshot of server database
docker exec actual-server sqlite3 /data/account.sqlite ".backup '${BACKUP_DIR}/account_${TIMESTAMP}.sqlite'"

# 2. Backup user file archives
tar -czf "${BACKUP_DIR}/user_files_${TIMESTAMP}.tar.gz" -C /var/lib/docker/volumes/actual-data/_data user-files

# 3. Retain last 30 days
find "${BACKUP_DIR}" -type f -mtime +30 -delete

echo "Backup completed successfully at ${TIMESTAMP}"
```
