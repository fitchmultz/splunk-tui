//! Roles table formatter.
//!
//! Responsibilities:
//! - Format role lists as formatted tables.
//!
//! Does NOT handle:
//! - Other resource types.

use anyhow::Result;
use splunk_client::Role;

/// Format roles as a formatted table.
pub fn format_roles(roles: &[Role]) -> Result<String> {
    let mut output = String::new();

    if roles.is_empty() {
        output.push_str("No roles found.\n");
        return Ok(output);
    }

    // Header
    output.push_str(&format!(
        "{:<20} {:<30} {:<30} {:<20}\n",
        "NAME", "CAPABILITIES", "SEARCH INDEXES", "IMPORTED ROLES"
    ));
    output.push_str(&format!(
        "{:<20} {:<30} {:<30} {:<20}\n",
        "====", "============", "==============", "=============="
    ));

    // Rows
    for role in roles {
        let name = &role.name;
        let capabilities = if role.capabilities.is_empty() {
            "-".to_string()
        } else {
            role.capabilities.join(", ")
        };
        let search_indexes = if role.search_indexes.is_empty() {
            "-".to_string()
        } else {
            role.search_indexes.join(", ")
        };
        let imported_roles = if role.imported_roles.is_empty() {
            "-".to_string()
        } else {
            role.imported_roles.join(", ")
        };

        // Truncate long fields for display
        let capabilities_display = truncate_field(&capabilities, 28);
        let search_indexes_display = truncate_field(&search_indexes, 28);
        let imported_roles_display = truncate_field(&imported_roles, 18);

        output.push_str(&format!(
            "{:<20} {:<30} {:<30} {:<20}\n",
            name, capabilities_display, search_indexes_display, imported_roles_display
        ));
    }

    Ok(output)
}

/// Format capabilities as a formatted table.
pub fn format_capabilities(capabilities: &[splunk_client::Capability]) -> Result<String> {
    let mut output = String::new();

    if capabilities.is_empty() {
        output.push_str("No capabilities found.\n");
        return Ok(output);
    }

    // Header
    output.push_str("Available Capabilities:\n\n");

    // Display in columns (4 columns)
    const COLUMNS: usize = 4;
    let items_per_column = capabilities.len().div_ceil(COLUMNS);

    for row in 0..items_per_column {
        for col in 0..COLUMNS {
            let idx = col * items_per_column + row;
            if let Some(cap) = capabilities.get(idx) {
                output.push_str(&format!("{:<25}", cap.name));
            }
        }
        output.push('\n');
    }

    Ok(output)
}

fn truncate_field(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}
