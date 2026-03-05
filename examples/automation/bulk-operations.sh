#!/usr/bin/env bash
# Perform bulk operations on Splunk saved searches
#
# RESPONSIBILITY:
#   Executes bulk enable, disable, or delete operations on saved searches
#   from a list file. Defaults to dry-run mode for safety; use --execute
#   to apply changes.
#
# DOES NOT:
#   - Modify searches without explicit --execute flag (dry-run by default)
#   - Create or edit search definitions (only toggles state)
#   - Handle complex filtering or selection (operates on explicit list only)
#
# PREREQUISITES:
#   - splunk-cli installed and in PATH
#   - SPLUNK_BASE_URL configured
#   - Authentication configured (SPLUNK_API_TOKEN or SPLUNK_USERNAME/PASSWORD)
#
# USAGE:
#   ./bulk-operations.sh [options]

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
OPERATION=""
INPUT_FILE=""
EXECUTE=false
SUMMARY_ONLY=false

# Show help information
show_help() {
  cat << 'EOF'
Usage: ./bulk-operations.sh [OPTIONS]

Perform bulk operations on Splunk saved searches.

OPERATIONS:
    disable-searches      Disable all searches in the input file
    enable-searches       Enable all searches in the input file
    delete-searches       Delete all searches in the input file

OPTIONS:
    --operation <op>      Operation to perform (required)
    --file <file>         File containing search names, one per line (required)
    --execute             Actually perform the operation (default: dry-run)
    --summary-only        Only show summary, skip detailed progress
    -h, --help            Show this help message

EXAMPLES:
    # Dry-run: see what would be disabled
    ./bulk-operations.sh --operation disable-searches --file searches.txt

    # Actually disable searches
    ./bulk-operations.sh --operation disable-searches --file searches.txt --execute

    # Enable searches from a list
    ./bulk-operations.sh --operation enable-searches --file to-enable.txt --execute

    # Delete searches (use with caution!)
    ./bulk-operations.sh --operation delete-searches --file obsolete.txt --execute

INPUT FILE FORMAT:
    One search name per line. Empty lines and lines starting with # are ignored.

    Example searches.txt:
    # Production searches to disable
    Daily Report
    Weekly Summary
    Error Monitor

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

# Parse command line arguments
parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --operation)
        OPERATION="$2"
        shift 2
        ;;
      --file)
        INPUT_FILE="$2"
        shift 2
        ;;
      --execute)
        EXECUTE=true
        shift
        ;;
      --summary-only)
        SUMMARY_ONLY=true
        shift
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

  if [[ -z "$OPERATION" ]]; then
    echo -e "${RED}Error: --operation is required${NC}" >&2
    show_help
    exit 1
  fi

  if [[ "$OPERATION" != "disable-searches" && "$OPERATION" != "enable-searches" && "$OPERATION" != "delete-searches" ]]; then
    echo -e "${RED}Error: Invalid operation '$OPERATION'${NC}" >&2
    echo "Valid operations: disable-searches, enable-searches, delete-searches" >&2
    exit 1
  fi

  if [[ -z "$INPUT_FILE" ]]; then
    echo -e "${RED}Error: --file is required${NC}" >&2
    show_help
    exit 1
  fi

  if [[ ! -f "$INPUT_FILE" ]]; then
    echo -e "${RED}Error: Input file not found: $INPUT_FILE${NC}" >&2
    exit 1
  fi
}

# Read search names from file, ignoring comments and empty lines
read_search_names() {
  local file="$1"
  grep -v '^#' "$file" | grep -v '^[[:space:]]*$' || true
}

# Perform operation on a single search
perform_operation() {
  local operation="$1"
  local search_name="$2"
  local action=""

  case "$operation" in
    disable-searches)
      action="disable"
      ;;
    enable-searches)
      action="enable"
      ;;
    delete-searches)
      action="delete"
      ;;
  esac

  splunk-cli saved-searches "$action" "$search_name" 2>/dev/null
}

# Main execution
main() {
  parse_args "$@"
  check_prerequisites

  # Read search names
  local searches=()
  while IFS= read -r line; do
    [[ -n "$line" ]] && searches+=("$line")
  done < <(read_search_names "$INPUT_FILE")

  local total=${#searches[@]}

  if [[ $total -eq 0 ]]; then
    echo -e "${YELLOW}Warning: No search names found in $INPUT_FILE${NC}"
    exit 0
  fi

  echo -e "${BLUE}Bulk Operation: ${OPERATION}${NC}"
  echo -e "${BLUE}Input file: ${INPUT_FILE}${NC}"
  echo -e "${BLUE}Total searches: ${total}${NC}"

  if [[ "$EXECUTE" == false ]]; then
    echo -e "${YELLOW}⚠ DRY-RUN MODE: No changes will be made${NC}"
    echo -e "${YELLOW}  Use --execute to apply changes${NC}"
  else
    if [[ "$OPERATION" == "delete-searches" ]]; then
      echo -e "${RED}⚠ WARNING: This will DELETE searches!${NC}"
    fi
    echo -e "${YELLOW}➜ EXECUTE MODE: Changes will be applied${NC}"
  fi

  echo ""

  local success=0
  local failed=0
  local skipped=0

  for i in "${!searches[@]}"; do
    local search_name="${searches[$i]}"
    local num=$((i + 1))

    if [[ "$SUMMARY_ONLY" == false ]]; then
      echo -ne "${BLUE}[${num}/${total}]${NC} ${search_name} ... "
    fi

    if [[ "$EXECUTE" == false ]]; then
      # Dry-run: just verify search exists
      if splunk-cli saved-searches list 2>/dev/null | grep -q "^${search_name}$"; then
        if [[ "$SUMMARY_ONLY" == false ]]; then
          echo -e "${GREEN}[would ${OPERATION%-searches}]${NC}"
        fi
        ((success++))
      else
        if [[ "$SUMMARY_ONLY" == false ]]; then
          echo -e "${YELLOW}[not found - would skip]${NC}"
        fi
        ((skipped++))
      fi
    else
      # Execute mode
      if perform_operation "$OPERATION" "$search_name"; then
        if [[ "$SUMMARY_ONLY" == false ]]; then
          echo -e "${GREEN}[done]${NC}"
        fi
        ((success++))
      else
        if [[ "$SUMMARY_ONLY" == false ]]; then
          echo -e "${RED}[failed]${NC}"
        fi
        ((failed++))
      fi
    fi
  done

  echo ""
  echo -e "${BLUE}Summary:${NC}"
  echo -e "  ${GREEN}Success:  ${success}${NC}"
  if [[ $skipped -gt 0 ]]; then
    echo -e "  ${YELLOW}Skipped:  ${skipped}${NC}"
  fi
  if [[ $failed -gt 0 ]]; then
    echo -e "  ${RED}Failed:   ${failed}${NC}"
  fi

  if [[ "$EXECUTE" == false ]]; then
    echo ""
    echo -e "${YELLOW}This was a dry-run. Use --execute to apply changes.${NC}"
  fi

  if [[ $failed -gt 0 ]]; then
    exit 1
  fi

  exit 0
}

main "$@"
