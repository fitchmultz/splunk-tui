#!/usr/bin/env bash
# License usage analysis and reporting for Splunk capacity planning
#
# RESPONSIBILITY:
#   - Shows current license usage from splunk-cli license
#   - Searches license_logs for usage trends
#   - Shows daily peak usage analysis
#   - Warns if usage > 80% of quota
#
# DOES NOT:
#   - Modify license configurations
#   - Change license pools or allocations
#   - Access or modify license master settings
#
# PREREQUISITES:
#   - splunk-cli installed and in PATH
#   - SPLUNK_BASE_URL configured
#   - Authentication configured (SPLUNK_API_TOKEN or SPLUNK_USERNAME/PASSWORD)
#
# USAGE:
#   ./license-usage.sh [options]

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
LOOKBACK_DAYS=30
WARNING_THRESHOLD=80
CRITICAL_THRESHOLD=95

show_help() {
  cat << EOF
License Usage Analysis - Capacity Planning Tool

Analyzes Splunk license usage patterns and trends to help identify
potential capacity constraints before they impact operations.

USAGE:
  ${SCRIPT_NAME} [OPTIONS]

OPTIONS:
  -d, --days DAYS       Lookback period in days (default: ${LOOKBACK_DAYS})
  -w, --warn PERCENT    Warning threshold percentage (default: ${WARNING_THRESHOLD})
  -c, --critical PERCENT Critical threshold percentage (default: ${CRITICAL_THRESHOLD})
  -o, --output FORMAT   Output format: text, json (default: text)
  -h, --help            Show this help message

EXAMPLES:
  # Basic license analysis with default 30-day lookback
  ./${SCRIPT_NAME}

  # Analyze with custom thresholds
  ./${SCRIPT_NAME} -w 75 -c 90

  # JSON output for automation/monitoring
  ./${SCRIPT_NAME} -o json

ENVIRONMENT:
  SPLUNK_BASE_URL       Splunk REST API URL (required)
  SPLUNK_API_TOKEN      Splunk API token (preferred)
  SPLUNK_USERNAME       Splunk username (alternative)
  SPLUNK_PASSWORD       Splunk password (alternative)
  NO_COLOR              Disable colored output

EXIT CODES:
  0   Success, usage below warning threshold
  1   General error
  2   Missing prerequisites
  3   Invalid arguments
  4   Warning: usage above warning threshold
  5   Critical: usage above critical threshold
EOF
}

check_prerequisites() {
  local errors=0

  # Check splunk-cli exists
  if ! command -v splunk-cli &> /dev/null; then
    echo -e "${RED}Error: splunk-cli not found in PATH${NC}" >&2
    echo "Please install splunk-cli and ensure it's in your PATH" >&2
    errors=$((errors + 1))
  fi

  # Check SPLUNK_BASE_URL
  if [[ -z "${SPLUNK_BASE_URL:-}" ]]; then
    echo -e "${RED}Error: SPLUNK_BASE_URL is not set${NC}" >&2
    errors=$((errors + 1))
  fi

  # Check authentication
  if [[ -z "${SPLUNK_API_TOKEN:-}" && ( -z "${SPLUNK_USERNAME:-}" || -z "${SPLUNK_PASSWORD:-}" ) ]]; then
    echo -e "${RED}Error: Authentication not configured${NC}" >&2
    echo "Set either SPLUNK_API_TOKEN or SPLUNK_USERNAME/SPLUNK_PASSWORD" >&2
    errors=$((errors + 1))
  fi

  if [[ $errors -gt 0 ]]; then
    exit 2
  fi

  echo -e "${GREEN}✓${NC} Prerequisites verified"
}

# Parse command line arguments
parse_args() {
  OUTPUT_FORMAT="text"

  while [[ $# -gt 0 ]]; do
    case "$1" in
      -h|--help)
        show_help
        exit 0
        ;;
      -d|--days)
        if [[ -n "${2:-}" && "$2" =~ ^[0-9]+$ ]]; then
          LOOKBACK_DAYS="$2"
          shift 2
        else
          echo -e "${RED}Error: --days requires a numeric value${NC}" >&2
          exit 3
        fi
        ;;
      -w|--warn)
        if [[ -n "${2:-}" && "$2" =~ ^[0-9]+$ && "$2" -le 100 ]]; then
          WARNING_THRESHOLD="$2"
          shift 2
        else
          echo -e "${RED}Error: --warn requires a percentage (0-100)${NC}" >&2
          exit 3
        fi
        ;;
      -c|--critical)
        if [[ -n "${2:-}" && "$2" =~ ^[0-9]+$ && "$2" -le 100 ]]; then
          CRITICAL_THRESHOLD="$2"
          shift 2
        else
          echo -e "${RED}Error: --critical requires a percentage (0-100)${NC}" >&2
          exit 3
        fi
        ;;
      -o|--output)
        if [[ -n "${2:-}" && "$2" =~ ^(text|json)$ ]]; then
          OUTPUT_FORMAT="$2"
          shift 2
        else
          echo -e "${RED}Error: --output must be 'text' or 'json'${NC}" >&2
          exit 3
        fi
        ;;
      *)
        echo -e "${RED}Error: Unknown option: $1${NC}" >&2
        show_help
        exit 3
        ;;
    esac
  done

  # Validate thresholds
  if [[ "$WARNING_THRESHOLD" -ge "$CRITICAL_THRESHOLD" ]]; then
    echo -e "${RED}Error: Warning threshold must be less than critical threshold${NC}" >&2
    exit 3
  fi
}

