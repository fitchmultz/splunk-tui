#!/usr/bin/env bash
# Purpose: Validate staged SPL files before commit.
# Responsibilities: Discover staged .spl files and run `splunk-cli search validate` per file.
# Non-scope: It does not format files, mutate staged content, or scan non-SPL files.
# Invariants: Handles arbitrary file names safely and returns stable exit codes.

set -euo pipefail

show_help() {
    cat <<'HELP'
pre-commit-spl-check.sh - Validate staged SPL files

USAGE:
    scripts/pre-commit-spl-check.sh [--help]

DESCRIPTION:
    Validates every staged `.spl` file using:
      splunk-cli search validate --file <path> --json

EXAMPLES:
    # Run manually before committing
    scripts/pre-commit-spl-check.sh

    # Install as git pre-commit hook
    ln -s ../../scripts/pre-commit-spl-check.sh .git/hooks/pre-commit

EXIT CODES:
    0   Success (no SPL files or all validations passed)
    1   One or more SPL files failed validation
    2   Usage error
HELP
}

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
    show_help
    exit 0
fi

if [[ $# -gt 0 ]]; then
    echo "Invalid arguments. Use --help for usage." >&2
    exit 2
fi

echo "Validating SPL files..."

# Use NUL-delimited output to preserve arbitrary paths safely.
mapfile -d '' spl_files < <(git diff --cached --name-only -z --diff-filter=ACM -- '*.spl')

if [[ ${#spl_files[@]} -eq 0 ]]; then
    echo "No SPL files to validate."
    exit 0
fi

failed=0
for file in "${spl_files[@]}"; do
    echo "  Validating: $file"
    if ! splunk-cli search validate --file "$file" --json > /dev/null 2>&1; then
        echo "    FAILED: $file"
        splunk-cli search validate --file "$file" 2>&1 || true
        failed=1
    else
        echo "    PASSED: $file"
    fi
done

if [[ $failed -eq 1 ]]; then
    echo
    echo "SPL validation failed. Please fix the errors before committing."
    exit 1
fi

echo "All SPL files passed validation."
exit 0
