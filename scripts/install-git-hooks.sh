# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: 2025 Alexander Minges

#!/usr/bin/env bash
set -euo pipefail

# Install repository-provided hooks by pointing git to .githooks.
# Usage: ./scripts/install-git-hooks.sh

ROOT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"

echo "Configuring git hooks path to .githooks"
git config core.hooksPath "$ROOT_DIR/.githooks"

echo "Done. Existing hooks were not overwritten; git now reads from .githooks/"
