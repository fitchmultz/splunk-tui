//! Users CSV formatter.
//!
//! Responsibilities:
//! - Format user lists as CSV.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::{build_csv_header, build_csv_row, escape_csv, format_opt_str};
use anyhow::Result;
use splunk_client::User;

/// Format users as CSV.
pub fn format_users(users: &[User]) -> Result<String> {
    let mut output = String::new();

    // Header
    output.push_str(&build_csv_header(&[
        "name",
        "realname",
        "user_type",
        "default_app",
        "roles",
        "last_successful_login",
    ]));

    for user in users {
        let roles = user.roles.join(";");
        let last_login = user.last_successful_login.unwrap_or(0);

        output.push_str(&build_csv_row(&[
            escape_csv(&user.name),
            format_opt_str(user.realname.as_deref(), ""),
            format_opt_str(user.user_type.as_deref(), ""),
            format_opt_str(user.default_app.as_deref(), ""),
            escape_csv(&roles),
            escape_csv(&last_login.to_string()),
        ]));
    }

    Ok(output)
}
