//! Users table formatter.
//!
//! Responsibilities:
//! - Format user lists as formatted tables.
//!
//! Does NOT handle:
//! - Other resource types.

use anyhow::Result;
use splunk_client::User;

/// Format users as a formatted table.
pub fn format_users(users: &[User]) -> Result<String> {
    let mut output = String::new();

    if users.is_empty() {
        output.push_str("No users found.\n");
        return Ok(output);
    }

    // Header
    output.push_str(&format!(
        "{:<20} {:<30} {:<15} {:<15}\n",
        "NAME", "REAL NAME", "TYPE", "ROLES"
    ));
    output.push_str(&format!(
        "{:<20} {:<30} {:<15} {:<15}\n",
        "====", "=========", "====", "====="
    ));

    // Rows
    for user in users {
        let name = &user.name;
        let realname = user.realname.as_deref().unwrap_or("-");
        let user_type = user
            .user_type
            .as_ref()
            .map(|t| t.to_string())
            .unwrap_or_else(|| "-".to_string());
        let roles = if user.roles.is_empty() {
            "-".to_string()
        } else {
            user.roles.join(", ")
        };

        output.push_str(&format!(
            "{:<20} {:<30} {:<15} {:<15}\n",
            name, realname, user_type, roles
        ));
    }

    Ok(output)
}
