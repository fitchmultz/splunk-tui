//! Roles CSV formatter.
//!
//! Responsibilities:
//! - Format role lists as CSV.
//!
//! Does NOT handle:
//! - Other resource types.

use anyhow::Result;
use splunk_client::Role;

/// Format roles as CSV.
pub fn format_roles(roles: &[Role]) -> Result<String> {
    let mut output = String::new();

    // Header
    output.push_str("name,capabilities,search_indexes,search_filter,imported_roles,default_app\n");

    // Rows
    for role in roles {
        let capabilities = if role.capabilities.is_empty() {
            String::new()
        } else {
            role.capabilities.join(";")
        };
        let search_indexes = if role.search_indexes.is_empty() {
            String::new()
        } else {
            role.search_indexes.join(";")
        };
        let search_filter = role.search_filter.as_deref().unwrap_or("");
        let imported_roles = if role.imported_roles.is_empty() {
            String::new()
        } else {
            role.imported_roles.join(";")
        };
        let default_app = role.default_app.as_deref().unwrap_or("");

        output.push_str(&format!(
            "{},{},{},{},{},{}\n",
            escape_csv(&role.name),
            escape_csv(&capabilities),
            escape_csv(&search_indexes),
            escape_csv(search_filter),
            escape_csv(&imported_roles),
            escape_csv(default_app)
        ));
    }

    Ok(output)
}

/// Format capabilities as CSV.
pub fn format_capabilities(capabilities: &[splunk_client::Capability]) -> Result<String> {
    let mut output = String::new();

    // Header
    output.push_str("name\n");

    // Rows
    for cap in capabilities {
        output.push_str(&format!("{}\n", escape_csv(&cap.name)));
    }

    Ok(output)
}

fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
