#!/usr/bin/env bash
# Generate disk usage reports for Splunk indexes
#
# RESPONSIBILITY:
#   Analyzes disk usage across all Splunk indexes, generating a comprehensive
#   report showing per-index sizes, total storage consumption, and identifying
#   indexes approaching their configured limits.
#
# DOES NOT:
#   - Modify index configurations or data retention policies
#   - Delete or archive any data
#   - Adjust index size limits
#   - Perform any write operations on indexes
#
# PREREQUISITES:
#   - splunk-cli installed and in PATH
#   - SPLUNK_BASE_URL configured
#   - Authentication configured (SPLUNK_API_TOKEN or SPLUNK_USERNAME/PASSWORD)
#
# USAGE:
#   ./disk-usage-report.sh [options]
#
# OPTIONS:
#   --threshold N   Warning threshold percentage (default: 80)
#   --top N         Show only top N indexes by size (default: all)
#   --help          Show this help message

set -euo pipefail

# Color support (respect NO_COLOR)
if [[ -t 1 && -z "${NO_COLOR:-}" ]]; then
  RED='\033[0;31m'
  GREEN='\033[0;32m'
  YELLOW='\033[1;33m'
  BLUE='\033[0;34m'
  CYAN='\033[0;36m'
  BOLD='\033[1m'
  NC='\033[0m'
else
  RED='' GREEN='' YELLOW='' BLUE='' CYAN='' BOLD='' NC=''
fi

# Script configuration
SCRIPT_NAME="$(basename "$0")"
THRESHOLD=80
TOP_COUNT=0

# Data storage
indexes_data=""
total_indexes=0
total_size_bytes=0
indexes_above_threshold=()

# =============================================================================
# Helper Functions
# =============================================================================

show_help() {
  cat <<EOF
Splunk Index Disk Usage Report

Usage: ${SCRIPT_NAME} [OPTIONS]

OPTIONS:
  --threshold N   Warning threshold percentage (default: 80)
  --top N         Show only top N indexes by size (default: all)
  --help, -h      Show this help message

EXAMPLES:
  # Generate full disk usage report with default 80% threshold
  ./${SCRIPT_NAME}

  # Report with custom 90% warning threshold
  ./${SCRIPT_NAME} --threshold 90

  # Show only top 10 largest indexes
  ./${SCRIPT_NAME} --top 10

  # Combine options: top 5 indexes with 70% threshold
  ./${SCRIPT_NAME} --top 5 --threshold 70

ENVIRONMENT:
  SPLUNK_BASE_URL    Splunk REST API URL (required)
  SPLUNK_API_TOKEN   Splunk API token (preferred auth method)
  SPLUNK_USERNAME    Splunk username (alternative auth)
  SPLUNK_PASSWORD    Splunk password (alternative auth)
  NO_COLOR           Disable colored output when set

EXIT CODES:
  0   Report generated successfully
  1   Prerequisites not met or failed to retrieve data
  2   One or more indexes exceed warning threshold
EOF
}

check_prerequisites() {
  local errors=0

  # Check splunk-cli is installed
  if ! command -v splunk-cli &>/dev/null; then
    echo -e "${RED}ERROR:${NC} splunk-cli not found in PATH" >&2
    echo "Please install splunk-cli and ensure it's in your PATH" >&2
    ((errors++))
  fi

  # Check SPLUNK_BASE_URL is set
  if [[ -z "${SPLUNK_BASE_URL:-}" ]]; then
    echo -e "${RED}ERROR:${NC} SPLUNK_BASE_URL environment variable is not set" >&2
    echo "Please set SPLUNK_BASE_URL (e.g., https://splunk.example.com:8089)" >&2
    ((errors++))
  fi

  # Check authentication is configured
  if [[ -z "${SPLUNK_API_TOKEN:-}" && ( -z "${SPLUNK_USERNAME:-}" || -z "${SPLUNK_PASSWORD:-}" ) ]]; then
    echo -e "${RED}ERROR:${NC} No authentication configured" >&2
    echo "Please set either SPLUNK_API_TOKEN or both SPLUNK_USERNAME and SPLUNK_PASSWORD" >&2
    ((errors++))
  fi

  if [[ $errors -gt 0 ]]; then
    exit 1
  fi
}

