//! Alerts CSV formatter.
//!
//! Responsibilities:
//! - Format fired alerts as CSV.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::{build_csv_header, build_csv_row, escape_csv, format_opt_str};
use anyhow::Result;
use splunk_client::models::FiredAlert;

/// Format fired alerts as CSV.
pub fn format_fired_alerts(alerts: &[FiredAlert]) -> Result<String> {
    if alerts.is_empty() {
        return Ok(String::new());
    }

    let mut output = String::new();

    // Header
    output.push_str(&build_csv_header(&[
        "Name",
        "SavedSearch",
        "Severity",
        "TriggerTime",
        "SID",
        "Actions",
    ]));

    for alert in alerts {
        output.push_str(&build_csv_row(&[
            escape_csv(&alert.name),
            format_opt_str(alert.savedsearch_name.as_deref(), ""),
            format_opt_str(alert.severity.as_deref(), "Medium"),
            format_opt_str(alert.trigger_time_rendered.as_deref(), ""),
            format_opt_str(alert.sid.as_deref(), ""),
            format_opt_str(alert.actions.as_deref(), ""),
        ]));
    }

    Ok(output)
}

/// Format detailed fired alert info as CSV.
pub fn format_fired_alert_info(alert: &FiredAlert) -> Result<String> {
    let mut output = String::new();

    // Header
    output.push_str(&build_csv_header(&["Property", "Value"]));

    // Data rows
    let rows = vec![
        ("Name", alert.name.clone()),
        (
            "SavedSearch",
            alert.savedsearch_name.clone().unwrap_or_default(),
        ),
        (
            "Severity",
            alert
                .severity
                .clone()
                .unwrap_or_else(|| "Medium".to_string()),
        ),
        (
            "TriggerTime",
            alert.trigger_time_rendered.clone().unwrap_or_default(),
        ),
        ("SID", alert.sid.clone().unwrap_or_default()),
        ("Actions", alert.actions.clone().unwrap_or_default()),
        ("AlertType", alert.alert_type.clone().unwrap_or_default()),
        (
            "TriggeredAlerts",
            alert.triggered_alerts.clone().unwrap_or_default(),
        ),
    ];

    for (prop, value) in rows {
        output.push_str(&build_csv_row(&[escape_csv(prop), escape_csv(&value)]));
    }

    Ok(output)
}
