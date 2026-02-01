//! License XML formatter.
//!
//! Responsibilities:
//! - Format license information as XML.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::escape_xml;
use crate::formatters::{LicenseInfoOutput, LicenseInstallOutput, LicensePoolOperationOutput};
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

/// Format installed licenses as XML.
pub fn format_installed_licenses(licenses: &[splunk_client::InstalledLicense]) -> Result<String> {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<licenses>\n");

    for license in licenses {
        xml.push_str("  <license>\n");
        xml.push_str(&format!("    <name>{}</name>\n", escape_xml(&license.name)));
        xml.push_str(&format!(
            "    <type>{}</type>\n",
            escape_xml(&license.license_type)
        ));
        xml.push_str(&format!(
            "    <status>{}</status>\n",
            escape_xml(&license.status)
        ));
        xml.push_str(&format!(
            "    <quotaBytes>{}</quotaBytes>\n",
            license.quota_bytes
        ));
        if let Some(exp) = &license.expiration_time {
            xml.push_str(&format!(
                "    <expirationTime>{}</expirationTime>\n",
                escape_xml(exp)
            ));
        }
        xml.push_str("    <features>\n");
        for feature in &license.features {
            xml.push_str(&format!(
                "      <feature>{}</feature>\n",
                escape_xml(feature)
            ));
        }
        xml.push_str("    </features>\n");
        xml.push_str("  </license>\n");
    }

    xml.push_str("</licenses>");
    Ok(xml)
}

/// Format license installation result as XML.
pub fn format_license_install(result: &LicenseInstallOutput) -> Result<String> {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<licenseInstall>\n");

    xml.push_str(&format!("  <success>{}</success>\n", result.success));
    xml.push_str(&format!(
        "  <message>{}</message>\n",
        escape_xml(&result.message)
    ));
    if let Some(name) = &result.license_name {
        xml.push_str(&format!(
            "  <licenseName>{}</licenseName>\n",
            escape_xml(name)
        ));
    }

    xml.push_str("</licenseInstall>");
    Ok(xml)
}

/// Format license pools as XML.
pub fn format_license_pools(pools: &[splunk_client::LicensePool]) -> Result<String> {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<licensePools>\n");

    for pool in pools {
        xml.push_str("  <pool>\n");
        xml.push_str(&format!("    <name>{}</name>\n", escape_xml(&pool.name)));
        xml.push_str(&format!(
            "    <stackId>{}</stackId>\n",
            escape_xml(&pool.stack_id)
        ));
        xml.push_str(&format!("    <usedBytes>{}</usedBytes>\n", pool.used_bytes));
        xml.push_str(&format!(
            "    <quotaBytes>{}</quotaBytes>\n",
            escape_xml(&pool.quota)
        ));
        if let Some(desc) = &pool.description {
            xml.push_str(&format!(
                "    <description>{}</description>\n",
                escape_xml(desc)
            ));
        }
        xml.push_str("  </pool>\n");
    }

    xml.push_str("</licensePools>");
    Ok(xml)
}

/// Format license pool operation result as XML.
pub fn format_license_pool_operation(result: &LicensePoolOperationOutput) -> Result<String> {
    let mut xml =
        String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<licensePoolOperation>\n");

    xml.push_str(&format!(
        "  <operation>{}</operation>\n",
        escape_xml(&result.operation)
    ));
    xml.push_str(&format!(
        "  <poolName>{}</poolName>\n",
        escape_xml(&result.pool_name)
    ));
    xml.push_str(&format!("  <success>{}</success>\n", result.success));
    xml.push_str(&format!(
        "  <message>{}</message>\n",
        escape_xml(&result.message)
    ));

    xml.push_str("</licensePoolOperation>");
    Ok(xml)
}