# Format bytes to human-readable
format_bytes() {
  local bytes="$1"
  if command -v numfmt &> /dev/null; then
    numfmt --to=iec-i --suffix=B "$bytes" 2>/dev/null || echo "${bytes}B"
  else
    if [[ "$bytes" -lt 1024 ]]; then
      echo "${bytes}B"
    elif [[ "$bytes" -lt 1048576 ]]; then
      echo "$((bytes / 1024))KB"
    elif [[ "$bytes" -lt 1073741824 ]]; then
      echo "$((bytes / 1048576))MB"
    elif [[ "$bytes" -lt 1099511627776 ]]; then
      echo "$((bytes / 1073741824))GB"
    else
      echo "$(echo "scale=2; $bytes/1099511627776" | bc)TB"
    fi
  fi
}

# Fetch current license information
fetch_license_info() {
  echo -e "${BLUE}Fetching current license information...${NC}"

  local output
  if ! output=$(splunk-cli license --format json 2>&1); then
    echo -e "${RED}Failed to fetch license info: ${output}${NC}" >&2
    return 1
  fi

  echo "$output"
}

# Fetch license usage from license_logs
fetch_license_usage() {
  echo -e "${BLUE}Fetching license usage for last ${LOOKBACK_DAYS} days...${NC}"

  local search_query="search index=_internal source=*license_usage.log type=Usage earliest=-${LOOKBACK_DAYS}d@d latest=@d | eval date=strftime(_time, \"%Y-%m-%d\") | stats sum(b) as bytes by date | eval gb=round(bytes/1024/1024/1024,3) | sort date"

  local output
  if ! output=$(splunk-cli search "$search_query" --format json 2>&1); then
    echo -e "${RED}Failed to fetch license usage: ${output}${NC}" >&2
    return 1
  fi

  echo "$output"
}

