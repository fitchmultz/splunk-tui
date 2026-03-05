#!/usr/bin/env bash
# Cleanup old or failed Splunk search jobs
#
# RESPONSIBILITY:
#   Identifies and optionally removes stale search jobs based on age and status.
#   By default runs in dry-run mode to preview what would be cleaned. Supports
#   cleaning jobs older than a specified number of hours, with filtering by
#   job status (failed, done, running, etc.).
#
# DOES NOT:
#   - Delete jobs by default (requires explicit --execute flag)
#   - Cancel running jobs unless explicitly configured to do so
#   - Modify job results or saved searches
#   - Clean up index data or configuration
#
# PREREQUISITES:
#   - splunk-cli installed and in PATH
#   - SPLUNK_BASE_URL configured
#   - Authentication configured (SPLUNK_API_TOKEN or SPLUNK_USERNAME/PASSWORD)
#
# USAGE:
#   ./job-cleanup.sh [options]
#
# OPTIONS:
#   --dry-run         Preview what would be cleaned (default)
#   --execute         Actually perform cleanup (destructive)
#   --older-than N    Clean jobs older than N hours (default: 24)
#   --status FILTER   Filter by status: failed, done, running, all (default: all)
#   --force           Skip confirmation prompts (use with --execute)
#   --help            Show this help message

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
DRY_RUN=true
EXECUTE=false
OLDER_THAN_HOURS=24
STATUS_FILTER="all"
FORCE=false

# Data storage
declare -a jobs_to_clean=()
declare -a job_ids=()
declare -a job_names=()
declare -a job_statuses=()
declare -a job_ages=()

# =============================================================================
# Helper Functions
# =============================================================================

