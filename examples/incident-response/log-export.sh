#!/usr/bin/env bash
# Export logs for incident evidence preservation
#
# RESPONSIBILITY:
#   Exports logs from multiple Splunk indexes to separate files for incident
#   evidence collection. Creates a metadata file documenting search parameters,
#   time ranges, and export details for chain of custody.
#
# DOES NOT:
#   - Modify or delete any data from Splunk
#   - Encrypt or password-protect exported files
#   - Automatically upload to external storage
#   - Perform data deduplication or filtering beyond time range
#
# PREREQUISITES:
#   - splunk-cli installed and in PATH
#   - SPLUNK_BASE_URL configured
#   - Authentication configured (SPLUNK_API_TOKEN or SPLUNK_USERNAME/PASSWORD)
#   - Sufficient disk space in output directory
#
# USAGE:
#   ./log-export.sh --earliest <time> --latest <time> [options]

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
DEFAULT_OUTPUT_DIR="./evidence-exports"
DEFAULT_INDEXES=("main" "security" "windows" "firewall" "proxy")
DEFAULT_EARLIEST="-24h"
DEFAULT_LATEST="now"

show_help() {
  cat << EOF
Log Export Tool for Incident Evidence

Usage: ${SCRIPT_NAME} --earliest <time> --latest <time> [OPTIONS]

Required:
  --earliest <time>    Start time (e.g., "-24h", "2024-01-01T00:00:00", "@d")
  --latest <time>      End time (e.g., "now", "2024-01-02T00:00:00", "@d")

Options:
  -o, --output-dir <dir>    Output directory (default: ${DEFAULT_OUTPUT_DIR})
  -i, --indexes <list>      Comma-separated list of indexes (default: ${DEFAULT_INDEXES[*]})
  -q, --query <search>      Additional search filter (e.g., "src_ip=10.0.0.1")
  -f, --format <format>     Output format: json, csv, raw (default: json)
  --metadata-only           Only create metadata file, skip data export
  --help                    Display this help message

Examples:
  ${SCRIPT_NAME} --earliest "-24h" --latest "now"
  ${SCRIPT_NAME} --earliest "2024-01-01T00:00:00" --latest "2024-01-02T00:00:00" -o /mnt/evidence/case-001
  ${SCRIPT_NAME} --earliest "-4h" --latest "now" -q "src_ip=192.168.1.100" -f csv

Time Formats:
  Relative:   -24h, -7d, -1w, @d (start of day), @w (start of week)
  Absolute:   2024-01-01T00:00:00, 2024-01-01

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

sanitize_filename() {
  local input="$1"
  # Replace problematic characters with underscores
  echo "$input" | sed 's/[^a-zA-Z0-9._-]/_/g'
}

generate_metadata() {
  local output_dir="$1"
  local earliest="$2"
  local latest="$3"
  local indexes="$4"
  local query="$5"
  local format="$6"

  local metadata_file="${output_dir}/METADATA.json"
  local timestamp
  timestamp=$(date -u '+%Y-%m-%dT%H:%M:%SZ' 2>/dev/null || date -u '+%Y-%m-%dT%H:%M:%SZ')
  local hostname
  hostname=$(hostname 2>/dev/null || echo "unknown")
  local username
  username=$(whoami 2>/dev/null || echo "unknown")

  cat > "$metadata_file" << EOF
{
  "export_metadata": {
    "version": "1.0",
    "created_at": "${timestamp}",
    "exported_by": "${username}@${hostname}",
    "tool": "${SCRIPT_NAME}",
    "splunk_base_url": "${SPLUNK_BASE_URL}"
  },
  "search_parameters": {
    "earliest": "${earliest}",
    "latest": "${latest}",
    "indexes": [$(echo "$indexes" | tr ',' '\n' | sed 's/^/"/;s/$/"/' | paste -sd ',' -)],
    "additional_query": "${query}",
    "output_format": "${format}"
  },
  "chain_of_custody": {
    "evidence_type": "Splunk Log Export",
    "acquisition_method": "splunk-cli search API",
    "integrity_note": "Verify file hashes after transfer"
  },
  "files_generated": [
$(for idx in $(echo "$indexes" | tr ',' ' '); do
  echo "    {\"index\": \"${idx}\", \"filename\": \"$(sanitize_filename "$idx").${format}\"},"
done | sed '$ s/,$//')
  ]
}
EOF

  echo -e "${GREEN}Created metadata file: ${metadata_file}${NC}"
}

export_index() {
  local index="$1"
  local earliest="$2"
  local latest="$3"
  local query="$4"
  local format="$5"
  local output_dir="$6"

  local filename
  filename="$(sanitize_filename "$index").${format}"
  local output_file="${output_dir}/${filename}"

  echo -e "${BLUE}Exporting index '${index}' to ${filename}...${NC}"

  # Build search query
  local search="search index=\"${index}\""
  if [[ -n "$query" ]]; then
    search="${search} ${query}"
  fi
  search="${search} earliest=${earliest} latest=${latest}"

  # Map format to splunk-cli format option
  local format_flag="json"
  case "$format" in
    csv)
      format_flag="csv"
      ;;
    raw)
      format_flag="raw"
      ;;
    json)
      format_flag="json"
      ;;
  esac

  if ! splunk-cli search "$search" --format "$format_flag" --output-file "$output_file" 2>/dev/null; then
    echo -e "${YELLOW}Warning: Failed to export index '${index}' or no results${NC}" >&2
    return 0
  fi

  # Get file size
  local file_size
  if [[ -f "$output_file" ]]; then
    file_size=$(du -h "$output_file" 2>/dev/null | cut -f1 || echo "unknown")
    echo -e "${GREEN}  ✓ Exported: ${filename} (${file_size})${NC}"
  else
    echo -e "${YELLOW}  Warning: Output file not created${NC}"
  fi
}

