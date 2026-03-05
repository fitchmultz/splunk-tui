#!/usr/bin/env bash
# Track login activity and authentication anomalies
#
# RESPONSIBILITY:
#   Analyzes authentication patterns from Splunk's _audit index to identify
#   login success/failure trends, failed login attempts by source, and unusual
#   login patterns including off-hours access and new/unusual source IPs.
#
# DOES NOT:
#   - Block or modify any authentication settings
#   - Alert external systems (only reports to stdout)
#   - Track non-authentication audit events
#   - Store historical data between runs
#
# PREREQUISITES:
#   - splunk-cli installed and in PATH
#   - SPLUNK_BASE_URL configured
#   - Authentication configured (SPLUNK_API_TOKEN or SPLUNK_USERNAME/PASSWORD)
#
# USAGE:
#   ./login-tracking.sh [options]

set -euo pipefail

# Color support (respect NO_COLOR)
if [[ -t 1 && -z "${NO_COLOR:-}" ]]; then
  RED='\033[0;31m'
  GREEN='\033[0;32m'
  YELLOW='\033[1;33m'
  BLUE='\033[0;34m'
  NC='\033[0m'
else
  RED='' GREEN='' YELLOW='' BLUE='' NC=''
fi

# Configuration
DEFAULT_TIME_RANGE="-24h"
OFF_HOURS_START=22
OFF_HOURS_END=6

