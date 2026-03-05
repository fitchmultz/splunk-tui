#!/usr/bin/env bash
# Generate and save scheduled reports from Splunk saved searches
#
# RESPONSIBILITY:
#   Runs saved searches and exports results to timestamped files in multiple
#   formats (JSON, CSV). Designed to be run from cron for true scheduled
#   report generation.
#
# DOES NOT:
#   - Create or modify saved searches (read-only operation on searches)
#   - Handle report formatting or styling (exports raw data only)
#   - Send emails or notifications (use external tools for that)
#
# PREREQUISITES:
#   - splunk-cli installed and in PATH
#   - SPLUNK_BASE_URL configured
#   - Authentication configured (SPLUNK_API_TOKEN or SPLUNK_USERNAME/PASSWORD)
#
# USAGE:
#   ./scheduled-reports.sh [options]

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
REPORT_NAME=""
OUTPUT_DIR="./reports"
FORMAT="json"
TIMEOUT=300

# Show help information
show_help() {
  cat << 'EOF'
Usage: ./scheduled-reports.sh [OPTIONS]

Generate and save scheduled reports from Splunk saved searches.

OPTIONS:
    --report <name>       Name of the saved search to run (required)
    --output-dir <dir>    Output directory for reports (default: ./reports)
    --format <format>     Output format: json, csv (default: json)
    --timeout <seconds>   Search timeout in seconds (default: 300)
    --list                List available saved searches
    -h, --help            Show this help message

EXAMPLES:
    # Run a saved search and save as JSON
    ./scheduled-reports.sh --report "Daily Errors"

    # Export to CSV in a specific directory
    ./scheduled-reports.sh --report "Security Events" --format csv --output-dir /var/reports

    # List all available saved searches
    ./scheduled-reports.sh --list

CRON SETUP:
    # Run daily at 6 AM
    0 6 * * * /path/to/scheduled-reports.sh --report "Daily Summary" --output-dir /mnt/reports

ENVIRONMENT:
    SPLUNK_BASE_URL       Splunk REST API URL (required)
    SPLUNK_API_TOKEN      Splunk API token (preferred)
    SPLUNK_USERNAME       Splunk username (alternative)
    SPLUNK_PASSWORD       Splunk password (alternative)
    NO_COLOR              Disable colored output
EOF
}

# Check prerequisites
check_prerequisites() {
  local missing=()

  if ! command -v splunk-cli &> /dev/null; then
    missing+=("splunk-cli")
  fi

  if [[ -z "${SPLUNK_BASE_URL:-}" ]]; then
    missing+=("SPLUNK_BASE_URL")
  fi

  if [[ -z "${SPLUNK_API_TOKEN:-}" && (-z "${SPLUNK_USERNAME:-}" || -z "${SPLUNK_PASSWORD:-}") ]]; then
    missing+=("authentication (SPLUNK_API_TOKEN or SPLUNK_USERNAME/PASSWORD)")
  fi

  if [[ ${#missing[@]} -gt 0 ]]; then
    echo -e "${RED}Error: Missing prerequisites:${NC}" >&2
    for item in "${missing[@]}"; do
      echo "  - $item" >&2
    done
    exit 1
  fi
}

# List available saved searches
list_saved_searches() {
  echo -e "${BLUE}Available saved searches:${NC}"
  if ! splunk-cli saved-searches list 2>/dev/null; then
    echo -e "${RED}Failed to list saved searches${NC}" >&2
    exit 1
  fi
}

# Parse command line arguments
parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --report)
        REPORT_NAME="$2"
        shift 2
        ;;
      --output-dir)
        OUTPUT_DIR="$2"
        shift 2
        ;;
      --format)
        FORMAT="$2"
        shift 2
        ;;
      --timeout)
        TIMEOUT="$2"
        shift 2
        ;;
      --list)
        check_prerequisites
        list_saved_searches
        exit 0
        ;;
      -h|--help)
        show_help
        exit 0
        ;;
      *)
        echo -e "${RED}Error: Unknown option: $1${NC}" >&2
        show_help
        exit 1
        ;;
    esac
  done

  if [[ -z "$REPORT_NAME" ]]; then
    echo -e "${RED}Error: --report is required${NC}" >&2
    show_help
    exit 1
  fi

  if [[ "$FORMAT" != "json" && "$FORMAT" != "csv" ]]; then
    echo -e "${RED}Error: Invalid format '$FORMAT'. Use 'json' or 'csv'${NC}" >&2
    exit 1
  fi
}

# Generate timestamped filename
generate_filename() {
  local report_name="$1"
  local format="$2"
  local timestamp
  timestamp=$(date +%Y%m%d_%H%M%S)
  # Sanitize report name for filename
  local safe_name
  safe_name=$(echo "$report_name" | tr ' ' '_' | tr -cd '[:alnum:]_-')
  echo "${safe_name}_${timestamp}.${format}"
}

# Main execution
main() {
  parse_args "$@"
  check_prerequisites

  # Create output directory
  if [[ ! -d "$OUTPUT_DIR" ]]; then
    echo -e "${BLUE}Creating output directory: $OUTPUT_DIR${NC}"
    mkdir -p "$OUTPUT_DIR" || {
      echo -e "${RED}Failed to create output directory: $OUTPUT_DIR${NC}" >&2
      exit 1
    }
  fi

  local filename
  filename=$(generate_filename "$REPORT_NAME" "$FORMAT")
  local filepath="${OUTPUT_DIR}/${filename}"

  echo -e "${BLUE}Running saved search: ${REPORT_NAME}${NC}"
  echo -e "${BLUE}Output file: ${filepath}${NC}"

  # Run the saved search and export results
  local output_format="$FORMAT"
  if [[ "$FORMAT" == "csv" ]]; then
    output_format="csv"
  fi

  if splunk-cli saved-searches run "$REPORT_NAME" --output-format "$output_format" --output-file "$filepath" --timeout "$TIMEOUT"; then
    # Verify file was created and has content
    if [[ -f "$filepath" && -s "$filepath" ]]; then
      local filesize
      filesize=$(du -h "$filepath" | cut -f1)
      echo -e "${GREEN}✓ Report generated successfully${NC}"
      echo -e "${GREEN}  File: $filepath${NC}"
      echo -e "${GREEN}  Size: $filesize${NC}"

      # Count results if JSON
      if [[ "$FORMAT" == "json" ]]; then
        local count
        count=$(jq length "$filepath" 2>/dev/null || echo "unknown")
        echo -e "${GREEN}  Results: $count${NC}"
      fi

      exit 0
    else
      echo -e "${YELLOW}Warning: Report file is empty or was not created${NC}" >&2
      exit 1
    fi
  else
    echo -e "${RED}Failed to generate report${NC}" >&2
    exit 1
  fi
}

main "$@"
