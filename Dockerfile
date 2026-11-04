# Multi-stage, multi-architecture Dockerfile for actual-server (rs-budget)
# Supports linux/amd64 and linux/arm64 architectures
# Provides an all-in-one container serving both the sync engine API and the web client PWA on port 5006.

# Stage 1: Build binaries and web distribution assets
FROM rust:1.85-slim-bookworm AS builder

WORKDIR /usr/src/rs-budget

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace source tree
COPY . .

# Build actual-server and actual-web binaries in release mode
RUN cargo build --release --bin actual-server --bin actual-web

# Generate static web distribution shell and PWA manifest
RUN /usr/src/rs-budget/target/release/actual-web export-dist --output /usr/src/rs-budget/web-dist

# Stage 2: Runtime image
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    tzdata \
    && rm -rf /var/lib/apt/lists/*

# Create unprivileged actual user and group
RUN groupadd -g 1001 actual && \
    useradd -u 1001 -g actual -s /bin/bash -m actual

# Prepare data and web directories
RUN mkdir -p /data /app/web && chown -R actual:actual /data /app
VOLUME ["/data"]

WORKDIR /app

# Copy server binary and static web assets from builder
COPY --from=builder /usr/src/rs-budget/target/release/actual-server /app/actual-server
COPY --from=builder /usr/src/rs-budget/web-dist /app/web
RUN chown -R actual:actual /app

USER actual

ENV HOST=0.0.0.0
ENV PORT=5006
ENV ACTUAL_DATA_DIR=/data
ENV ACTUAL_WEB_DIR=/app/web

EXPOSE 5006

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:5006/account/needs-bootstrap || exit 1

ENTRYPOINT ["/app/actual-server"]
