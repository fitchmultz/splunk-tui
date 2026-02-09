#!/usr/bin/env bash
# Automate data onboarding workflow for Splunk
#
# RESPONSIBILITY:
#   Automates the complete data onboarding workflow:
#   1. Creates index if it doesn't exist
#   2. Configures HEC input if requested
#   3. Validates data ingestion with a test event
#   4. Creates saved searches for monitoring
#
# DOES NOT:
#   - Install or configure Splunk forwarders
#   - Set up complex data parsing or field extractions
#   - Handle SSL certificate configuration for HEC
#   - Replace manual data source configuration on forwarders
#
# PREREQUISITES:
#   - splunk-cli installed and in PATH
#   - SPLUNK_BASE_URL configured
#   - Authentication configured (SPLUNK_API_TOKEN or SPLUNK_USERNAME/PASSWORD)
#   - User must have admin privileges for index creation
#
# USAGE:
#   ./data-onboarding.sh [options]

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
INDEX_NAME=""
SOURCETYPE=""
ENABLE_HEC=false
HEC_TOKEN=""
CREATE_MONITORING=true
SKIP_VALIDATION=false

# Show help information
show_help() {
  cat << 'EOF'
Usage: ./data-onboarding.sh [OPTIONS]

Automate data onboarding workflow for Splunk.

OPTIONS:
    --index <name>        Index name to create (required)
    --sourcetype <type>   Sourcetype for the data (required)
    --hec                 Enable HEC (HTTP Event Collector) input
    --hec-token <token>   Use existing HEC token (skips HEC creation)
    --no-monitoring       Skip creation of monitoring saved searches
    --skip-validation     Skip test event validation
    -h, --help            Show this help message

EXAMPLES:
    # Basic onboarding with index and sourcetype
    ./data-onboarding.sh --index "my_app_logs" --sourcetype "my:application"

    # Enable HEC for HTTP ingestion
    ./data-onboarding.sh --index "web_logs" --sourcetype "access:combined" --hec

    # Use existing HEC token
    ./data-onboarding.sh --index "api_events" --sourcetype "json" --hec --hec-token "abc123"

WORKFLOW STEPS:
    1. Creates the specified index if it doesn't exist
    2. Configures HEC input if --hec is specified
    3. Sends a test event to validate ingestion
    4. Creates saved searches for monitoring data flow

POST-ONBOARDING:
    After running this script, configure your data source to send to:
    - HEC endpoint: https://<splunk-host>:8088/services/collector/event
    - Index: <the index you specified>
    - Sourcetype: <the sourcetype you specified>

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
      --index)
        INDEX_NAME="$2"
        shift 2
        ;;
      --sourcetype)
        SOURCETYPE="$2"
        shift 2
        ;;
      --hec)
        ENABLE_HEC=true
        shift
        ;;
      --hec-token)
        HEC_TOKEN="$2"
        shift 2
        ;;
      --no-monitoring)
        CREATE_MONITORING=false
        shift
        ;;
      --skip-validation)
        SKIP_VALIDATION=true
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

  if [[ -z "$INDEX_NAME" ]]; then
    echo -e "${RED}Error: --index is required${NC}" >&2
    show_help
    exit 1
  fi

  if [[ -z "$SOURCETYPE" ]]; then
    echo -e "${RED}Error: --sourcetype is required${NC}" >&2
    show_help
    exit 1
  fi
}

# Step 1: Create index if it doesn't exist
step_create_index() {
  echo -e "${BLUE}[Step 1/4] Checking/Creating index: ${INDEX_NAME}${NC}"

  # Check if index exists
  if splunk-cli indexes list 2>/dev/null | grep -q "^${INDEX_NAME}$"; then
    echo -e "  ${GREEN}✓ Index '${INDEX_NAME}' already exists${NC}"
    return 0
  fi

  echo -e "  ${BLUE}Creating index '${INDEX_NAME}'...${NC}"
  if splunk-cli indexes create "$INDEX_NAME" 2>/dev/null; then
    echo -e "  ${GREEN}✓ Index created successfully${NC}"
    return 0
  else
    echo -e "  ${RED}✗ Failed to create index${NC}" >&2
    return 1
  fi
}