calculate_export_size() {
  local output_dir="$1"
  local total_size
  total_size=$(du -sh "$output_dir" 2>/dev/null | cut -f1 || echo "unknown")
  local file_count
  file_count=$(find "$output_dir" -type f 2>/dev/null | wc -l)

  echo ""
  echo -e "${BLUE}Export Summary:${NC}"
  echo "  Total files: ${file_count}"
  echo "  Total size: ${total_size}"
  echo "  Location: ${output_dir}"
}

main() {
  local earliest=""
  local latest=""
  local output_dir="$DEFAULT_OUTPUT_DIR"
  local indexes=""
  local query=""
  local format="json"
  local metadata_only=false

  # Parse arguments
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --help)
        show_help
        exit 0
        ;;
      --earliest)
        if [[ -z "${2:-}" ]]; then
          echo -e "${RED}Error: --earliest requires a value${NC}" >&2
          exit 1
        fi
        earliest="$2"
        shift 2
        ;;
      --latest)
        if [[ -z "${2:-}" ]]; then
          echo -e "${RED}Error: --latest requires a value${NC}" >&2
          exit 1
        fi
        latest="$2"
        shift 2
        ;;
      -o|--output-dir)
        if [[ -z "${2:-}" ]]; then
          echo -e "${RED}Error: --output-dir requires a value${NC}" >&2
          exit 1
        fi
        output_dir="$2"
        shift 2
        ;;
      -i|--indexes)
        if [[ -z "${2:-}" ]]; then
          echo -e "${RED}Error: --indexes requires a value${NC}" >&2
          exit 1
        fi
        indexes="$2"
        shift 2
        ;;
      -q|--query)
        if [[ -z "${2:-}" ]]; then
          echo -e "${RED}Error: --query requires a value${NC}" >&2
          exit 1
        fi
        query="$2"
        shift 2
        ;;
      -f|--format)
        if [[ -z "${2:-}" ]]; then
          echo -e "${RED}Error: --format requires a value${NC}" >&2
          exit 1
        fi
        format="$2"
        if [[ "$format" != "json" && "$format" != "csv" && "$format" != "raw" ]]; then
          echo -e "${RED}Error: Format must be json, csv, or raw${NC}" >&2
          exit 1
        fi
        shift 2
        ;;
      --metadata-only)
        metadata_only=true
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

  # Validate required arguments
  if [[ -z "$earliest" ]]; then
    echo -e "${RED}Error: --earliest is required${NC}" >&2
    show_help
    exit 1
  fi

  if [[ -z "$latest" ]]; then
    echo -e "${RED}Error: --latest is required${NC}" >&2
    show_help
    exit 1
  fi

  # Use default indexes if not specified
  if [[ -z "$indexes" ]]; then
    indexes=$(IFS=,; echo "${DEFAULT_INDEXES[*]}")
  fi

  check_prerequisites

  # Create output directory
  if [[ ! -d "$output_dir" ]]; then
    echo -e "${BLUE}Creating output directory: ${output_dir}${NC}"
    mkdir -p "$output_dir" || {
      echo -e "${RED}Error: Failed to create output directory${NC}" >&2
      exit 1
    }
  fi

  echo -e "${GREEN}Log Export Tool${NC}"
  echo -e "${BLUE}===============${NC}"
  echo ""
  echo "Time Range: ${earliest} to ${latest}"
  echo "Indexes: ${indexes}"
  echo "Output Directory: ${output_dir}"
  echo "Format: ${format}"
  if [[ -n "$query" ]]; then
    echo "Additional Query: ${query}"
  fi
  echo ""

  # Generate metadata
  generate_metadata "$output_dir" "$earliest" "$latest" "$indexes" "$query" "$format"
  echo ""

  # Exit if metadata-only mode
  if [[ "$metadata_only" == true ]]; then
    echo -e "${GREEN}Metadata file created. Skipping data export.${NC}"
    exit 0
  fi

  # Export each index
  local success_count=0
  local failed_count=0

  IFS=',' read -ra INDEX_ARRAY <<< "$indexes"
  for idx in "${INDEX_ARRAY[@]}"; do
    idx=$(echo "$idx" | xargs)  # trim whitespace
    if export_index "$idx" "$earliest" "$latest" "$query" "$format" "$output_dir"; then
      ((success_count++)) || true
    else
      ((failed_count++)) || true
    fi
  done

  echo ""
  echo -e "${GREEN}Export Complete${NC}"
  calculate_export_size "$output_dir"

  if [[ $failed_count -gt 0 ]]; then
    echo -e "${YELLOW}Warning: ${failed_count} index(es) failed to export${NC}" >&2
    exit 0
  fi

  exit 0
}

main "$@"