show_help() {
  cat <<EOF
Splunk Search Job Cleanup Tool

Usage: ${SCRIPT_NAME} [OPTIONS]

OPTIONS:
  --dry-run         Preview what would be cleaned (default, safe)
  --execute         Actually perform cleanup (DESTRUCTIVE - use with caution)
  --older-than N    Clean jobs older than N hours (default: 24)
  --status FILTER   Filter by status: failed, done, running, all (default: all)
  --force           Skip confirmation prompts (use with --execute)
  --help, -h        Show this help message

EXAMPLES:
  # Preview jobs older than 24 hours (dry run - safe)
  ./${SCRIPT_NAME}

  # Preview only failed jobs older than 48 hours
  ./${SCRIPT_NAME} --older-than 48 --status failed

  # Actually clean done jobs older than 72 hours
  ./${SCRIPT_NAME} --execute --older-than 72 --status done

  # Force cleanup without prompts (use with extreme caution)
  ./${SCRIPT_NAME} --execute --force --older-than 24

  # Preview all jobs older than 1 hour
  ./${SCRIPT_NAME} --older-than 1 --status all

ENVIRONMENT:
  SPLUNK_BASE_URL    Splunk REST API URL (required)
  SPLUNK_API_TOKEN   Splunk API token (preferred auth method)
  SPLUNK_USERNAME    Splunk username (alternative auth)
  SPLUNK_PASSWORD    Splunk password (alternative auth)
  NO_COLOR           Disable colored output when set

SAFETY:
  - Default mode is --dry-run which only previews changes
  - --execute flag required to actually delete jobs
  - Running jobs are listed but require explicit handling

EXIT CODES:
  0   Cleanup completed/previewed successfully
  1   Prerequisites not met or operation failed
  3   User cancelled operation
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

confirm_action() {
  local message="$1"

  if [[ "$FORCE" == true ]]; then
    return 0
  fi

  echo
  echo -ne "${YELLOW}${message} [y/N]: ${NC}"
  read -r response

  case "$response" in
    [yY]|[yY][eE][sS])
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

format_duration() {
  local minutes="$1"
  local hours=$((minutes / 60))
  local days=$((hours / 24))
  local remaining_hours=$((hours % 24))
  local remaining_minutes=$((minutes % 60))

  if [[ $days -gt 0 ]]; then
    echo "${days}d ${remaining_hours}h ${remaining_minutes}m"
  elif [[ $hours -gt 0 ]]; then
    echo "${hours}h ${remaining_minutes}m"
  else
    echo "${remaining_minutes}m"
  fi
}

# =============================================================================
# Job Discovery Functions
# =============================================================================

discover_jobs() {
  local jobs_data=""
  local cutoff_minutes=$((OLDER_THAN_HOURS * 60))

  echo -e "${BLUE}Fetching job list...${NC}"

  # Fetch all jobs
  if ! jobs_data=$(splunk-cli jobs --list --output json 2>/dev/null); then
    echo -e "${RED}ERROR:${NC} Failed to retrieve job list from Splunk" >&2
    exit 1
  fi

  # Validate data
  if [[ -z "$jobs_data" ]] || [[ "$jobs_data" == "[]" ]]; then
    echo -e "${YELLOW}No jobs found on the server.${NC}"
    exit 0
  fi

  # Parse jobs (simplified parsing)
  local job_entries
  job_entries=$(echo "$jobs_data" | grep -oP '"sid":"[^"]+"' | sed 's/"sid":"//' | sed 's/"$//' || true)

  if [[ -z "$job_entries" ]]; then
    echo -e "${YELLOW}No jobs found to process.${NC}"
    exit 0
  fi

  for sid in $job_entries; do
    # Skip if sid is just the word "sid" (parsing artifact)
    [[ "$sid" == "sid" ]] && continue

    # Extract job details from the JSON
    local job_block
    job_block=$(echo "$jobs_data" | grep -A30 "\"sid\":\"${sid}\"")

    local name
    local status
    local publish_time
    local runtime

    name=$(echo "$job_block" | grep -oP '"name":"[^"]+"' | head -1 | sed 's/"name":"//' | sed 's/"$//' || echo "unnamed")
    status=$(echo "$job_block" | grep -oP '"status":"[^"]+"' | head -1 | sed 's/"status":"//' | sed 's/"$//' || echo "unknown")
    publish_time=$(echo "$job_block" | grep -oP '"publish_time":[0-9]+' | head -1 | cut -d: -f2 || echo "0")
    runtime=$(echo "$job_block" | grep -oP '"runtime":[0-9]+' | head -1 | cut -d: -f2 || echo "0")

    # Calculate age in minutes (simplified - using runtime as proxy)
    local age_minutes=${runtime:-0}
    [[ -z "$age_minutes" ]] && age_minutes=0

    # Apply status filter
    if [[ "$STATUS_FILTER" != "all" ]]; then
      if [[ "$status" != "$STATUS_FILTER" ]]; then
        continue
      fi
    fi

    # Check age threshold
    if [[ $age_minutes -ge $cutoff_minutes ]]; then
      job_ids+=("$sid")
      job_names+=("$name")
      job_statuses+=("$status")
      job_ages+=("$age_minutes")
    fi
  done
}

# =============================================================================
# Report and Cleanup Functions
# =============================================================================

print_header() {
  echo
  echo -e "${BLUE}╔════════════════════════════════════════════════════════════════╗${NC}"
  echo -e "${BLUE}║${BOLD}           Splunk Job Cleanup Tool                              ${NC}${BLUE}║${NC}"
  echo -e "${BLUE}╚════════════════════════════════════════════════════════════════╝${NC}"
  echo
  echo -e "  Server:        ${CYAN}${SPLUNK_BASE_URL}${NC}"
  echo -e "  Mode:          $([[ "$DRY_RUN" == true ]] && echo -e "${GREEN}DRY RUN (safe)${NC}" || echo -e "${RED}EXECUTE (destructive)${NC}")"
  echo -e "  Older than:    ${OLDER_THAN_HOURS} hours"
  echo -e "  Status filter: ${STATUS_FILTER}"
  echo -e "  Time:          $(date '+%Y-%m-%d %H:%M:%S')"
}

print_job_list() {
  local total=${#job_ids[@]}

  echo
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

  if [[ $total -eq 0 ]]; then
    echo -e "${GREEN}No jobs found matching the cleanup criteria.${NC}"
    echo
    echo -e "  Criteria: Jobs older than ${OLDER_THAN_HOURS} hours"
    [[ "$STATUS_FILTER" != "all" ]] && echo -e "            Status: ${STATUS_FILTER}"
    return
  fi

  echo -e "${YELLOW}Found ${total} job(s) matching cleanup criteria:${NC}"
  echo
  printf "  %-24s %-15s %-10s %s\n" "JOB ID" "AGE" "STATUS" "NAME"
  printf "  %-24s %-15s %-10s %s\n" "------------------------" "---------------" "----------" "--------------------"

  for ((i=0; i<total; i++)); do
    local sid="${job_ids[$i]}"
    local age_formatted
    age_formatted=$(format_duration "${job_ages[$i]}")
    local status="${job_statuses[$i]}"
    local name="${job_names[$i]}"

    # Color code status
    local status_color="$NC"
    case "$status" in
      failed)
        status_color="$RED"
        ;;
      done)
        status_color="$GREEN"
        ;;
      running)
        status_color="$YELLOW"
        ;;
    esac

    printf "  %-24s %-15s ${status_color}%-10s${NC} %s\n" "${sid:0:24}" "$age_formatted" "$status" "${name:0:30}"
  done
}

