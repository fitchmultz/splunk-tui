#!/usr/bin/env bash
# Analyze index growth trends and project capacity needs
#
# RESPONSIBILITY:
#   - Shows current index sizes from splunk-cli indexes list --detailed
#   - Searches _internal for ingestion metrics
#   - Calculates daily ingestion rates
#   - Projects capacity needs for 30/60/90 days
#
# DOES NOT:
#   - Modify any index settings or data
#   - Access data outside of _internal and indexes list
#   - Make permanent changes to Splunk configuration
#
# PREREQUISITES:
#   - splunk-cli installed and in PATH
#   - SPLUNK_BASE_URL configured
#   - Authentication configured (SPLUNK_API_TOKEN or SPLUNK_USERNAME/PASSWORD)
#
# USAGE:
#   ./index-growth.sh [options]

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
LOOKBACK_DAYS=7

show_help() {
  cat << EOF
Index Growth Analysis - Capacity Planning Tool

Analyzes index growth trends and projects future capacity needs based on
ingestion metrics from the _internal index.

USAGE:
  ${SCRIPT_NAME} [OPTIONS]

OPTIONS:
  -d, --days DAYS       Lookback period in days (default: ${LOOKBACK_DAYS})
  -p, --project DAYS    Custom projection days (comma-separated, default: 30,60,90)
  -o, --output FORMAT   Output format: text, json (default: text)
  -h, --help            Show this help message

EXAMPLES:
  # Basic analysis with default 7-day lookback
  ./${SCRIPT_NAME}

  # Analyze with 14-day lookback and custom projections
  ./${SCRIPT_NAME} -d 14 -p 30,60,90,180

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
  PROJECTION_DAYS="30,60,90"
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
      -p|--project)
        if [[ -n "${2:-}" ]]; then
          PROJECTION_DAYS="$2"
          shift 2
        else
          echo -e "${RED}Error: --project requires a value${NC}" >&2
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

# Format bytes to human-readable
format_bytes() {
  local bytes="$1"
  if [[ "$bytes" -lt 1024 ]]; then
    echo "${bytes}B"
  elif [[ "$bytes" -lt 1048576 ]]; then
    echo "$(echo "scale=2; $bytes/1024" | bc)KB"
  elif [[ "$bytes" -lt 1073741824 ]]; then
    echo "$(echo "scale=2; $bytes/1048576" | bc)MB"
  elif [[ "$bytes" -lt 1099511627776 ]]; then
    echo "$(echo "scale=2; $bytes/1073741824" | bc)GB"
  else
    echo "$(echo "scale=2; $bytes/1099511627776" | bc)TB"
  fi
}

# Fetch current index sizes
fetch_index_sizes() {
  echo -e "${BLUE}Fetching current index sizes...${NC}"

  local output
  if ! output=$(splunk-cli indexes list --detailed --format json 2>&1); then
    echo -e "${RED}Failed to fetch index sizes: ${output}${NC}" >&2
    return 1
  fi

  echo "$output"
}

# Fetch ingestion metrics from _internal
fetch_ingestion_metrics() {
  echo -e "${BLUE}Fetching ingestion metrics for last ${LOOKBACK_DAYS} days...${NC}"

  local search_query="search index=_internal source=*metrics.log group=per_index_thruput earliest=-${LOOKBACK_DAYS}d@d latest=@d | stats sum(kb) as daily_kb by date_mday, series | eval daily_bytes=daily_kb*1024"

  local output
  if ! output=$(splunk-cli search "$search_query" --format json 2>&1); then
    echo -e "${RED}Failed to fetch ingestion metrics: ${output}${NC}" >&2
    return 1
  fi

  echo "$output"
}

# Calculate growth projections
calculate_projections() {
  local current_size="$1"
  local daily_rate="$2"
  local projection_days="$3"

  local projected_size
  projected_size=$(echo "$current_size + ($daily_rate * $projection_days)" | bc)
  echo "$projected_size"
}

