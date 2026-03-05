//! Dashboard models for Splunk dashboard API.
//!
//! This module contains types for listing and viewing Splunk dashboards.

use serde::{Deserialize, Serialize};

/// Dashboard information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Dashboard {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub author: String,
    #[serde(rename = "isDashboard", default)]
    pub is_dashboard: bool,
    #[serde(rename = "isVisible", default)]
    pub is_visible: bool,
    #[serde(default)]
    pub version: Option<String>,
    /// The XML dashboard definition (may be large)
    #[serde(rename = "eai:data", default)]
    pub xml_data: Option<String>,
    /// Last updated timestamp
    #[serde(default)]
    pub updated: Option<String>,
}

/// Dashboard list response.
#[derive(Debug, Deserialize, Clone)]
pub struct DashboardListResponse {
    pub entry: Vec<DashboardEntry>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DashboardEntry {
    pub name: String,
    pub content: Dashboard,
}
