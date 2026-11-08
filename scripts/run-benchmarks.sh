#!/usr/bin/env bash
set -euo pipefail

# Automated benchmark runner for rs-budget Criterion benchmarks
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "=== Running rs-budget Criterion Benchmarks ==="
cd "${REPO_ROOT}"

echo ">> Benchmarking actual-core (Cents arithmetic, Envelope cascade, Rules matching, Spreadsheet DAG)..."
cargo bench -p actual-core --bench core_benchmarks -- --noplot

echo ">> Benchmarking actual-sync (HLC clock, Merkle Trie insert/diff, Protobuf codecs)..."
cargo bench -p actual-sync --bench sync_benchmarks -- --noplot

echo "=== Benchmark suite executed successfully! ==="
