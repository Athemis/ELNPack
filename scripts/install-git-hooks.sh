#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: 2025 Alexander Minges

set -euo pipefail

# Install repository-provided hooks by pointing git to .githooks.
# Usage: ./scripts/install-git-hooks.sh

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"

echo "Configuring git hooks path to .githooks"
git config core.hooksPath "$ROOT_DIR/.githooks"

echo ""
echo "Git hooks installed successfully!"
echo ""
echo "The following hooks are now active:"
echo "  • commit-msg  — Validates Conventional Commits format at commit time"
echo "  • pre-commit  — Runs cargo fmt, clippy, and test before each commit"
echo "  • pre-push    — Validates all commits before pushing to remote"
echo ""
echo "All hooks use shared validation logic from validate-commit-msg.sh"
echo ""
echo "Valid commit types: feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert"
echo "Example: feat: add new feature"
echo "         fix(ui): correct button alignment"
echo ""
echo "To bypass pre-commit hook locally: SKIP_PRE_COMMIT=1 git commit"
echo "Note: commit-msg and pre-push hooks cannot be bypassed with --no-verify"
echo "      to maintain commit message quality."
