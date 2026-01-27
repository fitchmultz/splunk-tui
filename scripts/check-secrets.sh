#!/usr/bin/env bash
# Secret-commit guard (local security gate)
#
# RESPONSIBILITY:
#   Fail fast if sensitive/local-only files are tracked in git.
#
# DOES NOT:
#   - Scan file contents for secrets
#   - Automatically untrack/remove files
#   - Replace a real secret scanner (this is a targeted guardrail)
#
# WHY:
#   .gitignore does not apply to files already tracked. This script ensures
#   `make ci` fails if forbidden files are still in the git index.

set -euo pipefail

# Respect NO_COLOR (https://no-color.org/) and non-tty output
if [[ -t 1 && -z "${NO_COLOR:-}" ]]; then
  RED='\033[0;31m'
  GREEN='\033[0;32m'
  YELLOW='\033[1;33m'
  BLUE='\033[0;34m'
  NC='\033[0m'
else
  RED=''
  GREEN=''
  YELLOW=''
  BLUE=''
  NC=''
fi

show_help() {
  cat <<'EOF'
USAGE:
  scripts/check-secrets.sh [--help]

DESCRIPTION:
  Secret-commit guard that fails if certain sensitive/local-only paths are TRACKED in git.
  This prevents accidental commits of real credentials or private environment details.

FORBIDDEN TRACKED PATHS:
  - .env
  - .env.test
  - docs/splunk-test-environment.md
  - rust_out (and anything under rust_out/)

NOTES:
  - This checks only what is TRACKED (git index), not untracked local files.
  - .gitignore will NOT protect you if the file was committed previously.

EXAMPLES:
  # Run the guard
  scripts/check-secrets.sh

  # Run via Makefile target
  make lint-secrets

REMEDIATION (if it fails):
  1) Untrack the files (keep local copies):
     git rm --cached -- .env .env.test docs/splunk-test-environment.md rust_out

  2) Commit the removals:
     git commit -m "chore(security): stop tracking local secret files"

  3) Re-run:
     make lint-secrets
EOF
}

# Parse arguments
for arg in "$@"; do
  case "$arg" in
    --help|-h)
      show_help
      exit 0
      ;;
    *)
      echo -e "${RED}Error:${NC} Unknown argument '$arg'"
      echo "Run 'scripts/check-secrets.sh --help' for usage."
      exit 2
      ;;
  esac
done

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo -e "${RED}Error:${NC} Not inside a git repository."
  exit 2
fi

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

declare -a forbidden_tracked=()

# Single authoritative list of tracked files (NUL-separated for safety).
while IFS= read -r -d '' path; do
  case "$path" in
    ".env" | ".env.test" | "docs/splunk-test-environment.md" | "rust_out" | rust_out/*)
      forbidden_tracked+=("$path")
      ;;
  esac
done < <(git ls-files -z)

if [[ "${#forbidden_tracked[@]}" -gt 0 ]]; then
  echo -e "${RED}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo -e "${RED}Secret-commit guard FAILED:${NC} forbidden paths are TRACKED in git."
  echo -e "${RED}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo
  echo -e "${YELLOW}Tracked forbidden paths:${NC}"
  for p in "${forbidden_tracked[@]}"; do
    echo "  - $p"
  done
  echo
  echo -e "${YELLOW}Why this matters:${NC}"
  echo "  .gitignore does not apply to files already in the git index."
  echo "  These paths often contain real credentials or private environment details."
  echo
  echo -e "${YELLOW}Fix (recommended):${NC}"
  cat <<'EOF'
  1) Untrack the files (keep local copies on disk):
     git rm --cached -- .env .env.test docs/splunk-test-environment.md rust_out

  2) Commit the removals:
     git commit -m "chore(security): stop tracking local secret files"

  3) Verify:
     make lint-secrets
EOF
  echo
  exit 1
fi

echo -e "${GREEN}✓ Secret-commit guard OK:${NC} no forbidden paths are tracked."
