#!/usr/bin/env bash
# Rapid incident investigation search for IOCs across multiple data sources
#
# RESPONSIBILITY:
#   Searches for Indicators of Compromise (IOCs) such as IP addresses, file hashes,
#   or usernames across authentication logs, network traffic, endpoint events, and
#   proxy logs. Provides rapid triage capabilities during security incidents.
#
# DOES NOT:
#   - Perform real-time monitoring or continuous detection
#   - Modify or delete any Splunk data
#   - Correlate events across time windows automatically
#   - Export data to external systems
#
# PREREQUISITES:
#   - splunk-cli installed and in PATH
#   - SPLUNK_BASE_URL configured
#   - Authentication configured (SPLUNK_API_TOKEN or SPLUNK_USERNAME/PASSWORD)
#
# USAGE:
#   ./rapid-search.sh <IOC> [options]

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
DEFAULT_HOURS=24
MAX_HOURS=168  # 7 days

# Output modes
OUTPUT_MODE="table"

show_help() {
  cat << EOF
Rapid IOC Investigation Search

Usage: ${SCRIPT_NAME} <IOC> [OPTIONS]

Arguments:
  IOC           The Indicator of Compromise to search for (IP, hash, username)

Options:
  -h, --hours N     Time range in hours (default: ${DEFAULT_HOURS}, max: ${MAX_HOURS})
  -o, --output      Output format: json or table (default: table)
  --help            Display this help message

Examples:
  ${SCRIPT_NAME} 192.168.1.100
  ${SCRIPT_NAME} "user@domain.com" --hours 48
  ${SCRIPT_NAME} "a1b2c3d4e5f6..." --hours 72 --output json | jq '.[] | select(.index=="windows")'

Environment:
  SPLUNK_BASE_URL     Splunk REST API URL
  SPLUNK_API_TOKEN    API token for authentication
  SPLUNK_USERNAME     Username for authentication (if not using token)
  SPLUNK_PASSWORD     Password for authentication (if not using token)
  NO_COLOR            Disable colored output

EOF
}

