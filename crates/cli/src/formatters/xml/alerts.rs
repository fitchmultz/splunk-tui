//! Fired alerts XML formatter.
//!
//! Responsibilities:
//! - Format fired alerts list and details as XML.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::escape_xml;
use anyhow::Result;
use splunk_client::models::FiredAlert;

/// Format fired alerts as XML.
pub fn format_fired_alerts(alerts: &[FiredAlert]) -> Result<String> {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<fired-alerts>\n");

    for alert in alerts {
        xml.push_str("  <alert>\n");
        xml.push_str(&format!("    <name>{}</name>\n", escape_xml(&alert.name)));
        if let Some(ref savedsearch) = alert.savedsearch_name {
            xml.push_str(&format!(
                "    <savedsearch>{}</savedsearch>\n",
                escape_xml(savedsearch)
            ));
        }
        if let Some(ref severity) = alert.severity {
            xml.push_str(&format!(
                "    <severity>{}</severity>\n",
                escape_xml(severity)
            ));
        }
        if let Some(ref trigger_time) = alert.trigger_time_rendered {
            xml.push_str(&format!(
                "    <trigger_time>{}</trigger_time>\n",
                escape_xml(trigger_time)
            ));
        }
        if let Some(ref sid) = alert.sid {
            xml.push_str(&format!("    <sid>{}</sid>\n", escape_xml(sid)));
        }
        if let Some(ref alert_type) = alert.alert_type {
            xml.push_str(&format!(
                "    <alert_type>{}</alert_type>\n",
                escape_xml(alert_type)
            ));
        }
        if let Some(ref actions) = alert.actions {
            xml.push_str(&format!("    <actions>{}</actions>\n", escape_xml(actions)));
        }
        xml.push_str("  </alert>\n");
    }

    xml.push_str("</fired-alerts>");
    Ok(xml)
}

/// Format detailed fired alert information as XML.
pub fn format_fired_alert_info(alert: &FiredAlert) -> Result<String> {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<fired-alert>\n");

    xml.push_str(&format!("  <name>{}</name>\n", escape_xml(&alert.name)));
    if let Some(ref savedsearch) = alert.savedsearch_name {
        xml.push_str(&format!(
            "  <savedsearch>{}</savedsearch>\n",
            escape_xml(savedsearch)
        ));
    }
    if let Some(ref severity) = alert.severity {
        xml.push_str(&format!(
            "  <severity>{}</severity>\n",
            escape_xml(severity)
        ));
    }
    if let Some(ref trigger_time) = alert.trigger_time_rendered {
        xml.push_str(&format!(
            "  <trigger_time>{}</trigger_time>\n",
            escape_xml(trigger_time)
        ));
    }
    if let Some(ref sid) = alert.sid {
        xml.push_str(&format!("  <sid>{}</sid>\n", escape_xml(sid)));
    }
    if let Some(ref alert_type) = alert.alert_type {
        xml.push_str(&format!(
            "  <alert_type>{}</alert_type>\n",
            escape_xml(alert_type)
        ));
    }
    if let Some(ref actions) = alert.actions {
        xml.push_str(&format!("  <actions>{}</actions>\n", escape_xml(actions)));
    }
    if let Some(ref triggered) = alert.triggered_alerts {
        xml.push_str(&format!(
            "  <triggered_alerts>{}</triggered_alerts>\n",
            escape_xml(triggered)
        ));
    }
    if let Some(ref digest_mode) = alert.digest_mode {
        xml.push_str(&format!("  <digest_mode>{}</digest_mode>\n", digest_mode));
    }

    xml.push_str("</fired-alert>");
    Ok(xml)
}