# Main analysis function
run_analysis() {
  echo -e "${BLUE}══════════════════════════════════════════════════════════════${NC}"
  echo -e "${BLUE}  Index Growth Analysis - Capacity Planning${NC}"
  echo -e "${BLUE}══════════════════════════════════════════════════════════════${NC}"
  echo ""

  # Fetch current index sizes
  local index_data
  if ! index_data=$(fetch_index_sizes); then
    exit 1
  fi

  # Fetch ingestion metrics
  local ingestion_data
  if ! ingestion_data=$(fetch_ingestion_metrics); then
    exit 1
  fi

  # Output results based on format
  if [[ "$OUTPUT_FORMAT" == "json" ]]; then
    output_json "$index_data" "$ingestion_data"
  else
    output_text "$index_data" "$ingestion_data"
  fi
}

output_text() {
  local index_data="$1"
  local ingestion_data="$2"

  echo -e "${YELLOW}Current Index Sizes:${NC}"
  echo "───────────────────────────────────────────────────────────────"

  # Parse and display index sizes
  echo "$index_data" | jq -r '.[] | select(.current_size_bytes != null) | "  \(.name): \(.current_size_bytes // 0) bytes"' 2>/dev/null || echo "  (Unable to parse index data)"

  echo ""
  echo -e "${YELLOW}Ingestion Analysis (${LOOKBACK_DAYS} day average):${NC}"
  echo "───────────────────────────────────────────────────────────────"

  # Calculate daily ingestion rates by index
  local daily_totals
  daily_totals=$(echo "$ingestion_data" | jq -s 'group_by(.series) | map({index: .[0].series, total_bytes: map(.daily_bytes | tonumber) | add, days: length, avg_daily: (map(.daily_bytes | tonumber) | add / '"$LOOKBACK_DAYS"')})' 2>/dev/null)

  if [[ -n "$daily_totals" && "$daily_totals" != "null" && "$daily_totals" != "[]" ]]; then
    echo "$daily_totals" | jq -r '.[] | "  \(.index): \(.avg_daily | floor) bytes/day (\(.days) days of data)"'
  else
    echo "  (No ingestion data available for analysis)"
  fi

  echo ""
  echo -e "${YELLOW}Capacity Projections:${NC}"
  echo "───────────────────────────────────────────────────────────────"

  # Parse projection days
  IFS=',' read -ra PROJECTIONS <<< "$PROJECTION_DAYS"

  for day in "${PROJECTIONS[@]}"; do
    echo -e "  ${GREEN}${day}-day projection:${NC}"
    # Show projections per index if we have data
    if [[ -n "$daily_totals" && "$daily_totals" != "null" && "$daily_totals" != "[]" ]]; then
      echo "$daily_totals" | jq -r --arg day "$day" '.[] | "    \(.index): +\(.avg_daily * ($day | tonumber) | floor) bytes"' 2>/dev/null || true
    fi
  done

  echo ""
  echo -e "${GREEN}Analysis complete.${NC}"
}

output_json() {
  local index_data="$1"
  local ingestion_data="$2"

  local daily_totals
  daily_totals=$(echo "$ingestion_data" | jq -s 'group_by(.series) | map({index: .[0].series, total_bytes: map(.daily_bytes | tonumber) | add, days: length, avg_daily: (map(.daily_bytes | tonumber) | add / '"$LOOKBACK_DAYS"')})' 2>/dev/null)

  jq -n \
    --arg timestamp "$(date -Iseconds)" \
    --arg lookback "$LOOKBACK_DAYS" \
    --argjson indexes "$index_data" \
    --argjson ingestion "$daily_totals" \
    --arg projections "$PROJECTION_DAYS" \
    '{
      timestamp: $timestamp,
      lookback_days: ($lookback | tonumber),
      indexes: $indexes,
      ingestion_analysis: $ingestion,
      projection_days: ($projections | split(","))
    }'
}

# Main entry point
main() {
  parse_args "$@"
  check_prerequisites
  run_analysis
}

main "$@"