# Output formatting
info() { echo -e "${BLUE}[INFO]${NC} $1"; }
success() { echo -e "${GREEN}[OK]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; }

show_help() {
  cat << 'EOF'
Track login activity and authentication anomalies

USAGE:
  ./login-tracking.sh [OPTIONS]

OPTIONS:
  -t, --time <range>     Time range for analysis (default: -24h)
  -o, --output <file>    Save report to file
  --off-hours-start <h>  Start hour for off-hours (default: 22)
  --off-hours-end <h>    End hour for off-hours (default: 6)
  --no-color             Disable colored output
  -h, --help             Show this help message

EXAMPLES:
  ./login-tracking.sh
  ./login-tracking.sh -t "-7d" --output weekly-logins.txt
  ./login-tracking.sh --off-hours-start 20 --off-hours-end 5

EXIT CODES:
  0  Success
  1  Prerequisites not met or command failed
  2  No data found for the specified time range
EOF
}

check_prerequisites() {
  local missing=()

  if ! command -v splunk-cli &> /dev/null; then
    missing+=("splunk-cli")
  fi

  if ! command -v jq &> /dev/null; then
    missing+=("jq")
  fi

  if [[ ${#missing[@]} -gt 0 ]]; then
    error "Failed to find required tools: ${missing[*]}"
    echo "Please install the missing prerequisites."
    exit 1
  fi

  if [[ -z "${SPLUNK_BASE_URL:-}" ]]; then
    error "SPLUNK_BASE_URL is not configured"
    echo "Set SPLUNK_BASE_URL environment variable or use --base-url"
    exit 1
  fi

  if [[ -z "${SPLUNK_API_TOKEN:-}" && ( -z "${SPLUNK_USERNAME:-}" || -z "${SPLUNK_PASSWORD:-}" ) ]]; then
    error "Authentication not configured"
    echo "Set SPLUNK_API_TOKEN or both SPLUNK_USERNAME and SPLUNK_PASSWORD"
    exit 1
  fi

  success "Prerequisites verified"
}

run_search() {
  local query="$1"
  local time_range="${2:-$DEFAULT_TIME_RANGE}"
  local output_format="${3:-json}"

  splunk-cli search execute "$query" --wait \
    --earliest "$time_range" \
    --latest "now" \
    --output "$output_format" \
    --quiet 2>/dev/null
}

analyze_login_success_failure() {
  local time_range="$1"

  info "Analyzing login success/failure counts..."

  local query='search index=_audit action=login user=*
    | eval status=if(action_result="success" OR action_result="succeeded", "success", "failure")
    | stats count by status
    | eval percent=round((count/sum(count))*100, 2)'

  local results
  if ! results=$(run_search "$query" "$time_range"); then
    error "Failed to fetch login statistics"
    return 1
  fi

  if [[ -z "$results" || "$results" == "[]" || "$results" == "null" ]]; then
    warn "No login data found for time range: $time_range"
    return 2
  fi

  echo ""
  echo "=== Login Success/Failure Summary ==="
  echo "$results" | jq -r '.[] | "  \(.status): \(.count) (\(.percent)%)"' 2>/dev/null || echo "$results"
  echo ""
}

analyze_failed_logins_by_source() {
  local time_range="$1"

  info "Analyzing failed login attempts by source..."

  local query='search index=_audit action=login (action_result="failure" OR action_result="failed" OR action_result="*fail*") user=*
    | stats count by src, user
    | sort - count
    | head 20'

  local results
  if ! results=$(run_search "$query" "$time_range"); then
    error "Failed to fetch failed login by source"
    return 1
  fi

  if [[ -z "$results" || "$results" == "[]" || "$results" == "null" ]]; then
    warn "No failed login attempts found for time range: $time_range"
    return 0
  fi

  echo "=== Top Failed Login Sources ==="
  echo "$results" | jq -r '.[] | "  \(.src) as \(.user): \(.count) attempts"' 2>/dev/null || echo "$results"

  # Check for brute force indicators
  local high_volume
  high_volume=$(echo "$results" | jq '[.[] | select(.count >= 10)] | length' 2>/dev/null || echo "0")

  if [[ "$high_volume" -gt 0 ]]; then
    echo ""
    warn "Potential brute force attacks detected (≥10 attempts):"
    echo "$results" | jq -r '.[] | select(.count >= 10) | "    - \(.src): \(.count) attempts"' 2>/dev/null
  fi
  echo ""
}

analyze_off_hours_logins() {
  local time_range="$1"
  local start_hour="$2"
  local end_hour="$3"

  info "Analyzing off-hours login activity (${start_hour}:00 - ${end_hour}:00)..."

  local query="search index=_audit action=login action_result=success user=*
    | eval hour=tonumber(strftime(_time, \"%H\"))
    | eval is_off_hours=if(hour>=${start_hour} OR hour<${end_hour}, \"yes\", \"no\")
    | where is_off_hours=\"yes\"
    | stats count by user, src
    | sort - count"

  local results
  if ! results=$(run_search "$query" "$time_range"); then
    error "Failed to fetch off-hours login data"
    return 1
  fi

  if [[ -z "$results" || "$results" == "[]" || "$results" == "null" ]]; then
    echo "No off-hours login activity detected"
    echo ""
    return 0
  fi

  echo "=== Off-Hours Login Activity ==="
  echo "$results" | jq -r '.[] | "  \(.user) from \(.src): \(.count) logins"' 2>/dev/null || echo "$results"

  local off_hours_count
  off_hours_count=$(echo "$results" | jq 'length' 2>/dev/null || echo "0")
  if [[ "$off_hours_count" -gt 0 ]]; then
    echo ""
    warn "Found $off_hours_count user(s) with off-hours login activity"
  fi
  echo ""
}

analyze_new_sources() {
  local time_range="$1"

  info "Analyzing login sources (new or unusual)..."

  local query='search index=_audit action=login action_result=success user=*
    | stats dc(user) as unique_users, count as total_logins by src
    | eval login_per_user=round(total_logins/unique_users, 2)
    | sort - total_logins'

  local results
  if ! results=$(run_search "$query" "$time_range"); then
    error "Failed to fetch login source data"
    return 1
  fi

  if [[ -z "$results" || "$results" == "[]" || "$results" == "null" ]]; then
    warn "No login source data found"
    return 0
  fi

  echo "=== Login Sources Summary ==="
  echo "$results" | jq -r '.[] | "  \(.src): \(.total_logins) logins, \(.unique_users) unique users"' 2>/dev/null || echo "$results"

  # Check for single-login sources (potential anomalies)
  local single_login
  single_login=$(echo "$results" | jq '[.[] | select(.total_logins == 1)] | length' 2>/dev/null || echo "0")

  if [[ "$single_login" -gt 0 ]]; then
    echo ""
    warn "Found $single_login source(s) with only 1 login (potential anomaly)"
  fi
  echo ""
}

main() {
  local time_range="$DEFAULT_TIME_RANGE"
  local output_file=""

  # Parse arguments
  while [[ $# -gt 0 ]]; do
    case "$1" in
      -t|--time)
        time_range="$2"
        shift 2
        ;;
      -o|--output)
        output_file="$2"
        shift 2
        ;;
      --off-hours-start)
        OFF_HOURS_START="$2"
        shift 2
        ;;
      --off-hours-end)
        OFF_HOURS_END="$2"
        shift 2
        ;;
      --no-color)
        RED='' GREEN='' YELLOW='' BLUE='' NC=''
        shift
        ;;
      -h|--help)
        show_help
        exit 0
        ;;
      *)
        error "Unknown option: $1"
        show_help
        exit 1
        ;;
    esac
  done

  # Header
  echo ""
  echo "========================================"
  echo "    Login Activity Security Audit"
  echo "========================================"
  echo "Time Range: $time_range"
  echo "Splunk: $SPLUNK_BASE_URL"
  echo "Generated: $(date)"
  echo "========================================"
  echo ""

  check_prerequisites

  # Capture output if file specified
  if [[ -n "$output_file" ]]; then
    exec > >(tee "$output_file")
  fi

  # Run analyses
  local exit_code=0

  analyze_login_success_failure "$time_range" || exit_code=$?
  analyze_failed_logins_by_source "$time_range" || exit_code=$?
  analyze_off_hours_logins "$time_range" "$OFF_HOURS_START" "$OFF_HOURS_END" || exit_code=$?
  analyze_new_sources "$time_range" || exit_code=$?

  # Summary
  echo "========================================"
  echo "    Audit Complete"
  echo "========================================"

  if [[ -n "$output_file" ]]; then
    success "Report saved to: $output_file"
  fi

  exit $exit_code
}

main "$@"