execute_cleanup() {
  local total=${#job_ids[@]}
  local success=0
  local failed=0

  if [[ $total -eq 0 ]]; then
    echo
    echo -e "${GREEN}No jobs to clean up.${NC}"
    return 0
  fi

  echo
  echo -e "${RED}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo -e "${BOLD}${RED}EXECUTING CLEANUP${NC}"
  echo -e "${RED}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo

  for ((i=0; i<total; i++)); do
    local sid="${job_ids[$i]}"
    local name="${job_names[$i]}"

    echo -ne "  Cleaning job ${sid} (${name:0:20})... "

    if splunk-cli jobs --delete "$sid" &>/dev/null; then
      echo -e "${GREEN}✓${NC}"
      ((success++))
    else
      echo -e "${RED}✗${NC}"
      ((failed++))
    fi
  done

  echo
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo -e "${BOLD}CLEANUP SUMMARY${NC}"
  echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo
  echo -e "  Total processed: ${total}"
  echo -e "  ${GREEN}Successful:      ${success}${NC}"
  [[ $failed -gt 0 ]] && echo -e "  ${RED}Failed:          ${failed}${NC}"

  [[ $failed -gt 0 ]] && return 1
  return 0
}

# =============================================================================
# Main
# =============================================================================

main() {
  # Parse arguments
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --dry-run)
        DRY_RUN=true
        EXECUTE=false
        shift
        ;;
      --execute)
        DRY_RUN=false
        EXECUTE=true
        shift
        ;;
      --older-than)
        if [[ -z "${2:-}" || "$2" =~ ^- ]]; then
          echo -e "${RED}ERROR:${NC} --older-than requires a value" >&2
          exit 1
        fi
        if ! [[ "$2" =~ ^[0-9]+$ ]] || [[ "$2" -lt 1 ]]; then
          echo -e "${RED}ERROR:${NC} --older-than must be a positive integer (hours)" >&2
          exit 1
        fi
        OLDER_THAN_HOURS="$2"
        shift 2
        ;;
      --status)
        if [[ -z "${2:-}" || "$2" =~ ^- ]]; then
          echo -e "${RED}ERROR:${NC} --status requires a value" >&2
          exit 1
        fi
        if [[ ! "$2" =~ ^(failed|done|running|all)$ ]]; then
          echo -e "${RED}ERROR:${NC} --status must be one of: failed, done, running, all" >&2
          exit 1
        fi
        STATUS_FILTER="$2"
        shift 2
        ;;
      --force)
        FORCE=true
        shift
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

  # Print header
  print_header

  # Discover jobs to clean
  discover_jobs

  # Print job list
  print_job_list

  # Execute or preview
  if [[ "$EXECUTE" == true ]]; then
    local total=${#job_ids[@]}

    if [[ $total -eq 0 ]]; then
      echo
      echo -e "${GREEN}No cleanup actions needed.${NC}"
      exit 0
    fi

    echo
    echo -e "${RED}⚠ WARNING: This will PERMANENTLY DELETE ${total} job(s)!${NC}"

    if ! confirm_action "Are you sure you want to proceed"; then
      echo
      echo -e "${YELLOW}Operation cancelled by user.${NC}"
      exit 3
    fi

    if execute_cleanup; then
      echo
      echo -e "${GREEN}✓ Cleanup completed successfully${NC}"
      exit 0
    else
      echo
      echo -e "${YELLOW}⚠ Cleanup completed with some failures${NC}"
      exit 1
    fi
  else
    # Dry run mode
    echo
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${GREEN}DRY RUN MODE${NC} - No jobs were actually deleted"
    echo -e "Run with ${BOLD}--execute${NC} to perform the cleanup"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    exit 0
  fi
}

main "$@"