check_prerequisites() {
  local missing=()

  if ! command -v splunk-cli &> /dev/null; then
    missing+=("splunk-cli")
  fi

  if [[ ${#missing[@]} -gt 0 ]]; then
    echo -e "${RED}Error: Missing required tools: ${missing[*]}${NC}" >&2
    echo "Please install splunk-cli and ensure it's in your PATH" >&2
    exit 1
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

detect_ioc_type() {
  local ioc="$1"

  # IP address detection (IPv4 and IPv6)
  if [[ "$ioc" =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "ip"
  elif [[ "$ioc" =~ ^[0-9a-fA-F]{32}$ ]]; then
    echo "md5"
  elif [[ "$ioc" =~ ^[0-9a-fA-F]{40}$ ]]; then
    echo "sha1"
  elif [[ "$ioc" =~ ^[0-9a-fA-F]{64}$ ]]; then
    echo "sha256"
  elif [[ "$ioc" =~ ^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$ ]]; then
    echo "guid"
  elif [[ "$ioc" =~ @ ]]; then
    echo "email"
  else
    echo "username"
  fi
}

build_searches() {
  local ioc="$1"
  local ioc_type="$2"
  local hours="$3"
  local earliest="-${hours}h"

  declare -A searches

  case "$ioc_type" in
    ip)
      searches[auth]="search index=* (src_ip=\"${ioc}\" OR dest_ip=\"${ioc}\" OR src=\"${ioc}\" OR dest=\"${ioc}\") earliest=${earliest} | head 100 | table _time, index, sourcetype, src_ip, dest_ip, action, user"
      searches[network]="search index=* (src_ip=\"${ioc}\" OR dest_ip=\"${ioc}\") earliest=${earliest} | head 100 | table _time, index, sourcetype, src_ip, dest_ip, src_port, dest_port, bytes_in, bytes_out"
      searches[proxy]="search index=proxy OR index=web OR sourcetype=*proxy* (src_ip=\"${ioc}\" OR dest_ip=\"${ioc}\") earliest=${earliest} | head 100 | table _time, index, sourcetype, src_ip, url, action, status"
      searches[endpoint]="search index=endpoint OR index=os OR sourcetype=*sysmon* (src_ip=\"${ioc}\" OR dest_ip=\"${ioc}\" OR ip=\"${ioc}\") earliest=${earliest} | head 100 | table _time, index, sourcetype, host, process_name, user"
      ;;
    md5|sha1|sha256)
      searches[endpoint]="search index=* (file_hash=\"${ioc}\" OR hash=\"${ioc}\" OR md5=\"${ioc}\" OR sha256=\"${ioc}\") earliest=${earliest} | head 100 | table _time, index, sourcetype, host, file_name, file_path, user, action"
      ;;
    username|email)
      searches[auth]="search index=* user=\"${ioc}\" earliest=${earliest} | head 100 | table _time, index, sourcetype, user, src_ip, action, status"
      searches[endpoint]="search index=* (user=\"${ioc}\" OR username=\"${ioc}\") earliest=${earliest} | head 100 | table _time, index, sourcetype, host, user, process_name, command_line"
      ;;
    *)
      # Generic search for unknown types
      searches[generic]="search index=\"${ioc}\" OR \"${ioc}\" earliest=${earliest} | head 100 | table _time, index, sourcetype, host, _raw"
      ;;
  esac

  for key in "${!searches[@]}"; do
    echo "${key}:${searches[$key]}"
  done
}

run_search() {
  local name="$1"
  local query="$2"
  local output_format="$3"

  echo -e "${BLUE}=== Searching ${name} ===${NC}" >&2

  local format_flag="table"
  if [[ "$output_format" == "json" ]]; then
    format_flag="json"
  fi

  if ! splunk-cli search "$query" --format "$format_flag" 2>/dev/null; then
    echo -e "${YELLOW}Warning: No results or error in ${name} search${NC}" >&2
    return 0
  fi
}

main() {
  local ioc=""
  local hours="$DEFAULT_HOURS"

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
      -o|--output)
        if [[ -z "${2:-}" ]]; then
          echo -e "${RED}Error: --output requires a value${NC}" >&2
          exit 1
        fi
        OUTPUT_MODE="$2"
        if [[ "$OUTPUT_MODE" != "json" && "$OUTPUT_MODE" != "table" ]]; then
          echo -e "${RED}Error: Output must be 'json' or 'table'${NC}" >&2
          exit 1
        fi
        shift 2
        ;;
      -*)
        echo -e "${RED}Error: Unknown option: $1${NC}" >&2
        show_help
        exit 1
        ;;
      *)
        if [[ -z "$ioc" ]]; then
          ioc="$1"
        else
          echo -e "${RED}Error: Multiple IOCs specified${NC}" >&2
          exit 1
        fi
        shift
        ;;
    esac
  done

  if [[ -z "$ioc" ]]; then
    echo -e "${RED}Error: IOC is required${NC}" >&2
    show_help
    exit 1
  fi

  # Validate hours
  if ! [[ "$hours" =~ ^[0-9]+$ ]]; then
    echo -e "${RED}Error: Hours must be a positive integer${NC}" >&2
    exit 1
  fi

  if [[ "$hours" -gt "$MAX_HOURS" ]]; then
    echo -e "${YELLOW}Warning: Hours capped at ${MAX_HOURS} (7 days)${NC}" >&2
    hours="$MAX_HOURS"
  fi

  check_prerequisites

  local ioc_type
  ioc_type=$(detect_ioc_type "$ioc")
  echo -e "${GREEN}Detected IOC type: ${ioc_type}${NC}" >&2
  echo -e "${GREEN}Searching for: ${ioc} (last ${hours} hours)${NC}" >&2
  echo "" >&2

  # Build and execute searches
  local results=0
  while IFS=: read -r name query; do
    if [[ -n "$name" && -n "$query" ]]; then
      if run_search "$name" "$query" "$OUTPUT_MODE"; then
        ((results++)) || true
      fi
      echo "" >&2
    fi
  done < <(build_searches "$ioc" "$ioc_type" "$hours")

  if [[ $results -eq 0 ]]; then
    echo -e "${YELLOW}No searches returned results${NC}" >&2
    exit 0
  fi

  echo -e "${GREEN}Search completed. Review results above.${NC}" >&2
  exit 0
}

main "$@"
