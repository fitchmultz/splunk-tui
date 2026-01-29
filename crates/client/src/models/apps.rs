//! App models for Splunk app management API.
//!
//! This module contains types for listing and managing Splunk apps.

use serde::{Deserialize, Serialize};

/// Splunk app information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct App {
    #[serde(default)]
    pub name: String,
    pub label: Option<String>,
    pub version: Option<String>,
    #[serde(rename = "is_configured")]
    pub is_configured: Option<bool>,
    #[serde(rename = "is_visible")]
    pub is_visible: Option<bool>,
    #[serde(default)]
    pub disabled: bool,
    pub description: Option<String>,
    pub author: Option<String>,
}

/// App entry wrapper.
#[derive(Debug, Deserialize, Clone)]
pub struct AppEntry {
    pub name: String,
    pub content: App,
}

/// App list response.
#[derive(Debug, Deserialize, Clone)]
pub struct AppListResponse {
    pub entry: Vec<AppEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_app() {
        let json = r#"{
            "name": "Splunk_TA_example",
            "label": "Example Add-on",
            "version": "1.2.3",
            "is_configured": true,
            "is_visible": true,
            "disabled": false,
            "description": "An example Splunk app",
            "author": "Splunk"
        }"#;
        let app: App = serde_json::from_str(json).unwrap();
        assert_eq!(app.name, "Splunk_TA_example");
        assert_eq!(app.label, Some("Example Add-on".to_string()));
        assert_eq!(app.version, Some("1.2.3".to_string()));
        assert_eq!(app.is_configured, Some(true));
        assert_eq!(app.is_visible, Some(true));
        assert!(!app.disabled);
        assert_eq!(app.description, Some("An example Splunk app".to_string()));
        assert_eq!(app.author, Some("Splunk".to_string()));
    }

    #[test]
    fn test_deserialize_app_with_optional_fields_missing() {
        let json = r#"{
            "name": "minimal_app",
            "disabled": true
        }"#;
        let app: App = serde_json::from_str(json).unwrap();
        assert_eq!(app.name, "minimal_app");
        assert_eq!(app.label, None);
        assert_eq!(app.version, None);
        assert_eq!(app.is_configured, None);
        assert_eq!(app.is_visible, None);
        assert!(app.disabled);
        assert_eq!(app.description, None);
        assert_eq!(app.author, None);
    }
}
