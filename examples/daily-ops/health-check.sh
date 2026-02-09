#!/usr/bin/env bash
# Comprehensive Splunk server health check workflow
#
# RESPONSIBILITY:
#   Performs a comprehensive health check of Splunk server including
#   connectivity, licensing, KVStore status, cluster health, and recent errors.
#   Generates a summary report with status indicators for each component.
#
# DOES NOT:
#   - Modify any server configuration or data
#   - Perform remediation actions (read-only operations only)
#   - Check individual search job status (use job-cleanup.sh for that)
#   - Verify custom app health or deployment server status
#
# PREREQUISITES:
#   - splunk-cli installed and in PATH
#   - SPLUNK_BASE_URL configured
#   - Authentication configured (SPLUNK_API_TOKEN or SPLUNK_USERNAME/PASSWORD)
#
# USAGE:
#   ./health-check.sh [options]
#
# OPTIONS:
#   --json      Output results in JSON format
#   --help      Show this help message

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
JSON_OUTPUT=false

# =============================================================================
# Helper Functions
# =============================================================================

show_help() {
  cat <<EOF
Comprehensive Splunk Server Health Check

Usage: ${SCRIPT_NAME} [OPTIONS]

OPTIONS:
  --json      Output results in JSON format
  --help, -h  Show this help message

EXAMPLES:
  # Run interactive health check with formatted output
  ./${SCRIPT_NAME}

  # Output health check results as JSON for automation
  ./${SCRIPT_NAME} --json

ENVIRONMENT:
  SPLUNK_BASE_URL    Splunk REST API URL (required)
  SPLUNK_API_TOKEN   Splunk API token (preferred auth method)
  SPLUNK_USERNAME    Splunk username (alternative auth)
  SPLUNK_PASSWORD    Splunk password (alternative auth)
  NO_COLOR           Disable colored output when set

EXIT CODES:
  0   All health checks passed
  1   One or more health checks failed or prerequisites not met
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

print_header() {
  local title="$1"
  if [[ "$JSON_OUTPUT" == false ]]; then
    echo
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BLUE}  ${title}${NC}"
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  fi
}

print_status() {
  local status="$1"
  local message="$2"

  if [[ "$JSON_OUTPUT" == false ]]; then
    case "$status" in
      ok)
        echo -e "  ${GREEN}✓${NC} ${message}"
        ;;
      warn)
        echo -e "  ${YELLOW}⚠${NC} ${message}"
        ;;
      error)
        echo -e "  ${RED}✗${NC} ${message}"
        ;;
      info)
        echo -e "  ${BLUE}ℹ${NC} ${message}"
        ;;
    esac
  fi
}

# =============================================================================
# Health Check Functions
# =============================================================================

check_connectivity() {
  print_header "SERVER CONNECTIVITY"

  local status="ok"
  local message=""
  local details=""

  if splunk-cli doctor &>/dev/null; then
    message="Splunk server is reachable and responding"
    details="Connection to ${SPLUNK_BASE_URL} successful"
    print_status ok "$message"
  else
    status="error"
    message="Failed to connect to Splunk server"
    details="Unable to reach ${SPLUNK_BASE_URL}"
    print_status error "$message"
  fi

  if [[ "$JSON_OUTPUT" == true ]]; then
    echo "{\"connectivity\": {\"status\": \"$status\", \"message\": \"$message\", \"details\": \"$details\"}}"
  fi

  [[ "$status" == "error" ]] && return 1
  return 0
}

check_license() {
  print_header "LICENSE STATUS"

  local status="ok"
  local message=""
  local license_data=""

  if license_data=$(splunk-cli license --output json 2>/dev/null); then
    # Extract license usage (simplified parsing)
    local used_gb
    local quota_gb
    local usage_pct

    used_gb=$(echo "$license_data" | grep -oP '"used_bytes":[0-9]+' | head -1 | cut -d: -f2 || echo "0")
    quota_gb=$(echo "$license_data" | grep -oP '"quota_bytes":[0-9]+' | head -1 | cut -d: -f2 || echo "1")

    if [[ -n "$used_gb" && -n "$quota_gb" && "$quota_gb" -gt 0 ]]; then
      usage_pct=$((used_gb * 100 / quota_gb))

      if [[ $usage_pct -lt 70 ]]; then
        status="ok"
        message="License usage at ${usage_pct}% (${used_gb}GB / ${quota_gb}GB)"
        print_status ok "$message"
      elif [[ $usage_pct -lt 90 ]]; then
        status="warn"
        message="License usage elevated at ${usage_pct}% (${used_gb}GB / ${quota_gb}GB)"
        print_status warn "$message"
      else
        status="error"
        message="License usage critical at ${usage_pct}% (${used_gb}GB / ${quota_gb}GB)"
        print_status error "$message"
      fi
    else
      status="warn"
      message="License data retrieved but parsing failed"
      print_status warn "$message"
    fi
  else
    status="error"
    message="Failed to retrieve license information"
    print_status error "$message"
  fi

  if [[ "$JSON_OUTPUT" == true ]]; then
    echo "{\"license\": {\"status\": \"$status\", \"message\": \"$message\"}}"
  fi

  [[ "$status" == "error" ]] && return 1
  return 0
}

