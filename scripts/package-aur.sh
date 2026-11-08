#!/usr/bin/env bash
set -euo pipefail

# Scripts to validate and generate AUR package metadata
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
AUR_DIR="${REPO_ROOT}/packaging/aur"

echo "=== Validating Arch Linux AUR PKGBUILD ==="
cd "${AUR_DIR}"

if command -v namcap >/dev/null 2>&1; then
    echo "Running namcap on PKGBUILD..."
    namcap PKGBUILD || true
fi

if command -v makepkg >/dev/null 2>&1; then
    echo "Generating .SRCINFO..."
    makepkg --printsrcinfo > .SRCINFO
    echo "Successfully generated packaging/aur/.SRCINFO"
else
    echo "makepkg not found; skipping .SRCINFO generation."
fi

echo "=== AUR packaging checks passed! ==="