format_bytes() {
  local bytes="$1"
  local unit="B"
  local value="$bytes"

  if [[ $bytes -ge 1099511627776 ]]; then
    value=$(awk "BEGIN {printf \"%.2f\", $bytes/1099511627776}")
    unit="TB"
  elif [[ $bytes -ge 1073741824 ]]; then
    value=$(awk "BEGIN {printf \"%.2f\", $bytes/1073741824}")
    unit="GB"
  elif [[ $bytes -ge 1048576 ]]; then
    value=$(awk "BEGIN {printf \"%.2f\", $bytes/1048576}")
    unit="MB"
  elif [[ $bytes -ge 1024 ]]; then
    value=$(awk "BEGIN {printf \"%.2f\", $bytes/1024}")
    unit="KB"
  fi

  echo "${value}${unit}"
}

parse_index_data() {
  # Fetch index data from splunk-cli
  if ! indexes_data=$(splunk-cli indexes list --detailed --output json 2>/dev/null); then
    echo -e "${RED}ERROR:${NC} Failed to retrieve index data from Splunk" >&2
    exit 1
  fi

  # Validate we got data
  if [[ -z "$indexes_data" ]] || [[ "$indexes_data" == "[]" ]]; then
    echo -e "${YELLOW}WARNING:${NC} No index data returned from Splunk" >&2
    exit 1
  fi
}

# =============================================================================
# Report Generation Functions
# =============================================================================

print_header() {
  echo
  echo -e "${BLUE}╔════════════════════════════════════════════════════════════════╗${NC}"
  echo -e "${BLUE}║${BOLD}           Splunk Index Disk Usage Report                       ${NC}${BLUE}║${NC}"
  echo -e "${BLUE}╚════════════════════════════════════════════════════════════════╝${NC}"
  echo
  echo -e "  Server:     ${CYAN}${SPLUNK_BASE_URL}${NC}"
  echo -e "  Threshold:  ${YELLOW}${THRESHOLD}%${NC}"
  echo -e "  Generated:  $(date '+%Y-%m-%d %H:%M:%S')"
}

