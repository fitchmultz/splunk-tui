#!/usr/bin/env bash
# Validate live test environment for CI enforcement.
# Exits 0 if tests can run, 1 if they should fail (required mode), 2 if skipped (optional mode).
#
# Usage: validate-live-test-env.sh [required|optional|skip]
#
# Exit codes:
#   0 - Environment valid, proceed with tests
#   1 - Environment invalid in required mode (caller should exit 1)
#   2 - Skipped in optional/skip mode (caller should exit 0)
#   3 - Invalid arguments
#
# Examples:
#   # CI mode - fail if env/server not configured
#   ./scripts/validate-live-test-env.sh required
#
#   # Local dev mode - skip gracefully if unavailable
#   ./scripts/validate-live-test-env.sh optional
#
#   # Explicit skip
#   ./scripts/validate-live-test-env.sh skip
#
# Required environment (set in .env.test):
#   SPLUNK_BASE_URL - Splunk REST API URL (e.g., https://localhost:8089)
#   SPLUNK_USERNAME - Splunk username (or SPLUNK_API_TOKEN)
#   SPLUNK_PASSWORD - Splunk password (or SPLUNK_API_TOKEN)

set -euo pipefail

show_help() {
    cat <<'EOF'
validate-live-test-env.sh - Validate live test environment for CI enforcement

USAGE:
    validate-live-test-env.sh [MODE]

MODES:
    required    Fail if .env.test missing or Splunk server unreachable (CI default)
    optional    Skip with warning if unavailable (local dev recommended)
    skip        Explicit bypass

EXIT CODES:
    0    Environment valid, proceed with tests
    1    Environment invalid in required mode
    2    Skipped in optional/skip mode
    3    Invalid arguments

EXAMPLES:
    # CI mode - fail if env/server not configured
    LIVE_TESTS_MODE=required make test-live

    # Local dev mode - skip gracefully if unavailable
    LIVE_TESTS_MODE=optional make test-live

    # Explicit skip
    LIVE_TESTS_MODE=skip make test-live

    # Legacy (still supported)
    SKIP_LIVE_TESTS=1 make test-live

ENVIRONMENT:
    SPLUNK_BASE_URL   Splunk REST API URL (required)
    SPLUNK_USERNAME   Splunk username (required if no token)
    SPLUNK_PASSWORD   Splunk password (required if no token)
    SPLUNK_API_TOKEN  API token (alternative to username/password)
    SPLUNK_SKIP_VERIFY Set to 'true' to skip TLS verification
EOF
}

if [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
    show_help
    exit 0
fi

# Colors (respects NO_COLOR)
if [[ -t 1 && -z "${NO_COLOR:-}" ]]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    NC='\033[0m'
else
    RED='' GREEN='' YELLOW='' NC=''
fi

MODE="${1:-required}"

# Handle legacy SKIP_LIVE_TESTS for backwards compatibility
if [[ "${SKIP_LIVE_TESTS:-0}" == "1" ]]; then
    case "$MODE" in
        required)
            echo -e "${RED}ERROR: SKIP_LIVE_TESTS=1 is not allowed in required mode${NC}"
            echo "Use LIVE_TESTS_MODE=optional or LIVE_TESTS_MODE=skip instead"
            exit 1
            ;;
        optional|skip)
            echo -e "${YELLOW}Skipping live tests (SKIP_LIVE_TESTS=1)${NC}"
            exit 2
            ;;
    esac
fi

case "$MODE" in
    skip)
        echo -e "${YELLOW}Skipping live tests (LIVE_TESTS_MODE=skip)${NC}"
        exit 2
        ;;
    optional|required)
        ;;
    *)
        echo "Usage: $0 [required|optional|skip]" >&2
        exit 3
        ;;
esac

# Path to .env.test (workspace root relative to script location)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ENV_TEST_PATH="${SCRIPT_DIR}/../.env.test"

# Load .env.test if it exists
if [[ -f "$ENV_TEST_PATH" ]]; then
    echo "Loading test environment from .env.test"
    # Source safely, handling potential errors
    set -a
    # shellcheck disable=SC1090
    source "$ENV_TEST_PATH" 2>/dev/null || true
    set +a
fi

# Check for required environment variables
check_env_vars() {
    local missing=()
    
    if [[ -z "${SPLUNK_BASE_URL:-}" ]]; then
        missing+=("SPLUNK_BASE_URL")
    fi
    
    # Need either API token or username+password
    if [[ -z "${SPLUNK_API_TOKEN:-}" ]]; then
        if [[ -z "${SPLUNK_USERNAME:-}" ]]; then
            missing+=("SPLUNK_USERNAME")
        fi
        if [[ -z "${SPLUNK_PASSWORD:-}" ]]; then
            missing+=("SPLUNK_PASSWORD")
        fi
    fi
    
    if [[ ${#missing[@]} -gt 0 ]]; then
        echo -e "${YELLOW}Missing environment variables: ${missing[*]}${NC}" >&2
        return 1
    fi
    return 0
}

# Check if Splunk server is reachable
check_server_reachable() {
    local base_url="${SPLUNK_BASE_URL:-}"
    
    if [[ -z "$base_url" ]]; then
        return 1
    fi
    
    # Extract host:port from URL
    local host_port
    host_port=$(echo "$base_url" | sed -E 's|^https?://||' | cut -d'/' -f1)
    
    # Use curl with short timeout to check reachability
    if command -v curl &>/dev/null; then
        if curl --connect-timeout 3 --max-time 5 -s -o /dev/null \
            ${SPLUNK_SKIP_VERIFY:+-k} "$base_url/services/server/info" 2>/dev/null; then
            return 0
        fi
    elif command -v nc &>/dev/null; then
        local host port
        host=$(echo "$host_port" | cut -d: -f1)
        port=$(echo "$host_port" | cut -d: -f2)
        port="${port:-8089}"
        if nc -z -w 3 "$host" "$port" 2>/dev/null; then
            return 0
        fi
    fi
    
    echo -e "${YELLOW}Splunk server unreachable: $base_url${NC}" >&2
    return 1
}

# Main validation logic
if ! check_env_vars; then
    case "$MODE" in
        required)
            echo -e "${RED}ERROR: Live tests are REQUIRED but environment is not configured${NC}" >&2
            echo "Set SPLUNK_BASE_URL, SPLUNK_USERNAME, SPLUNK_PASSWORD (or SPLUNK_API_TOKEN)" >&2
            echo "Or create .env.test from .env.test.example" >&2
            echo "To skip in CI, set LIVE_TESTS_MODE=optional (not recommended)" >&2
            exit 1
            ;;
        optional)
            echo -e "${YELLOW}Skipping live tests (optional mode): environment not configured${NC}"
            exit 2
            ;;
    esac
fi

if ! check_server_reachable; then
    case "$MODE" in
        required)
            echo -e "${RED}ERROR: Live tests are REQUIRED but Splunk server is unreachable${NC}" >&2
            echo "Server: $SPLUNK_BASE_URL" >&2
            echo "Check that Splunk is running and accessible" >&2
            exit 1
            ;;
        optional)
            echo -e "${YELLOW}Skipping live tests (optional mode): server unreachable${NC}"
            exit 2
            ;;
    esac
fi

echo -e "${GREEN}Live test environment validated${NC}"
echo "Server: $SPLUNK_BASE_URL"
exit 0
