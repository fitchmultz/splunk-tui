#!/usr/bin/env bash
# Investigate fired alerts from Splunk and retrieve detailed results
#
# RESPONSIBILITY:
#   Lists recently fired alerts from the configured time window, displays details
#   for high-severity alerts, and retrieves search results for deeper investigation.
#   Helps security analysts quickly triage and investigate triggered alerts.
#
# DOES NOT:
#   - Acknowledge or modify alert status in Splunk
#   - Suppress or disable alert rules
#   - Send notifications or create tickets
#   - Perform automated response actions
#
# PREREQUISITES:
#   - splunk-cli installed and in PATH
#   - SPLUNK_BASE_URL configured
#   - Authentication configured (SPLUNK_API_TOKEN or SPLUNK_USERNAME/PASSWORD)
#
# USAGE:
#   ./alert-investigation.sh [options]

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

# Script configuration
SCRIPT_NAME="$(basename "$0")"
DEFAULT_HOURS=4
MAX_HOURS=72
DEFAULT_LIMIT=50

show_help() {
  cat << EOF
Alert Investigation Tool

Usage: ${SCRIPT_NAME} [OPTIONS]

Options:
  -h, --hours N     Time window in hours (default: ${DEFAULT_HOURS}, max: ${MAX_HOURS})
  -l, --limit N     Maximum alerts to display (default: ${DEFAULT_LIMIT})
  -s, --severity    Filter by severity: critical, high, medium, low
  -d, --details     Show full details including search results
  --help            Display this help message

Examples:
  ${SCRIPT_NAME}                           # List alerts from last 4 hours
  ${SCRIPT_NAME} --hours 8                 # List alerts from last 8 hours
  ${SCRIPT_NAME} --severity critical       # Show only critical alerts
  ${SCRIPT_NAME} --details                 # Show full search results

Environment:
  SPLUNK_BASE_URL     Splunk REST API URL
  SPLUNK_API_TOKEN    API token for authentication
  SPLUNK_USERNAME     Username for authentication (if not using token)
  SPLUNK_PASSWORD     Password for authentication (if not using token)
  NO_COLOR            Disable colored output

EOF
}

check_prerequisites() {
  if ! command -v splunk-cli &> /dev/null; then
    echo -e "${RED}Error: splunk-cli is not installed or not in PATH${NC}" >&2
    echo "Please install splunk-cli and ensure it's available" >&2
    exit 1
  fi

  if ! command -v jq &> /dev/null; then
    echo -e "${YELLOW}Warning: jq is not installed. Some features may be limited${NC}" >&2
  fi

  if [[ -z "${SPLUNK_BASE_URL:-}" ]]; then
    echo -e "${RED}Error: SPLUNK_BASE_URL environment variable is not set${NC}" >&2
    echo "Example: export SPLUNK_BASE_URL=https://splunk.example.com:8089" >&2
    exit 1
  fi

  if [[ -z "${SPLUNK_API_TOKEN:-}" && ( -z "${SPLUNK_USERNAME:-}" || -z "${SPLUNK_PASSWORD:-}" ) ]]; then
    echo -e "${RED}Error: Authentication not configured${NC}" >&2
    echo "Set either SPLUNK_API_TOKEN or both SPLUNK_USERNAME and SPLUNK_PASSWORD" >&2
    exit 1
  fi
}

get_severity_color() {
  local severity="$1"
  case "${severity,,}" in
    critical)
      echo "$RED"
      ;;
    high)
      echo "$YELLOW"
      ;;
    medium)
      echo "$BLUE"
      ;;
    *)
      echo "$NC"
      ;;
  esac
}

format_timestamp() {
  local ts="$1"
  # Try to convert epoch or format existing timestamp
  if [[ "$ts" =~ ^[0-9]+$ ]]; then
    if command -v date &> /dev/null; then
      date -d "@$ts" '+%Y-%m-%d %H:%M:%S' 2>/dev/null || echo "$ts"
    else
      echo "$ts"
    fi
  else
    echo "$ts"
  fi
}

list_alerts() {
  local hours="$1"
  local limit="$2"
  local severity_filter="${3:-}"

  echo -e "${BLUE}Fetching alerts from last ${hours} hours...${NC}" >&2

  local output
  if ! output=$(splunk-cli alerts list --limit "$limit" 2>/dev/null); then
    echo -e "${RED}Error: Failed to fetch alerts${NC}" >&2
    return 1
  fi

  if [[ -z "$output" || "$output" == "[]" ]]; then
    echo -e "${YELLOW}No alerts found in the specified time window${NC}"
    return 0
  fi

  # Parse and display alerts (assuming JSON output from splunk-cli)
  if command -v jq &> /dev/null; then
    # Use jq for proper JSON parsing if available
    local jq_filter='.[] | select(.fired_time | capture("(?<ts>[0-9]+)").ts | tonumber > (now - '\"$hours\"' * 3600))'
    
    if [[ -n "$severity_filter" ]]; then
      jq_filter="${jq_filter} | select(.severity | ascii_downcase == \"${severity_filter}\")"
    fi

    echo "$output" | jq -r "$jq_filter | [.fired_time, .severity, .alert_name, .sid, .result_count] | @tsv" 2>/dev/null | while IFS=$'\t' read -r fired_time severity alert_name sid result_count; do
      local sev_color
      sev_color=$(get_severity_color "$severity")
      local formatted_time
      formatted_time=$(format_timestamp "$fired_time")
      printf "${sev_color}[%-8s]${NC} %s | %s (SID: %s, Results: %s)\n" "$severity" "$formatted_time" "$alert_name" "$sid" "$result_count"
    done
  else
    # Fallback to raw output
    echo "$output"
  fi
}

