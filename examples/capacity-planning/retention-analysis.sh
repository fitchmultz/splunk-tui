#!/usr/bin/env bash
# Analyze data retention policies and identify optimization opportunities
#
# RESPONSIBILITY:
#   - Reviews retention settings vs actual data age
#   - Shows indexes with custom retention
#   - Identifies potential cost optimizations
#   - Uses: splunk-cli indexes list --detailed, splunk-cli search
#
# DOES NOT:
#   - Modify any retention settings
#   - Delete or alter any data
#   - Change index configurations
#
# PREREQUISITES:
#   - splunk-cli installed and in PATH
#   - SPLUNK_BASE_URL configured
#   - Authentication configured (SPLUNK_API_TOKEN or SPLUNK_USERNAME/PASSWORD)
#
# USAGE:
#   ./retention-analysis.sh [options]

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

# Script configuration
SCRIPT_NAME="$(basename "$0")"
DEFAULT_RETENTION_DAYS=90

show_help() {
  cat << EOF
Retention Analysis - Capacity Planning Tool

Analyzes data retention policies across indexes to identify:
- Indexes with custom retention settings
- Potential cost optimization opportunities
- Retention compliance gaps

USAGE:
  ${SCRIPT_NAME} [OPTIONS]

OPTIONS:
  -d, --default DAYS    Default retention in days (default: ${DEFAULT_RETENTION_DAYS})
  -i, --index NAME      Analyze specific index only
  -o, --output FORMAT   Output format: text, json (default: text)
  -h, --help            Show this help message

EXAMPLES:
  # Analyze all indexes with default 90-day comparison
  ./${SCRIPT_NAME}

  # Analyze with custom default retention
  ./${SCRIPT_NAME} -d 180

  # Analyze specific index
  ./${SCRIPT_NAME} -i main

  # JSON output for automation
  ./${SCRIPT_NAME} -o json

ENVIRONMENT:
  SPLUNK_BASE_URL       Splunk REST API URL (required)
  SPLUNK_API_TOKEN      Splunk API token (preferred)
  SPLUNK_USERNAME       Splunk username (alternative)
  SPLUNK_PASSWORD       Splunk password (alternative)
  NO_COLOR              Disable colored output

EXIT CODES:
  0   Success
  1   General error
  2   Missing prerequisites
  3   Invalid arguments
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
  SPECIFIC_INDEX=""

  while [[ $# -gt 0 ]]; do
    case "$1" in
      -h|--help)
        show_help
        exit 0
        ;;
      -d|--default)
        if [[ -n "${2:-}" && "$2" =~ ^[0-9]+$ ]]; then
          DEFAULT_RETENTION_DAYS="$2"
          shift 2
        else
          echo -e "${RED}Error: --default requires a numeric value${NC}" >&2
          exit 3
        fi
        ;;
      -i|--index)
        if [[ -n "${2:-}" ]]; then
          SPECIFIC_INDEX="$2"
          shift 2
        else
          echo -e "${RED}Error: --index requires a value${NC}" >&2
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
}

# Format seconds to human-readable duration
format_duration() {
  local seconds="$1"
  local days=$((seconds / 86400))

  if [[ $days -ge 365 ]]; then
    local years
    years=$(echo "scale=1; $days/365" | bc)
    echo "${years}y"
  elif [[ $days -ge 30 ]]; then
    local months
    months=$(echo "scale=1; $days/30" | bc)
    echo "${months}mo"
  else
    echo "${days}d"
  fi
}

# Format bytes to human-readable
format_bytes() {
  local bytes="$1"
  if [[ "$bytes" -lt 1024 ]]; then
    echo "${bytes}B"
  elif [[ "$bytes" -lt 1048576 ]]; then
    echo "$(echo "scale=1; $bytes/1024" | bc)KB"
  elif [[ "$bytes" -lt 1073741824 ]]; then
    echo "$(echo "scale=1; $bytes/1048576" | bc)MB"
  elif [[ "$bytes" -lt 1099511627776 ]]; then
    echo "$(echo "scale=1; $bytes/1073741824" | bc)GB"
  else
    echo "$(echo "scale=2; $bytes/1099511627776" | bc)TB"
  fi
}

# Fetch index details
fetch_index_details() {
  echo -e "${BLUE}Fetching index details...${NC}"

  local output
  if [[ -n "$SPECIFIC_INDEX" ]]; then
    if ! output=$(splunk-cli indexes list --detailed --format json 2>&1); then
      echo -e "${RED}Failed to fetch index details: ${output}${NC}" >&2
      return 1
    fi
    # Filter to specific index
    echo "$output" | jq --arg idx "$SPECIFIC_INDEX" '[.[] | select(.name == $idx)]'
  else
    if ! output=$(splunk-cli indexes list --detailed --format json 2>&1); then
      echo -e "${RED}Failed to fetch index details: ${output}${NC}" >&2
      return 1
    fi
    echo "$output"
  fi
}

