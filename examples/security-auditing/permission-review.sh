#!/usr/bin/env bash
# Review user permissions and roles for security audit
#
# RESPONSIBILITY:
#   Generates comprehensive reports on user access including user lists with
#   roles, role capabilities, identification of admin-privileged users, and
#   detection of potentially inactive accounts (no login in 90 days).
#
# DOES NOT:
#   - Modify any user or role settings
#   - Disable or delete inactive accounts
#   - Compare against external identity providers
#   - Export sensitive data to external systems
#
# PREREQUISITES:
#   - splunk-cli installed and in PATH
#   - SPLUNK_BASE_URL configured
#   - Authentication configured (SPLUNK_API_TOKEN or SPLUNK_USERNAME/PASSWORD)
#
# USAGE:
#   ./permission-review.sh [options]

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

# Configuration
INACTIVE_DAYS=90
ADMIN_ROLES=("admin" "sc_admin" "power")

# Output formatting
info() { echo -e "${BLUE}[INFO]${NC} $1"; }
success() { echo -e "${GREEN}[OK]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; }

show_help() {
  cat << 'EOF'
Review user permissions and roles for security audit

USAGE:
  ./permission-review.sh [OPTIONS]

OPTIONS:
  -o, --output <file>      Save report to file
  --inactive-days <days>   Days since last login to flag as inactive (default: 90)
  --admin-roles <roles>    Comma-separated list of admin role names
  --include-capabilities   Include full capability lists for each role
  --no-color               Disable colored output
  -h, --help               Show this help message

EXAMPLES:
  ./permission-review.sh
  ./permission-review.sh --inactive-days 30 --output audit.txt
  ./permission-review.sh --admin-roles "admin,super_admin"

EXIT CODES:
  0  Success
  1  Prerequisites not met or command failed
  2  No data retrieved from Splunk
EOF
}

check_prerequisites() {
  local missing=()

  if ! command -v splunk-cli &> /dev/null; then
    missing+=("splunk-cli")
  fi

  if ! command -v jq &> /dev/null; then
    missing+=("jq")
  fi

  if [[ ${#missing[@]} -gt 0 ]]; then
    error "Failed to find required tools: ${missing[*]}"
    echo "Please install the missing prerequisites."
    exit 1
  fi

  if [[ -z "${SPLUNK_BASE_URL:-}" ]]; then
    error "SPLUNK_BASE_URL is not configured"
    echo "Set SPLUNK_BASE_URL environment variable"
    exit 1
  fi

  if [[ -z "${SPLUNK_API_TOKEN:-}" && ( -z "${SPLUNK_USERNAME:-}" || -z "${SPLUNK_PASSWORD:-}" ) ]]; then
    error "Authentication not configured"
    echo "Set SPLUNK_API_TOKEN or both SPLUNK_USERNAME and SPLUNK_PASSWORD"
    exit 1
  fi

  success "Prerequisites verified"
}

fetch_users() {
  info "Fetching user list..."

  local users
  if ! users=$(splunk-cli users list --output json --quiet 2>/dev/null); then
    error "Failed to fetch user list"
    return 1
  fi

  if [[ -z "$users" || "$users" == "[]" || "$users" == "null" ]]; then
    warn "No users found"
    return 2
  fi

  echo "$users"
}

fetch_roles() {
  info "Fetching role definitions..."

  local roles
  if ! roles=$(splunk-cli roles list --output json --quiet 2>/dev/null); then
    error "Failed to fetch role list"
    return 1
  fi

  if [[ -z "$roles" || "$roles" == "[]" || "$roles" == "null" ]]; then
    warn "No roles found"
    return 2
  fi

  echo "$roles"
}

fetch_capabilities() {
  info "Fetching available capabilities..."

  local caps
  if ! caps=$(splunk-cli roles capabilities --output json --quiet 2>/dev/null); then
    error "Failed to fetch capabilities"
    return 1
  fi

  echo "$caps"
}

analyze_user_roles() {
  local users="$1"

  echo ""
  echo "=== User Role Assignments ==="
  echo ""

  echo "$users" | jq -r '.[] | "User: \(.name)\n  Roles: \(.roles | join(", "))\n  Real Name: \(.realname // "N/A")\n  Email: \(.email // "N/A")\n"' 2>/dev/null || {
    echo "$users"
    return 1
  }

  local total_users
  total_users=$(echo "$users" | jq 'length' 2>/dev/null || echo "0")
  success "Total users: $total_users"
  echo ""
}

identify_admin_users() {
  local users="$1"

  echo "=== Users with Admin Privileges ==="
  echo ""

  local admin_found=false

  for admin_role in "${ADMIN_ROLES[@]}"; do
    local users_with_role
    users_with_role=$(echo "$users" | jq --arg role "$admin_role" '[.[] | select(.roles | contains([$role]))]')

    local count
    count=$(echo "$users_with_role" | jq 'length' 2>/dev/null || echo "0")

    if [[ "$count" -gt 0 ]]; then
      admin_found=true
      warn "Role '$admin_role' ($count user(s)):"
      echo "$users_with_role" | jq -r '.[] | "  - \(.name) (\(.email // "no email"))"' 2>/dev/null
      echo ""
    fi
  done

  if [[ "$admin_found" == false ]]; then
    echo "No users found with admin roles: ${ADMIN_ROLES[*]}"
  fi

  # Check for users with multiple admin roles (privilege escalation concern)
  echo "=== Users with Multiple Admin Roles ==="
  local multi_admin
  multi_admin=$(echo "$users" | jq --argjson roles "$(printf '%s\n' "${ADMIN_ROLES[@]}" | jq -R . | jq -s .)" '
    [.[] | select(.roles | map(select(. as $r | $roles | contains([$r]))) | length > 1)]
  ')

  local multi_count
  multi_count=$(echo "$multi_admin" | jq 'length' 2>/dev/null || echo "0")

  if [[ "$multi_count" -gt 0 ]]; then
    warn "Found $multi_count user(s) with multiple admin roles:"
    echo "$multi_admin" | jq -r '.[] | "  - \(.name): \(.roles | join(", "))"' 2>/dev/null
  else
    success "No users with multiple admin roles"
  fi
  echo ""
}

analyze_role_capabilities() {
  local roles="$1"
  local include_caps="${2:-false}"

  echo "=== Role Capabilities Summary ==="
  echo ""

  echo "$roles" | jq -r '.[] | "Role: \(.name)\n  Capabilities: \(.capabilities | length)\n  Imported Roles: \(.imported_roles // [] | join(", ") // "none")\n  Search Indexes: \(.search_indexes // [] | join(", ") // "default")\n"' 2>/dev/null || {
    echo "$roles"
    return 1
  }

  if [[ "$include_caps" == true ]]; then
    echo "=== Detailed Role Capabilities ==="
    echo ""
    echo "$roles" | jq -r '.[] | "Role: \(.name)\n  Capabilities:\n    - \(.capabilities | join("\n    - "))\n"' 2>/dev/null
  fi
}

check_inactive_accounts() {
  local users="$1"
  local inactive_threshold="$2"

  info "Checking for inactive accounts (no login in $inactive_threshold days)..."

  # Query for recent logins
  local query="search index=_audit action=login action_result=success user=* earliest=-${inactive_threshold}d
    | stats latest(_time) as last_login by user
    | eval days_since=round((now()-last_login)/86400, 0)"

  local recent_logins
  recent_logins=$(splunk-cli search execute "$query" --wait --output json --quiet 2>/dev/null) || {
    warn "Failed to query recent login activity"
    return 1
  }

  echo ""
  echo "=== Inactive Account Analysis ==="
  echo ""

  # Get all user names
  local all_users
  all_users=$(echo "$users" | jq -r '.[].name' 2>/dev/null | sort -u)

  # Get users with recent logins
  local active_users
  active_users=$(echo "$recent_logins" | jq -r '.[].user' 2>/dev/null | sort -u)

  # Find users without recent logins (system/service accounts may not have logins)
  local inactive_users=()
  while IFS= read -r user; do
    if [[ -n "$user" ]] && ! echo "$active_users" | grep -qx "$user"; then
      inactive_users+=("$user")
    fi
  done <<< "$all_users"

  if [[ ${#inactive_users[@]} -gt 0 ]]; then
    warn "Users with no login in the last $inactive_threshold days:"
    for user in "${inactive_users[@]}"; do
      # Get user details
      local user_details
      user_details=$(echo "$users" | jq --arg u "$user" '.[] | select(.name == $u)')
      local roles
      roles=$(echo "$user_details" | jq -r '.roles | join(", ")' 2>/dev/null)
      echo "  - $user (roles: $roles)"
    done
  else
    success "All users have logged in within the last $inactive_threshold days"
  fi

  # Show active users with days since last login
  if [[ -n "$recent_logins" && "$recent_logins" != "[]" && "$recent_logins" != "null" ]]; then
    echo ""
    echo "Recent login activity:"
    echo "$recent_logins" | jq -r '.[] | select(.days_since > 0) | "  - \(.user): \(.days_since) days ago"' 2>/dev/null | head -20
  fi
  echo ""
}

find_privileged_capabilities() {
  local roles="$1"

  echo "=== Privileged Capabilities Analysis ==="
  echo ""

  # Define high-risk capabilities
  local high_risk_caps=(
    "admin_all_objects"
    "change_authentication"
    "change_own_password"
    "delete_by_keyword"
    "edit_deployment_client"
    "edit_deployment_server"
    "edit_dist_peer"
    "edit_forwarders"
    "edit_httpauths"
    "edit_roles"
    "edit_roles_grantable"
    "edit_server"
    "edit_splunktcp"
    "edit_splunktcp_ssl"
    "edit_tcp"
    "edit_tcp_ssl"
    "edit_telemetry_settings"
    "edit_user"
    "edit_tokens_own"
    "edit_tokens_settings"
    "idx_control"
    "license_edit"
    "rest_apps_management"
    "rest_properties_get"
    "rest_properties_set"
    "run_debug_commands"
    "schedule_search"
    "use_file_operator"
  )

  local high_risk_found=false

  for cap in "${high_risk_caps[@]}"; do
    local roles_with_cap
    roles_with_cap=$(echo "$roles" | jq --arg cap "$cap" '[.[] | select(.capabilities | contains([$cap])) | .name]')

    local count
    count=$(echo "$roles_with_cap" | jq 'length' 2>/dev/null || echo "0")

    if [[ "$count" -gt 0 ]]; then
      high_risk_found=true
      local role_names
      role_names=$(echo "$roles_with_cap" | jq -r '.[]' 2>/dev/null | tr '\n' ',' | sed 's/,$//')
      warn "Capability '$cap' assigned to: $role_names"
    fi
  done

  if [[ "$high_risk_found" == false ]]; then
    success "No high-risk capabilities found in role definitions"
  fi
  echo ""
}

main() {
  local output_file=""
  local include_capabilities=false

  # Parse arguments
  while [[ $# -gt 0 ]]; do
    case "$1" in
      -o|--output)
        output_file="$2"
        shift 2
        ;;
      --inactive-days)
        INACTIVE_DAYS="$2"
        shift 2
        ;;
      --admin-roles)
        IFS=',' read -ra ADMIN_ROLES <<< "$2"
        shift 2
        ;;
      --include-capabilities)
        include_capabilities=true
        shift
        ;;
      --no-color)
        RED='' GREEN='' YELLOW='' BLUE='' NC=''
        shift
        ;;
      -h|--help)
        show_help
        exit 0
        ;;
      *)
        error "Unknown option: $1"
        show_help
        exit 1
        ;;
    esac
  done

  # Header
  echo ""
  echo "========================================"
  echo "    Permission Security Review"
  echo "========================================"
  echo "Splunk: $SPLUNK_BASE_URL"
  echo "Admin Roles: ${ADMIN_ROLES[*]}"
  echo "Inactive Threshold: ${INACTIVE_DAYS} days"
  echo "Generated: $(date)"
  echo "========================================"
  echo ""

  check_prerequisites

  # Capture output if file specified
  if [[ -n "$output_file" ]]; then
    exec > >(tee "$output_file")
  fi

  # Fetch data
  local users roles
  users=$(fetch_users) || exit 1
  roles=$(fetch_roles) || exit 1

  # Run analyses
  analyze_user_roles "$users"
  identify_admin_users "$users"
  analyze_role_capabilities "$roles" "$include_capabilities"
  check_inactive_accounts "$users" "$INACTIVE_DAYS"
  find_privileged_capabilities "$roles"

  # Summary
  echo "========================================"
  echo "    Permission Review Complete"
  echo "========================================"

  if [[ -n "$output_file" ]]; then
    success "Report saved to: $output_file"
  fi

  exit 0
}

main "$@"
