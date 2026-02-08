//! Fired alerts table formatter.
//!
//! Responsibilities:
//! - Format fired alert lists and details as formatted tables.
//!
//! Does NOT handle:
//! - Other resource types.

use anyhow::Result;
use splunk_client::models::FiredAlert;

/// Format fired alerts as a formatted table.
pub fn format_fired_alerts(alerts: &[FiredAlert]) -> Result<String> {
    let mut output = String::new();

    if alerts.is_empty() {
        output.push_str("No fired alerts found.");
        return Ok(output);
    }

    output.push_str(&format!(
        "{:<50} {:<20} {:<10} {:<20}\n",
        "NAME", "SAVED SEARCH", "SEVERITY", "TRIGGER TIME"
    ));
    output.push_str(&format!(
        "{:<50} {:<20} {:<10} {:<20}\n",
        "====", "============", "========", "============"
    ));

    for alert in alerts {
        let savedsearch = alert.savedsearch_name.as_deref().unwrap_or("-");
        let severity = alert
            .severity
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Medium".to_string());
        let trigger_time = alert.trigger_time_rendered.as_deref().unwrap_or("-");

        // Truncate fields if too long
        let name = if alert.name.len() > 50 {
            format!("{}...", &alert.name[..47])
        } else {
            alert.name.clone()
        };
        let savedsearch = if savedsearch.len() > 20 {
            format!("{}...", &savedsearch[..17])
        } else {
            savedsearch.to_string()
        };

        output.push_str(&format!(
            "{:<50} {:<20} {:<10} {:<20}\n",
            name, savedsearch, severity, trigger_time
        ));
    }

    Ok(output)
}

/// Format detailed fired alert information.
pub fn format_fired_alert_info(alert: &FiredAlert) -> Result<String> {
    let mut output = String::new();

    output.push_str("--- Fired Alert Information ---\n");
    output.push_str(&format!("Name: {}\n", alert.name));
    if let Some(ref savedsearch) = alert.savedsearch_name {
        output.push_str(&format!("Saved Search: {}\n", savedsearch));
    }
    if let Some(ref severity) = alert.severity {
        output.push_str(&format!("Severity: {}\n", severity));
    }
    if let Some(ref alert_type) = alert.alert_type {
        output.push_str(&format!("Alert Type: {}\n", alert_type));
    }
    if let Some(ref trigger_time) = alert.trigger_time_rendered {
        output.push_str(&format!("Trigger Time: {}\n", trigger_time));
    }
    if let Some(ref sid) = alert.sid {
        output.push_str(&format!("Search ID (SID): {}\n", sid));
    }
    if let Some(ref actions) = alert.actions {
        output.push_str(&format!("Actions: {}\n", actions));
    }
    if let Some(ref triggered) = alert.triggered_alerts {
        output.push_str(&format!("Triggered Alerts: {}\n", triggered));
    }

    Ok(output)
}
