//! Alert models for Splunk alerts API.
//!
//! This module contains types for listing and managing fired alerts.
//!
//! # What this module handles:
//! - Deserialization of fired alert data from Splunk REST API
//! - Type-safe representation of alert metadata and trigger information
//!
//! # What this module does NOT handle:
//! - Direct HTTP API calls (see [`crate::endpoints::alerts`])
//! - Client-side filtering or searching of alerts
//!
//! Splunk alerts API endpoints:
//! - /services/alerts/fired_alerts
//! - /services/alerts/fired_alerts/{name}

use serde::{Deserialize, Serialize};
use std::fmt;

/// Alert severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[derive(Default)]
pub enum AlertSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
    /// Unknown or unrecognized severity value.
    #[serde(other)]
    #[default]
    Unknown,
}

impl fmt::Display for AlertSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Info => write!(f, "Info"),
            Self::Low => write!(f, "Low"),
            Self::Medium => write!(f, "Medium"),
            Self::High => write!(f, "High"),
            Self::Critical => write!(f, "Critical"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Fired alert information.
///
/// Represents a triggered alert instance from Splunk's fired alerts endpoint.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FiredAlert {
    /// The alert name (entry name from API).
    #[serde(default)]
    pub name: String,
    /// Additional alert actions triggered by this alert.
    pub actions: Option<String>,
    /// Indicates if the alert was historical or real-time.
    #[serde(rename = "alertType")]
    pub alert_type: Option<String>,
    /// Digest mode setting.
    #[serde(rename = "digestMode")]
    pub digest_mode: Option<bool>,
    /// Expiration time rendered.
    #[serde(rename = "expirationTimeRendered")]
    pub expiration_time_rendered: Option<String>,
    /// Name of the saved search that triggered the alert.
    #[serde(rename = "savedsearchName")]
    pub savedsearch_name: Option<String>,
    /// Severity level: Info, Low, Medium, High, Critical.
    pub severity: Option<AlertSeverity>,
    /// The search ID of the search that triggered the alert.
    pub sid: Option<String>,
    /// The time the alert was triggered.
    #[serde(rename = "triggerTime")]
    pub trigger_time: Option<i64>,
    /// Trigger time rendered.
    #[serde(rename = "triggerTimeRendered")]
    pub trigger_time_rendered: Option<String>,
    /// Triggered alerts count.
    #[serde(rename = "triggeredAlerts")]
    pub triggered_alerts: Option<String>,
}

/// Fired alert entry wrapper.
///
/// Splunk's REST API wraps each resource in an entry structure containing
/// metadata and the actual content.
#[derive(Debug, Deserialize, Clone)]
pub struct FiredAlertEntry {
    pub name: String,
    pub content: FiredAlert,
}

/// Fired alert list response.
///
/// Wrapper struct for deserializing the Splunk API response when listing fired alerts.
#[derive(Debug, Deserialize, Clone)]
pub struct FiredAlertListResponse {
    /// The list of fired alert entries returned by the API.
    pub entry: Vec<FiredAlertEntry>,
}

/// Alert configuration for a saved search.
///
/// Represents alert settings that can be configured on a saved search.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AlertConfig {
    /// Whether alerting is enabled on this saved search.
    #[serde(rename = "isScheduled")]
    pub is_scheduled: Option<bool>,
    /// Alert condition (e.g., "number of events", "custom").
    #[serde(rename = "alertCondition")]
    pub alert_condition: Option<String>,
    /// Alert severity: Info, Low, Medium, High, Critical.
    #[serde(rename = "alertSeverity")]
    pub alert_severity: Option<AlertSeverity>,
    /// Alert expiration in seconds.
    #[serde(rename = "alertExpires")]
    pub alert_expires: Option<String>,
    /// Digest mode (group alerts).
    #[serde(rename = "alertDigestMode")]
    pub alert_digest_mode: Option<bool>,
    /// Alert track setting.
    #[serde(rename = "alertTrack")]
    pub alert_track: Option<String>,
    /// Comma-separated list of alert actions (email, webhook, etc.).
    #[serde(rename = "actions")]
    pub actions: Option<String>,
    /// Email alert settings.
    #[serde(rename = "action.email")]
    pub action_email: Option<bool>,
    #[serde(rename = "action.email.to")]
    pub action_email_to: Option<String>,
    /// Webhook alert settings.
    #[serde(rename = "action.webhook")]
    pub action_webhook: Option<bool>,
    #[serde(rename = "action.webhook.param.url")]
    pub action_webhook_url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alert_severity_deserialization() {
        assert_eq!(
            serde_json::from_str::<AlertSeverity>("\"Info\"").unwrap(),
            AlertSeverity::Info
        );
        assert_eq!(
            serde_json::from_str::<AlertSeverity>("\"Low\"").unwrap(),
            AlertSeverity::Low
        );
        assert_eq!(
            serde_json::from_str::<AlertSeverity>("\"Medium\"").unwrap(),
            AlertSeverity::Medium
        );
        assert_eq!(
            serde_json::from_str::<AlertSeverity>("\"High\"").unwrap(),
            AlertSeverity::High
        );
        assert_eq!(
            serde_json::from_str::<AlertSeverity>("\"Critical\"").unwrap(),
            AlertSeverity::Critical
        );
    }

    #[test]
    fn test_alert_severity_unknown_fallback() {
        assert_eq!(
            serde_json::from_str::<AlertSeverity>("\"UnknownValue\"").unwrap(),
            AlertSeverity::Unknown
        );
        assert_eq!(
            serde_json::from_str::<AlertSeverity>("\"invalid\"").unwrap(),
            AlertSeverity::Unknown
        );
    }

    #[test]
    fn test_alert_severity_display() {
        assert_eq!(AlertSeverity::Info.to_string(), "Info");
        assert_eq!(AlertSeverity::Low.to_string(), "Low");
        assert_eq!(AlertSeverity::Medium.to_string(), "Medium");
        assert_eq!(AlertSeverity::High.to_string(), "High");
        assert_eq!(AlertSeverity::Critical.to_string(), "Critical");
        assert_eq!(AlertSeverity::Unknown.to_string(), "Unknown");
    }

    #[test]
    fn test_alert_severity_default() {
        assert_eq!(AlertSeverity::default(), AlertSeverity::Unknown);
    }

    #[test]
    fn test_fired_alert_with_severity() {
        let json = r#"{
            "name": "TestAlert",
            "severity": "High"
        }"#;
        let alert: FiredAlert = serde_json::from_str(json).unwrap();
        assert_eq!(alert.name, "TestAlert");
        assert_eq!(alert.severity, Some(AlertSeverity::High));
    }

    #[test]
    fn test_alert_config_with_severity() {
        let json = r#"{
            "isScheduled": true,
            "alertSeverity": "Critical"
        }"#;
        let config: AlertConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.is_scheduled, Some(true));
        assert_eq!(config.alert_severity, Some(AlertSeverity::Critical));
    }
}