show_alert_details() {
  local sid="$1"

  echo -e "${BLUE}Retrieving details for alert SID: ${sid}${NC}"

  # Get job results
  local results
  if ! results=$(splunk-cli jobs --results "$sid" --format json 2>/dev/null); then
    echo -e "${YELLOW}Warning: Could not retrieve results for SID ${sid}${NC}" >&2
    return 1
  fi

  if [[ -z "$results" || "$results" == "[]" ]]; then
    echo -e "${YELLOW}No results found for this alert${NC}"
    return 0
  fi

  # Display first 10 results
  echo -e "${GREEN}Alert Results (first 10 events):${NC}"
  if command -v jq &> /dev/null; then
    echo "$results" | jq '.[0:10]'
  else
    echo "$results" | head -20
  fi
}

investigate_high_severity() {
  local hours="$1"
  local limit="$2"

  echo -e "${BLUE}Investigating high and critical severity alerts...${NC}" >&2

  local output
  if ! output=$(splunk-cli alerts list --limit "$limit" 2>/dev/null); then
    echo -e "${RED}Error: Failed to fetch alerts${NC}" >&2
    return 1
  fi

  if [[ -z "$output" || "$output" == "[]" ]]; then
    return 0
  fi

  if command -v jq &> /dev/null; then
    # Find high/critical severity alerts
    local high_alerts
    high_alerts=$(echo "$output" | jq -r '[.[] | select(.severity | ascii_downcase | test("critical|high"))]')

    local count
    count=$(echo "$high_alerts" | jq 'length')

    if [[ "$count" -eq 0 ]]; then
      echo -e "${GREEN}No high or critical severity alerts found${NC}"
      return 0
    fi

    echo -e "${YELLOW}Found ${count} high/critical severity alert(s)${NC}"

    # Show details for each
    echo "$high_alerts" | jq -r '.[] | [.sid, .alert_name, .severity] | @tsv' | while IFS=$'\t' read -r sid alert_name severity; do
      echo ""
      echo -e "${RED}=== CRITICAL ALERT: ${alert_name} ===${NC}"
      show_alert_details "$sid"
    done
  fi
}

main() {
  local hours="$DEFAULT_HOURS"
  local limit="$DEFAULT_LIMIT"
  local severity=""
  local show_details=false

  # Parse arguments
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --help)
        show_help
        exit 0
        ;;
      -h|--hours)
        if [[ -z "${2:-}" ]]; then
          echo -e "${RED}Error: --hours requires a value${NC}" >&2
          exit 1
        fi
        hours="$2"
        shift 2
        ;;
      -l|--limit)
        if [[ -z "${2:-}" ]]; then
          echo -e "${RED}Error: --limit requires a value${NC}" >&2
          exit 1
        fi
        limit="$2"
        shift 2
        ;;
      -s|--severity)
        if [[ -z "${2:-}" ]]; then
          echo -e "${RED}Error: --severity requires a value${NC}" >&2
          exit 1
        fi
        severity="$2"
        if [[ "$severity" != "critical" && "$severity" != "high" && "$severity" != "medium" && "$severity" != "low" ]]; then
          echo -e "${RED}Error: Severity must be critical, high, medium, or low${NC}" >&2
          exit 1
        fi
        shift 2
        ;;
      -d|--details)
        show_details=true
        shift
        ;;
      -*)
        echo -e "${RED}Error: Unknown option: $1${NC}" >&2
        show_help
        exit 1
        ;;
      *)
        echo -e "${RED}Error: Unknown argument: $1${NC}" >&2
        show_help
        exit 1
        ;;
    esac
  done

  # Validate hours
  if ! [[ "$hours" =~ ^[0-9]+$ ]]; then
    echo -e "${RED}Error: Hours must be a positive integer${NC}" >&2
    exit 1
  fi

  if [[ "$hours" -gt "$MAX_HOURS" ]]; then
    echo -e "${YELLOW}Warning: Hours capped at ${MAX_HOURS}${NC}" >&2
    hours="$MAX_HOURS"
  fi

  # Validate limit
  if ! [[ "$limit" =~ ^[0-9]+$ ]] || [[ "$limit" -lt 1 ]]; then
    echo -e "${RED}Error: Limit must be a positive integer${NC}" >&2
    exit 1
  fi

  check_prerequisites

  echo -e "${GREEN}Alert Investigation Tool${NC}"
  echo -e "${BLUE}========================${NC}"
  echo ""

  # List all alerts
  if ! list_alerts "$hours" "$limit" "$severity"; then
    exit 1
  fi

  echo ""

  # If details requested or high severity investigation
  if [[ "$show_details" == true ]]; then
    investigate_high_severity "$hours" "$limit"
  fi

  echo -e "${GREEN}Investigation complete.${NC}"
  exit 0
}

main "$@"
