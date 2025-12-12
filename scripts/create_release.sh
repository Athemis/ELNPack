#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: 2025 Alexander Minges

# Helper for manual, tag-based releases.
# - Bumps workspace version via cargo-edit
# - Optionally commits and tags
# Usage: scripts/create_release.sh <version> [--no-commit] [--no-tag]

set -euo pipefail

usage() {
    echo "Usage: $0 <version> [--no-commit] [--no-tag]" >&2
    exit 1
}

[[ $# -ge 1 ]] || usage
VERSION="$1"
shift
TAG="v${VERSION}"
CREATE_COMMIT=1
CREATE_TAG=1

while [[ $# -gt 0 ]]; do
    case "$1" in
    --no-commit) CREATE_COMMIT=0 ;;
    --no-tag) CREATE_TAG=0 ;;
    *) usage ;;
    esac
    shift
done

if ! command -v cargo >/dev/null 2>&1; then
    echo "cargo is required" >&2
    exit 1
fi
if ! cargo set-version --help >/dev/null 2>&1; then
    echo "cargo-edit missing: install with 'cargo install cargo-edit'" >&2
    exit 1
fi
if ! git diff --quiet --stat; then
    echo "Working tree not clean; stash/commit before bumping." >&2
    exit 1
fi

echo "Setting workspace version to ${VERSION}"
cargo set-version --workspace "${VERSION}"

git status --short

if ((CREATE_COMMIT)); then
    git commit -am "chore(release): ${TAG}"
fi

if ((CREATE_TAG)); then
    git tag "${TAG}"
fi

echo "Done. Push with: git push && git push origin ${TAG}"
