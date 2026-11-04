# Multi-stage, multi-architecture Dockerfile for actual-server (rs-budget)
# Supports linux/amd64 and linux/arm64 architectures

# Stage 1: Build binary
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

# Build actual-server binary in release mode
RUN cargo build --release --bin actual-server

# Stage 2: Runtime image
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    tzdata \
    && rm -rf /var/lib/apt/lists/*

# Create unprivileged actual user and group
RUN groupadd -g 1001 actual && \
    useradd -u 1001 -g actual -s /bin/bash -m actual

# Prepare data directory
RUN mkdir -p /data && chown -R actual:actual /data
VOLUME ["/data"]

WORKDIR /app
COPY --from=builder /usr/src/rs-budget/target/release/actual-server /app/actual-server
RUN chown actual:actual /app/actual-server

USER actual

ENV HOST=0.0.0.0
ENV PORT=5006
ENV ACTUAL_DATA_DIR=/data

EXPOSE 5006

HEALTHCHECK --interval=30s --timeout=5s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:5006/account/needs-bootstrap || exit 1

ENTRYPOINT ["/app/actual-server"]
