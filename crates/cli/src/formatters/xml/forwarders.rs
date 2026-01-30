//! Forwarders XML formatter.
//!
//! Responsibilities:
//! - Format forwarder lists as XML.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::escape_xml;
use anyhow::Result;
use splunk_client::Forwarder;

/// Format forwarders as XML.
pub fn format_forwarders(forwarders: &[Forwarder], detailed: bool) -> Result<String> {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<forwarders>\n");

    for forwarder in forwarders {
        xml.push_str("  <forwarder>\n");
        xml.push_str(&format!(
            "    <name>{}</name>\n",
            escape_xml(&forwarder.name)
        ));

        if let Some(hostname) = &forwarder.hostname {
            xml.push_str(&format!(
                "    <hostname>{}</hostname>\n",
                escape_xml(hostname)
            ));
        }

        if let Some(client_name) = &forwarder.client_name {
            xml.push_str(&format!(
                "    <clientName>{}</clientName>\n",
                escape_xml(client_name)
            ));
        }

        if let Some(ip) = &forwarder.ip_address {
            xml.push_str(&format!("    <ipAddress>{}</ipAddress>\n", escape_xml(ip)));
        }

        if let Some(version) = &forwarder.version {
            xml.push_str(&format!("    <version>{}</version>\n", escape_xml(version)));
        }

        if let Some(last_phone) = &forwarder.last_phone {
            xml.push_str(&format!(
                "    <lastPhone>{}</lastPhone>\n",
                escape_xml(last_phone)
            ));
        }

        // When detailed, include additional fields
        if detailed {
            if let Some(utsname) = &forwarder.utsname {
                xml.push_str(&format!("    <utsname>{}</utsname>\n", escape_xml(utsname)));
            }

            if let Some(repo_loc) = &forwarder.repository_location {
                xml.push_str(&format!(
                    "    <repositoryLocation>{}</repositoryLocation>\n",
                    escape_xml(repo_loc)
                ));
            }

            if let Some(server_classes) = &forwarder.server_classes {
                xml.push_str("    <serverClasses>\n");
                for sc in server_classes {
                    xml.push_str(&format!(
                        "      <serverClass>{}</serverClass>\n",
                        escape_xml(sc)
                    ));
                }
                xml.push_str("    </serverClasses>\n");
            }
        }

        xml.push_str("  </forwarder>\n");
    }

    xml.push_str("</forwarders>");
    Ok(xml)
}