check_kvstore() {
  print_header "KVSTORE STATUS"

  local status="ok"
  local message=""

  if health_data=$(splunk-cli health --output json 2>/dev/null); then
    if echo "$health_data" | grep -q '"kvstore".*"healthy"'; then
      status="ok"
      message="KVStore is healthy and operational"
      print_status ok "$message"
    elif echo "$health_data" | grep -qi "kvstore"; then
      status="warn"
      message="KVStore status indeterminate or degraded"
      print_status warn "$message"
    else
      status="error"
      message="KVStore status not found in health check"
      print_status error "$message"
    fi
  else
    status="error"
    message="Failed to retrieve health status"
    print_status error "$message"
  fi

  if [[ "$JSON_OUTPUT" == true ]]; then
    echo "{\"kvstore\": {\"status\": \"$status\", \"message\": \"$message\"}}"
  fi

  [[ "$status" == "error" ]] && return 1
  return 0
}

check_cluster() {
  print_header "CLUSTER HEALTH"

  local status="ok"
  local message=""
  local cluster_data=""

  if cluster_data=$(splunk-cli cluster show --output json 2>/dev/null); then
    local peer_count
    peer_count=$(echo "$cluster_data" | grep -o '"peers":\[' | wc -l || echo "0")

    if echo "$cluster_data" | grep -q "cluster_manager"; then
      if [[ $peer_count -gt 0 ]]; then
        status="ok"
        message="Cluster manager responding with ${peer_count} peer(s)"
        print_status ok "$message"
      else
        status="warn"
        message="Cluster manager responding but no peers detected"
        print_status warn "$message"
      fi
    else
      status="info"
      message="Not a cluster member or standalone instance"
      print_status info "$message"
    fi
  else
    status="info"
    message="Cluster information unavailable (may be standalone)"
    print_status info "$message"
  fi

  if [[ "$JSON_OUTPUT" == true ]]; then
    echo "{\"cluster\": {\"status\": \"$status\", \"message\": \"$message\"}}"
  fi

  return 0
}

check_recent_errors() {
  print_header "RECENT ERRORS (Last 1 Hour)"

  local status="ok"
  local message=""
  local error_count=0
  local logs_data=""

  if logs_data=$(splunk-cli logs --search 'index=_internal sourcetype=splunkd log_level=ERROR' --last 1h --limit 10 --output json 2>/dev/null); then
    error_count=$(echo "$logs_data" | grep -c '"event"' || echo "0")

    if [[ $error_count -eq 0 ]]; then
      status="ok"
      message="No errors detected in the last hour"
      print_status ok "$message"
    elif [[ $error_count -lt 5 ]]; then
      status="warn"
      message="${error_count} error(s) detected in the last hour (review recommended)"
      print_status warn "$message"
    else
      status="error"
      message="${error_count} errors detected in the last hour (investigation required)"
      print_status error "$message"
    fi

    # Show sample errors if not in JSON mode
    if [[ "$JSON_OUTPUT" == false && $error_count -gt 0 ]]; then
      echo
      echo "  Sample recent errors:"
      echo "$logs_data" | grep -oP '"message":"[^"]+"' | head -3 | sed 's/"message":"/    - /' | sed 's/"$//' || true
    fi
  else
    status="warn"
    message="Unable to retrieve recent error logs"
    print_status warn "$message"
  fi

  if [[ "$JSON_OUTPUT" == true ]]; then
    echo "{\"recent_errors\": {\"status\": \"$status\", \"message\": \"$message\", \"count\": $error_count}}"
  fi

  [[ "$status" == "error" ]] && return 1
  return 0
}

# =============================================================================
# Main
# =============================================================================

main() {
  local overall_status=0

  # Parse arguments
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --json)
        JSON_OUTPUT=true
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

  if [[ "$JSON_OUTPUT" == false ]]; then
    echo -e "${BLUE}╔══════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${BLUE}║           Splunk Health Check Report                         ║${NC}"
    echo -e "${BLUE}╚══════════════════════════════════════════════════════════════╝${NC}"
    echo
    echo "Target: ${SPLUNK_BASE_URL}"
    echo "Time:   $(date '+%Y-%m-%d %H:%M:%S')"
  fi

  # Run health checks
  check_connectivity || overall_status=1
  check_license || overall_status=1
  check_kvstore || overall_status=1
  check_cluster || overall_status=1
  check_recent_errors || overall_status=1

  # Summary
  if [[ "$JSON_OUTPUT" == false ]]; then
    echo
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    if [[ $overall_status -eq 0 ]]; then
      echo -e "${GREEN}  ✓ All health checks passed${NC}"
    else
      echo -e "${YELLOW}  ⚠ One or more health checks reported issues${NC}"
    fi
    echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  fi

  exit $overall_status
}

main "$@"
