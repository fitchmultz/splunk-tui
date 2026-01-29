//! License XML formatter.
//!
//! Responsibilities:
//! - Format license information as XML.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::LicenseInfoOutput;
use crate::formatters::common::escape_xml;
use anyhow::Result;

/// Format license information as XML.
pub fn format_license(license: &LicenseInfoOutput) -> Result<String> {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<licenseInfo>\n");

    xml.push_str("  <usage>\n");
    for u in &license.usage {
        xml.push_str("    <entry>\n");
        xml.push_str(&format!("      <name>{}</name>\n", escape_xml(&u.name)));
        if let Some(stack_id) = &u.stack_id {
            xml.push_str(&format!(
                "      <stackId>{}</stackId>\n",
                escape_xml(stack_id)
            ));
        }
        xml.push_str(&format!(
            "      <usedBytes>{}</usedBytes>\n",
            u.effective_used_bytes()
        ));
        xml.push_str(&format!("      <quotaBytes>{}</quotaBytes>\n", u.quota));
        xml.push_str("    </entry>\n");
    }
    xml.push_str("  </usage>\n");

    xml.push_str("  <pools>\n");
    for p in &license.pools {
        xml.push_str("    <pool>\n");
        xml.push_str(&format!("      <name>{}</name>\n", escape_xml(&p.name)));
        xml.push_str(&format!(
            "      <stackId>{}</stackId>\n",
            escape_xml(&p.stack_id)
        ));
        xml.push_str(&format!("      <usedBytes>{}</usedBytes>\n", p.used_bytes));
        xml.push_str(&format!(
            "      <quotaBytes>{}</quotaBytes>\n",
            escape_xml(&p.quota)
        ));
        if let Some(desc) = &p.description {
            xml.push_str(&format!(
                "      <description>{}</description>\n",
                escape_xml(desc)
            ));
        }
        xml.push_str("    </pool>\n");
    }
    xml.push_str("  </pools>\n");

    xml.push_str("  <stacks>\n");
    for s in &license.stacks {
        xml.push_str("    <stack>\n");
        xml.push_str(&format!("      <name>{}</name>\n", escape_xml(&s.name)));
        xml.push_str(&format!("      <label>{}</label>\n", escape_xml(&s.label)));
        xml.push_str(&format!(
            "      <type>{}</type>\n",
            escape_xml(&s.type_name)
        ));
        xml.push_str(&format!("      <quotaBytes>{}</quotaBytes>\n", s.quota));
        xml.push_str("    </stack>\n");
    }
    xml.push_str("  </stacks>\n");

    xml.push_str("</licenseInfo>");
    Ok(xml)
}
