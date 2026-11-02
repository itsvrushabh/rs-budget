#!/usr/bin/env bash
# Regenerates golden test vector fixtures using upstream Actual reference calculator.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "==> Generating golden envelope budget fixtures..."
node "${ROOT_DIR}/tools/golden-generator/generate_scenarios.js"
echo "==> Golden fixtures successfully regenerated in fixtures/budgets/"