# Fetch actual data age from search
fetch_data_age() {
  local index_name="$1"

  local search_query="search index=${index_name} earliest=0 latest=now | head 1 | eval age_days=round((now()-_time)/86400,1) | stats max(age_days) as oldest_event"

  local output
  output=$(splunk-cli search "$search_query" --format json 2>&1) || true

  if [[ -n "$output" && "$output" != "null" && "$output" != "[]" ]]; then
    echo "$output" | jq -r '.[0].oldest_event // "unknown"'
  else
    echo "unknown"
  fi
}

# Analyze retention settings
analyze_retention() {
  local index_data="$1"

  local analysis
  analysis=$(echo "$index_data" | jq --arg default "$DEFAULT_RETENTION_DAYS" '
    map({
      name: .name,
      current_size_bytes: (.current_size_bytes // 0 | tonumber),
      max_size_bytes: (.max_size_bytes // 0 | tonumber),
      frozen_time_secs: (.frozen_time_secs // 0 | tonumber),
      retention_days: ((.frozen_time_secs // 0 | tonumber) / 86400 | floor),
      default_retention: ($default | tonumber),
      has_custom_retention: (((.frozen_time_secs // 0 | tonumber) / 86400 | floor) != ($default | tonumber))
    })
  ')

  echo "$analysis"
}

# Identify optimization opportunities
find_optimizations() {
  local analysis="$1"

  local optimizations
  optimizations=$(echo "$analysis" | jq '
    {
      oversized: map(select(.current_size_bytes > 0 and .max_size_bytes > 0 and .current_size_bytes > .max_size_bytes * 0.9) | {name: .name, issue: "near_capacity"}),
      short_retention_large: map(select(.current_size_bytes > 10737418240 and .retention_days < 30) | {name: .name, issue: "short_retention_large_volume", size: .current_size_bytes}),
      long_retention_small: map(select(.current_size_bytes < 1073741824 and .retention_days > 365) | {name: .name, issue: "long_retention_small_volume", retention: .retention_days}),
      unlimited_size: map(select(.max_size_bytes == 0 or .max_size_bytes == null) | {name: .name, issue: "unlimited_size"})
    }
  ')

  echo "$optimizations"
}

# Main analysis function
run_analysis() {
  echo -e "${BLUE}══════════════════════════════════════════════════════════════${NC}"
  echo -e "${BLUE}  Retention Analysis - Capacity Planning${NC}"
  echo -e "${BLUE}══════════════════════════════════════════════════════════════${NC}"
  echo ""

  if [[ -n "$SPECIFIC_INDEX" ]]; then
    echo -e "${CYAN}Analyzing index: ${SPECIFIC_INDEX}${NC}"
    echo ""
  fi

  # Fetch index details
  local index_data
  if ! index_data=$(fetch_index_details); then
    exit 1
  fi

  # Check if we have data
  local count
  count=$(echo "$index_data" | jq 'length')
  if [[ "$count" -eq 0 ]]; then
    if [[ -n "$SPECIFIC_INDEX" ]]; then
      echo -e "${RED}Error: Index '${SPECIFIC_INDEX}' not found${NC}" >&2
    else
      echo -e "${RED}Error: No indexes found${NC}" >&2
    fi
    exit 1
  fi

  # Analyze retention
  local analysis
  analysis=$(analyze_retention "$index_data")

  # Find optimizations
  local optimizations
  optimizations=$(find_optimizations "$analysis")

  # Output results
  if [[ "$OUTPUT_FORMAT" == "json" ]]; then
    output_json "$analysis" "$optimizations"
  else
    output_text "$analysis" "$optimizations"
  fi
}

output_text() {
  local analysis="$1"
  local optimizations="$2"

  # Summary
  local total_indexes
  total_indexes=$(echo "$analysis" | jq 'length')
  local custom_count
  custom_count=$(echo "$analysis" | jq '[.[] | select(.has_custom_retention)] | length')
  local total_size
  total_size=$(echo "$analysis" | jq '[.[] | .current_size_bytes] | add')

  echo -e "${YELLOW}Summary:${NC}"
  echo "───────────────────────────────────────────────────────────────"
  echo "  Total Indexes:      ${total_indexes}"
  echo "  Custom Retention:   ${custom_count}"
  echo "  Total Data Size:    $(format_bytes "$total_size")"
  echo "  Default Retention:  ${DEFAULT_RETENTION_DAYS} days"

  # Indexes with custom retention
  echo ""
  echo -e "${YELLOW}Indexes with Custom Retention:${NC}"
  echo "───────────────────────────────────────────────────────────────"

  local custom_indexes
  custom_indexes=$(echo "$analysis" | jq '[.[] | select(.has_custom_retention)]')

  if [[ "$custom_indexes" != "[]" && -n "$custom_indexes" ]]; then
    echo "$custom_indexes" | jq -r '.[] | "  \(.name)\n    Retention: \(.retention_days)d (default: \(.default_retention)d)\n    Size: \(.current_size_bytes) bytes"' | while IFS= read -r line; do
      if [[ "$line" =~ ^[[:space:]]+Size: ]]; then
        local size
        size=$(echo "$line" | grep -oE '[0-9]+' | head -1)
        echo "    Size: $(format_bytes "$size")"
      else
        echo "$line"
      fi
    done
  else
    echo "  (All indexes use default retention)"
  fi

  # All indexes detail
  echo ""
  echo -e "${YELLOW}Index Retention Details:${NC}"
  echo "───────────────────────────────────────────────────────────────"
  printf "  %-20s %-12s %-15s %-12s\n" "Index" "Retention" "Size" "Max Size"
  printf "  %-20s %-12s %-15s %-12s\n" "─────" "─────────" "────" "────────"

  echo "$analysis" | jq -r '.[] | "\(.name)|\(.retention_days)d|\(.current_size_bytes)|\(.max_size_bytes)"' | while IFS='|' read -r name retention current max; do
    local size_fmt
    size_fmt=$(format_bytes "$current")
    local max_fmt
    if [[ "$max" == "0" || "$max" == "null" ]]; then
      max_fmt="unlimited"
    else
      max_fmt=$(format_bytes "$max")
    fi
    printf "  %-20s %-12s %-15s %-12s\n" "$name" "$retention" "$size_fmt" "$max_fmt"
  done

  # Optimization opportunities
  echo ""
  echo -e "${YELLOW}Optimization Opportunities:${NC}"
  echo "───────────────────────────────────────────────────────────────"

  local has_opportunities=false

  # Near capacity
  local near_capacity
  near_capacity=$(echo "$optimizations" | jq '.oversized[]')
  if [[ -n "$near_capacity" && "$near_capacity" != "null" && "$near_capacity" != "[]" ]]; then
    has_opportunities=true
    echo -e "  ${RED}⚠ Near Capacity (over 90% of max size):${NC}"
    echo "$optimizations" | jq -r '.oversized[] | "    - \(.name)"'
    echo ""
  fi

  # Short retention with large volume
  local short_retention
  short_retention=$(echo "$optimizations" | jq '.short_retention_large[]')
  if [[ -n "$short_retention" && "$short_retention" != "null" && "$short_retention" != "[]" ]]; then
    has_opportunities=true
    echo -e "  ${YELLOW}⚠ Large Volume with Short Retention (<30 days):${NC}"
    echo "$optimizations" | jq -r '.short_retention_large[] | "    - \(.name) ($(format_bytes \(.size)))"'
    echo ""
  fi

  # Long retention with small volume
  local long_retention
  long_retention=$(echo "$optimizations" | jq '.long_retention_small[]')
  if [[ -n "$long_retention" && "$long_retention" != "null" && "$long_retention" != "[]" ]]; then
    has_opportunities=true
    echo -e "  ${CYAN}ℹ Long Retention on Small Volume (>365 days, <1GB):${NC}"
    echo "$optimizations" | jq -r '.long_retention_small[] | "    - \(.name) (\(.retention)d)"'
    echo ""
  fi

  # Unlimited size
  local unlimited
  unlimited=$(echo "$optimizations" | jq '.unlimited_size[]')
  if [[ -n "$unlimited" && "$unlimited" != "null" && "$unlimited" != "[]" ]]; then
    has_opportunities=true
    echo -e "  ${YELLOW}⚠ Unlimited Max Size:${NC}"
    echo "$optimizations" | jq -r '.unlimited_size[] | "    - \(.name)"'
    echo ""
  fi

  if [[ "$has_opportunities" == false ]]; then
    echo -e "  ${GREEN}✓ No obvious optimization issues detected${NC}"
  fi

  # Recommendations
  echo ""
  echo -e "${YELLOW}Recommendations:${NC}"
  echo "───────────────────────────────────────────────────────────────"
  echo "  • Review indexes with custom retention for business need"
  echo "  • Consider extending retention for high-value data"
  echo "  • Archive or reduce retention for low-value, high-volume indexes"
  echo "  • Set maxDataSize limits on unlimited indexes for predictability"

  echo ""
  echo -e "${GREEN}Analysis complete.${NC}"
}

output_json() {
  local analysis="$1"
  local optimizations="$2"

  local total_size
  total_size=$(echo "$analysis" | jq '[.[] | .current_size_bytes] | add')
  local custom_count
  custom_count=$(echo "$analysis" | jq '[.[] | select(.has_custom_retention)] | length')

  jq -n \
    --arg timestamp "$(date -Iseconds)" \
    --arg default_retention "$DEFAULT_RETENTION_DAYS" \
    --argjson analysis "$analysis" \
    --argjson optimizations "$optimizations" \
    --arg total_size "$total_size" \
    --arg custom_count "$custom_count" \
    '{
      timestamp: $timestamp,
      default_retention_days: ($default_retention | tonumber),
      summary: {
        total_indexes: ($analysis | length),
        custom_retention_count: ($custom_count | tonumber),
        total_data_bytes: ($total_size | tonumber)
      },
      indexes: $analysis,
      optimization_opportunities: $optimizations
    }'
}

# Main entry point
main() {
  parse_args "$@"
  check_prerequisites
  run_analysis
}

main "$@"
