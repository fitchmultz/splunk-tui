#!/usr/bin/env bash
# Purpose: Validate live-test prerequisites and exit with mode-aware status codes.
# Responsibilities: Load `.env.test` safely, validate required env vars, and check Splunk reachability.
# Non-scope: Does not execute tests or mutate project files.
# Invariants: Never executes `.env.test` as shell code and uses documented exit codes.

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

trim_whitespace() {
    local value="$1"
    value="${value#"${value%%[![:space:]]*}"}"
    value="${value%"${value##*[![:space:]]}"}"
    printf '%s' "$value"
}

strip_surrounding_quotes() {
    local value="$1"
    if [[ ${#value} -ge 2 ]]; then
        local first="${value:0:1}"
        local last="${value: -1}"
        if [[ "$first" == "$last" && ( "$first" == '"' || "$first" == "'" ) ]]; then
            printf '%s' "${value:1:${#value}-2}"
            return
        fi
    fi
    printf '%s' "$value"
}

load_env_file() {
    local path="$1"
    local line_number=0
    local line key raw_value value

    while IFS= read -r line || [[ -n "$line" ]]; do
        line_number=$((line_number + 1))
        line="${line%$'\r'}"

        if [[ -z "$(trim_whitespace "$line")" ]]; then
            continue
        fi

        if [[ "$line" =~ ^[[:space:]]*# ]]; then
            continue
        fi

        if [[ "$line" =~ ^[[:space:]]*([A-Za-z_][A-Za-z0-9_]*)[[:space:]]*=(.*)$ ]]; then
            key="${BASH_REMATCH[1]}"
            raw_value="$(trim_whitespace "${BASH_REMATCH[2]}")"
            value="$(strip_surrounding_quotes "$raw_value")"
            export "$key=$value"
        else
            echo -e "${YELLOW}Ignoring invalid .env.test entry at line ${line_number}${NC}" >&2
        fi
    done < "$path"
}

# Load .env.test as key-value data (never execute as shell code)
if [[ -f "$ENV_TEST_PATH" ]]; then
    echo "Loading test environment from .env.test"
    load_env_file "$ENV_TEST_PATH"
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
            echo "To continue locally without a live Splunk server, use LIVE_TESTS_MODE=optional" >&2
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
