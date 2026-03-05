#!/usr/bin/env bash
# Track configuration changes for compliance auditing
#
# RESPONSIBILITY:
#   Monitors recent configuration changes in Splunk, identifying changes by
#   user, tracking critical setting modifications, and providing a timeline
#   of administrative actions for compliance purposes.
#
# DOES NOT:
#   - Revert or block any configuration changes
#   - Modify audit settings or policies
#   - Track non-configuration audit events
#   - Store data between runs
#
# PREREQUISITES:
#   - splunk-cli installed and in PATH
#   - SPLUNK_BASE_URL configured
#   - Authentication configured (SPLUNK_API_TOKEN or SPLUNK_USERNAME/PASSWORD)
#
# USAGE:
#   ./config-changes.sh [options]

set -euo pipefail

# Color support (respect NO_COLOR)
if [[ -t 1 && -z "${NO_COLOR:-}" ]]; then
  RED='\033[0;31m'
  GREEN='\033[0;32m'
  YELLOW='\033[1;33m'
  BLUE='\033[0;34m'
  CYAN='\033[0;36m'
  NC='\033[0m'
else
  RED='' GREEN='' YELLOW='' BLUE='' CYAN='' NC=''
fi

# Configuration
DEFAULT_TIME_RANGE="-24h"
CRITICAL_OPERATIONS=("create" "delete" "edit" "update" "remove" "enable" "disable")

# Output formatting
info() { echo -e "${BLUE}[INFO]${NC} $1"; }
success() { echo -e "${GREEN}[OK]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; }
highlight() { echo -e "${CYAN}$1${NC}"; }

show_help() {
  cat << 'EOF'
Track configuration changes for compliance auditing

USAGE:
  ./config-changes.sh [OPTIONS]

OPTIONS:
  -t, --time <range>       Time range for analysis (default: -24h)
  -u, --user <username>    Filter changes by specific user
  -o, --output <file>      Save report to file
  --critical-only          Show only critical/high-impact changes
  --by-user                Group changes by user
  --by-object              Group changes by object type
  --no-color               Disable colored output
  -h, --help               Show this help message

EXAMPLES:
  ./config-changes.sh
  ./config-changes.sh -t "-7d" --by-user
  ./config-changes.sh --user admin --critical-only
  ./config-changes.sh -t "-1h" --output emergency-audit.txt

EXIT CODES:
  0  Success
  1  Prerequisites not met or command failed
  2  No configuration changes found
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
    echo "Set SPLUNK_BASE_URL environment variable"
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

fetch_audit_events() {
  local time_range="$1"
  local filter_user="${2:-}"

  info "Fetching configuration audit events..."

  local params=()
  params+=("--earliest" "$time_range")
  params+=("--latest" "now")

  if [[ -n "$filter_user" ]]; then
    params+=("--user" "$filter_user")
  fi

  local events
  if ! events=$(splunk-cli audit list "${params[@]}" --output json --quiet 2>/dev/null); then
    error "Failed to fetch audit events"
    return 1
  fi

  if [[ -z "$events" || "$events" == "[]" || "$events" == "null" ]]; then
    warn "No audit events found for time range: $time_range"
    return 2
  fi

  echo "$events"
}

analyze_changes_timeline() {
  local events="$1"

  echo ""
  echo "=== Configuration Changes Timeline ==="
  echo ""

  echo "$events" | jq -r '[.[] | {timestamp: (.timestamp // "unknown"), user: (.user // "unknown"), action: (.action // "unknown"), object: (.object // "unknown")}] | sort_by(.timestamp) | .[] | "\(.timestamp) | \(.user) | \(.action) | \(.object)"' 2>/dev/null | while IFS='|' read -r timestamp user action object; do
    # Trim whitespace
    timestamp=$(echo "$timestamp" | xargs)
    user=$(echo "$user" | xargs)
    action=$(echo "$action" | xargs)
    object=$(echo "$object" | xargs)

    # Color-code actions
    local action_color="$NC"
    local action_lower
    action_lower=$(echo "$action" | tr '[:upper:]' '[:lower:]')
    case "$action_lower" in
      create|add)
        action_color="$GREEN"
        ;;
      delete|remove)
        action_color="$RED"
        ;;
      edit|update|modify)
        action_color="$YELLOW"
        ;;
      *)
        action_color="$BLUE"
        ;;
    esac

    printf "  %s | %s | ${action_color}%-10s${NC} | %s\n" "$timestamp" "$user" "$action" "$object"
  done

  local count
  count=$(echo "$events" | jq 'length' 2>/dev/null || echo "0")
  echo ""
  success "Total events: $count"
  echo ""
}