# Step 2: Configure HEC input if requested
step_configure_hec() {
  if [[ "$ENABLE_HEC" == false ]]; then
    echo -e "${BLUE}[Step 2/4] HEC configuration skipped (--hec not specified)${NC}"
    return 0
  fi

  echo -e "${BLUE}[Step 2/4] Configuring HEC input${NC}"

  # Use existing token if provided
  if [[ -n "$HEC_TOKEN" ]]; then
    echo -e "  ${GREEN}✓ Using provided HEC token${NC}"
    echo -e "  ${BLUE}  Token: ${HEC_TOKEN:0:8}...${NC}"
    return 0
  fi

  # Check if HEC is available
  if ! splunk-cli hec list 2>/dev/null &>/dev/null; then
    echo -e "  ${YELLOW}⚠ HEC management not available via CLI${NC}"
    echo -e "  ${YELLOW}  Configure HEC manually in Splunk Web${NC}"
    return 0
  fi

  # Create HEC token for this index
  local token_name="${INDEX_NAME}_hec_token"
  echo -e "  ${BLUE}Creating HEC token: ${token_name}${NC}"

  # Note: HEC token creation syntax may vary based on CLI implementation
  local token
  token=$(splunk-cli hec create "$token_name" --index "$INDEX_NAME" --sourcetype "$SOURCETYPE" 2>/dev/null || echo "")

  if [[ -n "$token" ]]; then
    HEC_TOKEN="$token"
    echo -e "  ${GREEN}✓ HEC token created${NC}"
    echo -e "  ${GREEN}  Token: ${HEC_TOKEN:0:8}...${NC}"
  else
    echo -e "  ${YELLOW}⚠ Could not create HEC token automatically${NC}"
    echo -e "  ${YELLOW}  Configure HEC manually in Splunk Web${NC}"
  fi
}

# Step 3: Validate data ingestion with test event
step_validate_ingestion() {
  if [[ "$SKIP_VALIDATION" == true ]]; then
    echo -e "${BLUE}[Step 3/4] Validation skipped (--skip-validation)${NC}"
    return 0
  fi

  echo -e "${BLUE}[Step 3/4] Validating data ingestion${NC}"

  local test_event
  test_event="{\"event\": \"onboarding_test\", \"index\": \"${INDEX_NAME}\", \"sourcetype\": \"${SOURCETYPE}\", \"message\": \"Test event for data onboarding validation\", \"timestamp\": \"$(date -Iseconds)\"}"

  if [[ -n "$HEC_TOKEN" ]]; then
    # Send test event via HEC
    local hec_url="${SPLUNK_BASE_URL}/services/collector/event"
    # Remove the port 8089 if present for HEC default port 8088
    hec_url="${hec_url/:8089/:8088}"

    echo -e "  ${BLUE}Sending test event via HEC...${NC}"
    if curl -s -k "${hec_url}" \
      -H "Authorization: Splunk ${HEC_TOKEN}" \
      -d "${test_event}" > /dev/null 2>&1; then
      echo -e "  ${GREEN}✓ Test event sent${NC}"
    else
      echo -e "  ${YELLOW}⚠ Could not send test event via HEC${NC}"
    fi
  else
    echo -e "  ${YELLOW}⚠ No HEC token available, skipping HEC test${NC}"
  fi

  # Wait a moment for indexing
  echo -e "  ${BLUE}Waiting for indexing...${NC}"
  sleep 3

  # Search for test event
  echo -e "  ${BLUE}Searching for test event...${NC}"
  local search_query="index=\"${INDEX_NAME}\" sourcetype=\"${SOURCETYPE}\" onboarding_test"
  local results
  results=$(splunk-cli search "$search_query" --limit 1 2>/dev/null || echo "")

  if [[ -n "$results" && "$results" != "[]" ]]; then
    echo -e "  ${GREEN}✓ Test event found in index${NC}"
  else
    echo -e "  ${YELLOW}⚠ Test event not yet indexed (may take time)${NC}"
    echo -e "  ${BLUE}  You can verify later with:${NC}"
    echo -e "  ${BLUE}  splunk-cli search 'index=\"${INDEX_NAME}\"'${NC}"
  fi
}