# Calculate usage statistics
calculate_stats() {
  local usage_data="$1"

  local stats
  stats=$(echo "$usage_data" | jq -s '
    {
      total_days: length,
      avg_daily: (map(.bytes | tonumber) | add / length),
      max_daily: (map(.bytes | tonumber) | max),
      min_daily: (map(.bytes | tonumber) | min),
      total_usage: (map(.bytes | tonumber) | add)
    }
  ' 2>/dev/null)

  echo "$stats"
}

# Main analysis function
run_analysis() {
  echo -e "${BLUE}══════════════════════════════════════════════════════════════${NC}"
  echo -e "${BLUE}  License Usage Analysis - Capacity Planning${NC}"
  echo -e "${BLUE}══════════════════════════════════════════════════════════════${NC}"
  echo ""

  # Fetch license information
  local license_data
  if ! license_data=$(fetch_license_info); then
    exit 1
  fi

  # Fetch usage data
  local usage_data
  if ! usage_data=$(fetch_license_usage); then
    exit 1
  fi

  # Calculate statistics
  local stats
  stats=$(calculate_stats "$usage_data")

  # Determine exit code based on thresholds
  local exit_code=0
  local license_quota
  license_quota=$(echo "$license_data" | jq -r '.quota // 0' 2>/dev/null || echo "0")

  if [[ "$license_quota" -gt 0 ]]; then
    local avg_daily
    avg_daily=$(echo "$stats" | jq -r '.avg_daily // 0')
    local usage_percent
    usage_percent=$(echo "scale=2; ($avg_daily / $license_quota) * 100" | bc)

    if [[ $(echo "$usage_percent >= $CRITICAL_THRESHOLD" | bc -l) -eq 1 ]]; then
      exit_code=5
    elif [[ $(echo "$usage_percent >= $WARNING_THRESHOLD" | bc -l) -eq 1 ]]; then
      exit_code=4
    fi
  fi

  # Output results
  if [[ "$OUTPUT_FORMAT" == "json" ]]; then
    output_json "$license_data" "$usage_data" "$stats" "$exit_code"
  else
    output_text "$license_data" "$usage_data" "$stats" "$exit_code"
  fi

  exit "$exit_code"
}

output_text() {
  local license_data="$1"
  local usage_data="$2"
  local stats="$3"
  local exit_code="$4"

  # License Information
  echo -e "${YELLOW}Current License Configuration:${NC}"
  echo "───────────────────────────────────────────────────────────────"

  local license_quota
  license_quota=$(echo "$license_data" | jq -r '.quota // "N/A"' 2>/dev/null)
  local license_type
  license_type=$(echo "$license_data" | jq -r '.type // "N/A"' 2>/dev/null)
  local expiration
  expiration=$(echo "$license_data" | jq -r '.expiration // "N/A"' 2>/dev/null)

  if [[ "$license_quota" != "N/A" && "$license_quota" != "null" ]]; then
    echo "  License Quota:  $(format_bytes "$license_quota")"
  else
    echo "  License Quota:  (Unable to determine)"
  fi
  echo "  License Type:   ${license_type}"
  echo "  Expiration:     ${expiration}"

  echo ""
  echo -e "${YELLOW}Usage Analysis (${LOOKBACK_DAYS} days):${NC}"
  echo "───────────────────────────────────────────────────────────────"

  local avg_daily
  avg_daily=$(echo "$stats" | jq -r '.avg_daily // 0' | cut -d. -f1)
  local max_daily
  max_daily=$(echo "$stats" | jq -r '.max_daily // 0' | cut -d. -f1)
  local min_daily
  min_daily=$(echo "$stats" | jq -r '.min_daily // 0' | cut -d. -f1)
  local total_usage
  total_usage=$(echo "$stats" | jq -r '.total_usage // 0' | cut -d. -f1)

  if [[ "$avg_daily" -gt 0 ]]; then
    echo "  Average Daily:  $(format_bytes "$avg_daily")"
    echo "  Peak Daily:     $(format_bytes "$max_daily")"
    echo "  Minimum Daily:  $(format_bytes "$min_daily")"
    echo "  Total Usage:    $(format_bytes "$total_usage")"

    if [[ "$license_quota" != "N/A" && "$license_quota" != "null" && "$license_quota" -gt 0 ]]; then
      local avg_percent
      avg_percent=$(echo "scale=1; ($avg_daily / $license_quota) * 100" | bc)
      local max_percent
      max_percent=$(echo "scale=1; ($max_daily / $license_quota) * 100" | bc)

      echo ""
      echo "  Avg Utilization: ${avg_percent}% of quota"
      echo "  Peak Utilization: ${max_percent}% of quota"

      # Show daily breakdown
      echo ""
      echo -e "${YELLOW}Daily Usage Breakdown:${NC}"
      echo "───────────────────────────────────────────────────────────────"

      echo "$usage_data" | jq -r '.[] | "  \(.date): \(.gb) GB"' 2>/dev/null | head -10 || echo "  (Daily breakdown not available)"
      local count
      count=$(echo "$usage_data" | jq 'length' 2>/dev/null)
      if [[ "$count" -gt 10 ]]; then
        echo "  ... and $((count - 10)) more days"
      fi

      # Threshold warnings
      echo ""
      if [[ $(echo "$max_percent >= $CRITICAL_THRESHOLD" | bc -l) -eq 1 ]]; then
        echo -e "${RED}⚠ CRITICAL: Peak usage (${max_percent}%) exceeds critical threshold (${CRITICAL_THRESHOLD}%)${NC}"
      elif [[ $(echo "$avg_percent >= $WARNING_THRESHOLD" | bc -l) -eq 1 ]]; then
        echo -e "${YELLOW}⚠ WARNING: Average usage (${avg_percent}%) exceeds warning threshold (${WARNING_THRESHOLD}%)${NC}"
      else
        echo -e "${GREEN}✓ Usage within normal limits${NC}"
      fi
    fi
  else
    echo "  (No usage data available for the specified period)"
  fi

  echo ""
  if [[ "$exit_code" -eq 5 ]]; then
    echo -e "${RED}Analysis complete with CRITICAL status.${NC}"
  elif [[ "$exit_code" -eq 4 ]]; then
    echo -e "${YELLOW}Analysis complete with WARNING status.${NC}"
  else
    echo -e "${GREEN}Analysis complete.${NC}"
  fi
}

output_json() {
  local license_data="$1"
  local usage_data="$2"
  local stats="$3"
  local exit_code="$4"

  jq -n \
    --arg timestamp "$(date -Iseconds)" \
    --arg lookback "$LOOKBACK_DAYS" \
    --argjson license "$license_data" \
    --argjson usage "$usage_data" \
    --argjson stats "$stats" \
    --arg exit_code "$exit_code" \
    --arg warning_threshold "$WARNING_THRESHOLD" \
    --arg critical_threshold "$CRITICAL_THRESHOLD" \
    '{
      timestamp: $timestamp,
      lookback_days: ($lookback | tonumber),
      license_info: $license,
      daily_usage: $usage,
      statistics: $stats,
      thresholds: {
        warning: ($warning_threshold | tonumber),
        critical: ($critical_threshold | tonumber)
      },
      status: (if ($exit_code | tonumber) == 5 then "critical" elif ($exit_code | tonumber) == 4 then "warning" else "ok" end),
      exit_code: ($exit_code | tonumber)
    }'
}

# Main entry point
main() {
  parse_args "$@"
  check_prerequisites
  run_analysis
}

main "$@"