print_summary() {
  local total_formatted
  total_formatted=$(format_bytes "$total_size_bytes")

  echo
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo -e "${BOLD}SUMMARY${NC}"
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo
  printf "  %-25s %s\n" "Total Indexes:" "${total_indexes}"
  printf "  %-25s %s\n" "Total Storage Used:" "${total_formatted}"

  if [[ ${#indexes_above_threshold[@]} -gt 0 ]]; then
    echo
    echo -e "  ${YELLOW}⚠ Indexes above ${THRESHOLD}% threshold:${NC} ${#indexes_above_threshold[@]}"
    for idx in "${indexes_above_threshold[@]}"; do
      echo -e "      - ${YELLOW}${idx}${NC}"
    done
  else
    echo
    echo -e "  ${GREEN}✓ All indexes below ${THRESHOLD}% threshold${NC}"
  fi
}

print_index_table() {
  local count=0

  echo
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo -e "${BOLD}INDEX DETAILS${NC}"
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo
  printf "  %-20s %12s %12s %8s\n" "INDEX NAME" "CURRENT" "MAX" "USAGE%"
  printf "  %-20s %12s %12s %8s\n" "--------------------" "------------" "------------" "--------"

  # Process each index (using jq-like parsing with grep/sed/awk)
  # This is a simplified parsing approach
  local index_names
  index_names=$(echo "$indexes_data" | grep -oP '"name":"[^"]+"' | sed 's/"name":"//' | sed 's/"$//' || true)

  for index_name in $index_names; do
    # Skip internal parsing artifacts
    [[ "$index_name" == "name" ]] && continue

    local current_size=0
    local max_size=0
    local usage_pct=0

    # Extract current size (this is a simplified parsing)
    current_size=$(echo "$indexes_data" | grep -A20 "\"name\":\"${index_name}\"" | grep -oP '"current_size":[0-9]+' | head -1 | cut -d: -f2 || echo "0")
    max_size=$(echo "$indexes_data" | grep -A20 "\"name\":\"${index_name}\"" | grep -oP '"max_size":[0-9]+' | head -1 | cut -d: -f2 || echo "0")

    # Handle null or missing values
    [[ -z "$current_size" ]] && current_size=0
    [[ -z "$max_size" ]] && max_size=0

    # Calculate usage percentage
    if [[ $max_size -gt 0 ]]; then
      usage_pct=$((current_size * 100 / max_size))
    else
      usage_pct=0
    fi

    # Format sizes
    local current_fmt
    local max_fmt
    current_fmt=$(format_bytes "$current_size")
    max_fmt=$(format_bytes "$max_size")

    # Track totals
    ((total_size_bytes += current_size))
    ((total_indexes++))

    # Check threshold
    local color="$GREEN"
    if [[ $usage_pct -ge $THRESHOLD ]]; then
      color="$YELLOW"
      indexes_above_threshold+=("${index_name} (${usage_pct}%)")
    fi
    if [[ $usage_pct -ge 95 ]]; then
      color="$RED"
    fi

    # Print row
    printf "  %-20s %12s %12s ${color}%7s%%${NC}\n" "${index_name:0:20}" "$current_fmt" "$max_fmt" "$usage_pct"

    # Respect --top limit
    ((count++))
    if [[ $TOP_COUNT -gt 0 && $count -ge $TOP_COUNT ]]; then
      break
    fi
  done
}

# =============================================================================
# Main
# =============================================================================

main() {
  # Parse arguments
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --threshold)
        if [[ -z "${2:-}" || "$2" =~ ^- ]]; then
          echo -e "${RED}ERROR:${NC} --threshold requires a value" >&2
          exit 1
        fi
        if ! [[ "$2" =~ ^[0-9]+$ ]] || [[ "$2" -lt 1 ]] || [[ "$2" -gt 100 ]]; then
          echo -e "${RED}ERROR:${NC} Threshold must be an integer between 1 and 100" >&2
          exit 1
        fi
        THRESHOLD="$2"
        shift 2
        ;;
      --top)
        if [[ -z "${2:-}" || "$2" =~ ^- ]]; then
          echo -e "${RED}ERROR:${NC} --top requires a value" >&2
          exit 1
        fi
        if ! [[ "$2" =~ ^[0-9]+$ ]] || [[ "$2" -lt 1 ]]; then
          echo -e "${RED}ERROR:${NC} --top must be a positive integer" >&2
          exit 1
        fi
        TOP_COUNT="$2"
        shift 2
        ;;
      --help|-h)
        show_help
        exit 0
        ;;
      *)
        echo -e "${RED}ERROR:${NC} Unknown option: $1" >&2
        echo "Use --help for usage information" >&2
        exit 1
        ;;
    esac
  done

  # Check prerequisites
  check_prerequisites

  # Fetch data
  parse_index_data

  # Generate report
  print_header
  print_index_table
  print_summary

  # Exit with appropriate code
  if [[ ${#indexes_above_threshold[@]} -gt 0 ]]; then
    echo
    echo -e "${YELLOW}Exit code 2: ${#indexes_above_threshold[@]} index(es) exceed threshold${NC}"
    exit 2
  fi

  echo
  echo -e "${GREEN}✓ Disk usage report completed successfully${NC}"
  exit 0
}

main "$@"