# Step 4: Create saved searches for monitoring
step_create_monitoring() {
  if [[ "$CREATE_MONITORING" == false ]]; then
    echo -e "${BLUE}[Step 4/4] Monitoring searches skipped (--no-monitoring)${NC}"
    return 0
  fi

  echo -e "${BLUE}[Step 4/4] Creating monitoring saved searches${NC}"

  local search_name_prefix="${INDEX_NAME}_monitor"
  local created=0

  # Create data volume monitoring search
  local volume_search="${search_name_prefix}_volume"
  local volume_query="index=\"${INDEX_NAME}\" | stats count by sourcetype | eval status=if(count>0, \"active\", \"inactive\")"

  echo -e "  ${BLUE}Creating: ${volume_search}${NC}"
  if splunk-cli saved-searches create "$volume_search" --search "$volume_query" --description "Monitor data volume for ${INDEX_NAME}" 2>/dev/null; then
    echo -e "    ${GREEN}✓ Volume monitor created${NC}"
    ((created++))
  else
    echo -e "    ${YELLOW}⚠ Could not create volume monitor${NC}"
  fi

  # Create error monitoring search
  local error_search="${search_name_prefix}_errors"
  local error_query="index=\"${INDEX_NAME}\" (error OR fail* OR exception) | stats count | where count > 0"

  echo -e "  ${BLUE}Creating: ${error_search}${NC}"
  if splunk-cli saved-searches create "$error_search" --search "$error_query" --description "Monitor errors in ${INDEX_NAME}" 2>/dev/null; then
    echo -e "    ${GREEN}✓ Error monitor created${NC}"
    ((created++))
  else
    echo -e "    ${YELLOW}⚠ Could not create error monitor${NC}"
  fi

  echo -e "  ${GREEN}✓ Created ${created} monitoring searches${NC}"
}

# Print summary
print_summary() {
  echo ""
  echo -e "${GREEN}╔══════════════════════════════════════════════════════════╗${NC}"
  echo -e "${GREEN}║           Data Onboarding Complete                       ║${NC}"
  echo -e "${GREEN}╚══════════════════════════════════════════════════════════╝${NC}"
  echo ""
  echo -e "${BLUE}Index:${NC}        ${INDEX_NAME}"
  echo -e "${BLUE}Sourcetype:${NC}   ${SOURCETYPE}"
  if [[ "$ENABLE_HEC" == true ]]; then
    echo -e "${BLUE}HEC Enabled:${NC}  Yes"
    if [[ -n "$HEC_TOKEN" ]]; then
      echo -e "${BLUE}HEC Token:${NC}    ${HEC_TOKEN:0:8}..."
    fi
  fi
  echo ""
  echo -e "${BLUE}Next steps:${NC}"
  echo "  1. Configure your data source to send to Splunk"
  if [[ "$ENABLE_HEC" == true ]]; then
    echo "     HEC URL: ${SPLUNK_BASE_URL/:8089/:8088}/services/collector/event"
    echo "     Headers: Authorization: Splunk <your-token>"
    echo "     Payload: {\"event\": \"...\", \"index\": \"${INDEX_NAME}\", \"sourcetype\": \"${SOURCETYPE}\"}"
  fi
  echo "  2. Verify data ingestion: splunk-cli search 'index=\"${INDEX_NAME}\"'"
  echo "  3. Check monitoring searches: splunk-cli saved-searches list"
  echo ""
}

# Main execution
main() {
  parse_args "$@"
  check_prerequisites

  echo -e "${BLUE}Starting data onboarding workflow${NC}"
  echo -e "${BLUE}=================================${NC}"
  echo ""

  # Run all steps
  if ! step_create_index; then
    echo -e "${RED}Workflow failed at Step 1${NC}" >&2
    exit 1
  fi

  step_configure_hec
  step_validate_ingestion
  step_create_monitoring

  print_summary

  exit 0
}

main "$@"
