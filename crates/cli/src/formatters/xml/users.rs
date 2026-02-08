//! Users XML formatter.
//!
//! Responsibilities:
//! - Format user lists as XML.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::escape_xml;
use anyhow::Result;
use splunk_client::User;

/// Format users as XML.
pub fn format_users(users: &[User]) -> Result<String> {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<users>\n");

    for user in users {
        xml.push_str("  <user>\n");
        xml.push_str(&format!("    <name>{}</name>\n", escape_xml(&user.name)));

        if let Some(ref realname) = user.realname {
            xml.push_str(&format!(
                "    <realname>{}</realname>\n",
                escape_xml(realname)
            ));
        }

        if let Some(ref user_type) = user.user_type {
            xml.push_str(&format!(
                "    <type>{}</type>\n",
                escape_xml(&user_type.to_string())
            ));
        }

        if let Some(ref default_app) = user.default_app {
            xml.push_str(&format!(
                "    <defaultApp>{}</defaultApp>\n",
                escape_xml(default_app)
            ));
        }

        if !user.roles.is_empty() {
            xml.push_str("    <roles>\n");
            for role in &user.roles {
                xml.push_str(&format!("      <role>{}</role>\n", escape_xml(role)));
            }
            xml.push_str("    </roles>\n");
        }

        if let Some(last_login) = user.last_successful_login {
            xml.push_str(&format!(
                "    <lastSuccessfulLogin>{}</lastSuccessfulLogin>\n",
                last_login
            ));
        }

        xml.push_str("  </user>\n");
    }

    xml.push_str("</users>\n");
    Ok(xml)
}
