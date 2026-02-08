#!/bin/bash
# Pre-commit hook for validating SPL files
#
# Install: ln -s ../../scripts/pre-commit-spl-check.sh .git/hooks/pre-commit
#
# This hook validates all .spl files before allowing a commit.
# Requires splunk-cli to be installed and configured.

set -e

echo "Validating SPL files..."

# Find all staged .spl files
spl_files=$(git diff --cached --name-only --diff-filter=ACM | grep '\.spl$' || true)

if [ -z "$spl_files" ]; then
    echo "No SPL files to validate."
    exit 0
fi

# Validate each file
failed=0
for file in $spl_files; do
    echo "  Validating: $file"
    if ! splunk-cli search validate --file "$file" --json > /dev/null 2>&1; then
        echo "    ✗ FAILED: $file"
        # Show errors
        splunk-cli search validate --file "$file" 2>&1 || true
        failed=1
    else
        echo "    ✓ PASSED: $file"
    fi
done

if [ $failed -eq 1 ]; then
    echo ""
    echo "SPL validation failed. Please fix the errors before committing."
    exit 1
fi

echo "All SPL files passed validation."
exit 0