analyze_changes_by_user() {
  local events="$1"

  echo ""
  echo "=== Changes by User ==="
  echo ""

  local by_user
  by_user=$(echo "$events" | jq '
    group_by(.user) |
    map({
      user: (.[0].user // "unknown"),
      count: length,
      actions: (group_by(.action) | map({action: .[0].action, count: length}) | sort_by(-.count))
    }) |
    sort_by(-.count)
  ')

  echo "$by_user" | jq -r '.[] | "User: \(.user)\n  Total Changes: \(.count)\n  Actions:\n    - " + (.actions | map("\(.action): \(.count)") | join("\n    - ")) + "\n"'
}

analyze_changes_by_object() {
  local events="$1"

  echo ""
  echo "=== Changes by Object Type ==="
  echo ""

  # Group by object type
  local by_object
  by_object=$(echo "$events" | jq '
    group_by(.object) |
    map({
      object: (.[0].object // "unknown"),
      count: length,
      users: (map(.user) | unique),
      recent_action: .[-1].action
    }) |
    sort_by(-.count)
  ')

  echo "$by_object" | jq -r '.[] | "Object: \(.object)\n  Changes: \(.count)\n  Users: \(.users | join(", "))\n  Most Recent Action: \(.recent_action)\n"'
}

identify_critical_changes() {
  local events="$1"

  echo ""
  echo "=== Critical/High-Impact Changes ==="
  echo ""

  local critical_found=false

  # Check for delete operations
  local deletes
  deletes=$(echo "$events" | jq '[.[] | select(.action | ascii_downcase | test("delete|remove"; "i"))]')
  local delete_count
  delete_count=$(echo "$deletes" | jq 'length' 2>/dev/null || echo "0")

  if [[ "$delete_count" -gt 0 ]]; then
    critical_found=true
    warn "DELETIONS ($delete_count events):"
    echo "$deletes" | jq -r '.[] | "  - \(.timestamp): \(.user) deleted \(.object)"' 2>/dev/null
    echo ""
  fi

  # Check for user/role changes
  local user_changes
  user_changes=$(echo "$events" | jq '[.[] | select(.object | ascii_downcase | test("user|role|capability|permission"; "i"))]')
  local user_change_count
  user_change_count=$(echo "$user_changes" | jq 'length' 2>/dev/null || echo "0")

  if [[ "$user_change_count" -gt 0 ]]; then
    critical_found=true
    warn "USER/ROLE CHANGES ($user_change_count events):"
    echo "$user_changes" | jq -r '.[] | "  - \(.timestamp): \(.user) performed \(.action) on \(.object)"' 2>/dev/null
    echo ""
  fi

  # Check for authentication changes
  local auth_changes
  auth_changes=$(echo "$events" | jq '[.[] | select(.object | ascii_downcase | test("auth|password|certificate|ssl|ldap|sso"; "i"))]')
  local auth_change_count
  auth_change_count=$(echo "$auth_changes" | jq 'length' 2>/dev/null || echo "0")

  if [[ "$auth_change_count" -gt 0 ]]; then
    critical_found=true
    warn "AUTHENTICATION CHANGES ($auth_change_count events):"
    echo "$auth_changes" | jq -r '.[] | "  - \(.timestamp): \(.user) performed \(.action) on \(.object)"' 2>/dev/null
    echo ""
  fi

  # Check for server/system configuration changes
  local system_changes
  system_changes=$(echo "$events" | jq '[.[] | select(.object | ascii_downcase | test("server|config|setting|conf"; "i"))]')
  local system_change_count
  system_change_count=$(echo "$system_changes" | jq 'length' 2>/dev/null || echo "0")

  if [[ "$system_change_count" -gt 0 ]]; then
    critical_found=true
    warn "SYSTEM CONFIGURATION CHANGES ($system_change_count events):"
    echo "$system_changes" | jq -r '.[] | "  - \(.timestamp): \(.user) performed \(.action) on \(.object)"' 2>/dev/null
    echo ""
  fi

  # Check for changes during off-hours (10 PM - 6 AM)
  local off_hours_changes
  off_hours_changes=$(echo "$events" | jq '
    [.[] | select(.timestamp | split("T")[1][:2] | tonumber >= 22 or tonumber < 6)]
  ')
  local off_hours_count
  off_hours_count=$(echo "$off_hours_changes" | jq 'length' 2>/dev/null || echo "0")

  if [[ "$off_hours_count" -gt 0 ]]; then
    warn "OFF-HOURS CHANGES ($off_hours_count events between 22:00-06:00):"
    echo "$off_hours_changes" | jq -r '.[] | "  - \(.timestamp): \(.user) performed \(.action) on \(.object)"' 2>/dev/null | head -10
    if [[ "$off_hours_count" -gt 10 ]]; then
      echo "  ... and $((off_hours_count - 10)) more"
    fi
    echo ""
  fi

  if [[ "$critical_found" == false ]]; then
    success "No critical or high-impact changes detected"
  fi
  echo ""
}

analyze_config_file_changes() {
  local time_range="$1"

  echo ""
  echo "=== Configuration File Changes (from _internal) ==="
  echo ""

  local query='search index=_internal source=*splunkd.log component=ConfLogger action=* | stats count by action, conf_file, stanza | sort - count'

  local results
  if ! results=$(run_search "$query" "$time_range"); then
    warn "Failed to fetch configuration file changes"
    return 1
  fi

  if [[ -z "$results" || "$results" == "[]" || "$results" == "null" ]]; then
    echo "No configuration file changes found in _internal logs"
    return 0
  fi

  echo "$results" | jq -r '.[] | "  \(.action) on \(.conf_file) [\(.stanza)]: \(.count) times"' 2>/dev/null | head -20

  local count
  count=$(echo "$results" | jq 'length' 2>/dev/null || echo "0")
  echo ""
  success "Found $count configuration file change entries"
  echo ""
}

find_unauthorized_changes() {
  local events="$1"

  echo ""
  echo "=== Potential Unauthorized Changes ==="
  echo ""

  # Check for changes by non-admin users to sensitive objects
  local sensitive_objects
  sensitive_objects=$(echo "$events" | jq '
    [.[] | select(
      (.object | ascii_downcase | test("admin|system|license|cluster|deployment"; "i")) and
      (.user | ascii_downcase | test("admin|splunk-system-user") | not)
    )]
  ')

  local sensitive_count
  sensitive_count=$(echo "$sensitive_objects" | jq 'length' 2>/dev/null || echo "0")

  if [[ "$sensitive_count" -gt 0 ]]; then
    warn "Non-admin users modified sensitive objects ($sensitive_count events):"
    echo "$sensitive_objects" | jq -r '.[] | "  - \(.timestamp): \(.user) performed \(.action) on \(.object)"' 2>/dev/null
  else
    success "No unauthorized changes to sensitive objects detected"
  fi
  echo ""
}

main() {
  local time_range="$DEFAULT_TIME_RANGE"
  local output_file=""
  local filter_user=""
  local critical_only=false
  local by_user=false
  local by_object=false

  # Parse arguments
  while [[ $# -gt 0 ]]; do
    case "$1" in
      -t|--time)
        time_range="$2"
        shift 2
        ;;
      -u|--user)
        filter_user="$2"
        shift 2
        ;;
      -o|--output)
        output_file="$2"
        shift 2
        ;;
      --critical-only)
        critical_only=true
        shift
        ;;
      --by-user)
        by_user=true
        shift
        ;;
      --by-object)
        by_object=true
        shift
        ;;
      --no-color)
        RED='' GREEN='' YELLOW='' BLUE='' CYAN='' NC=''
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
  echo "    Configuration Changes Audit"
  echo "========================================"
  echo "Time Range: $time_range"
  if [[ -n "$filter_user" ]]; then
    echo "Filtered User: $filter_user"
  fi
  echo "Splunk: $SPLUNK_BASE_URL"
  echo "Generated: $(date)"
  echo "========================================"
  echo ""

  check_prerequisites

  # Capture output if file specified
  if [[ -n "$output_file" ]]; then
    exec > >(tee "$output_file")
  fi

  # Fetch audit events
  local events
  if ! events=$(fetch_audit_events "$time_range" "$filter_user"); then
    local rc=$?
    if [[ $rc -eq 2 ]]; then
      echo "No configuration changes to report."
      exit 0
    fi
    exit 1
  fi

  # Run analyses
  if [[ "$critical_only" == true ]]; then
    identify_critical_changes "$events"
  else
    analyze_changes_timeline "$events"

    if [[ "$by_user" == true ]]; then
      analyze_changes_by_user "$events"
    fi

    if [[ "$by_object" == true ]]; then
      analyze_changes_by_object "$events"
    fi

    identify_critical_changes "$events"
    find_unauthorized_changes "$events"
    analyze_config_file_changes "$time_range"
  fi

  # Summary
  echo "========================================"
  echo "    Configuration Audit Complete"
  echo "========================================"

  if [[ -n "$output_file" ]]; then
    success "Report saved to: $output_file"
  fi

  exit 0
}

main "$@"
