#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: 2025 Alexander Minges

# Shared validation logic for commit messages
# Used by both commit-msg and pre-push hooks to ensure consistency

set -euo pipefail

# Conventional Commits pattern (type[!][scope]: subject)
readonly COMMIT_MSG_REGEX='^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([^)]+\))?(!)?: .+'

# Valid commit types for reference
readonly VALID_TYPES="feat, fix, docs, style, refactor, perf, test, build, ci, chore, revert"

# Validate a single commit message
# Arguments:
#   $1 - commit message to validate
# Returns:
#   0 if valid, 1 if invalid
validate_commit_msg() {
    local msg="$1"

    # Skip merge commits
    if [[ "$msg" =~ ^Merge ]]; then
        return 0
    fi

    # Check if commit message follows Conventional Commits
    if [[ ! "$msg" =~ $COMMIT_MSG_REGEX ]]; then
        return 1
    fi

    return 0
}

# Print error message for invalid commit
# Arguments:
#   $1 - invalid commit message
print_error() {
    local msg="$1"

    echo "ERROR: Invalid commit message format:" >&2
    echo "  '$msg'" >&2
    echo >&2
    echo "Commit messages must follow Conventional Commits specification." >&2
    echo "Example: feat: add new feature" >&2
    echo "         fix(ui): correct button alignment" >&2
    echo >&2
    echo "Valid types: $VALID_TYPES" >&2
}
