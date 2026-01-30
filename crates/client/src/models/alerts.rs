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
    #[serde(rename = "alert_type")]
    pub alert_type: Option<String>,
    /// Digest mode setting.
    #[serde(rename = "digest_mode")]
    pub digest_mode: Option<bool>,
    /// Expiration time rendered.
    #[serde(rename = "expiration_time_rendered")]
    pub expiration_time_rendered: Option<String>,
    /// Name of the saved search that triggered the alert.
    #[serde(rename = "savedsearch_name")]
    pub savedsearch_name: Option<String>,
    /// Severity level: Info, Low, Medium, High, Critical. Default is Medium.
    pub severity: Option<String>,
    /// The search ID of the search that triggered the alert.
    pub sid: Option<String>,
    /// The time the alert was triggered.
    #[serde(rename = "trigger_time")]
    pub trigger_time: Option<i64>,
    /// Trigger time rendered.
    #[serde(rename = "trigger_time_rendered")]
    pub trigger_time_rendered: Option<String>,
    /// Triggered alerts count.
    #[serde(rename = "triggered_alerts")]
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
    #[serde(rename = "is_scheduled")]
    pub is_scheduled: Option<bool>,
    /// Alert condition (e.g., "number of events", "custom").
    #[serde(rename = "alert_condition")]
    pub alert_condition: Option<String>,
    /// Alert severity: Info, Low, Medium, High, Critical.
    #[serde(rename = "alert_severity")]
    pub alert_severity: Option<String>,
    /// Alert expiration in seconds.
    #[serde(rename = "alert_expires")]
    pub alert_expires: Option<String>,
    /// Digest mode (group alerts).
    #[serde(rename = "alert_digest_mode")]
    pub alert_digest_mode: Option<bool>,
    /// Alert track setting.
    #[serde(rename = "alert_track")]
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
