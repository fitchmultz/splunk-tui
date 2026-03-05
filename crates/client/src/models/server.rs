//! Server and health models for Splunk server info API.
//!
//! This module contains types for server information and health status.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Health status values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum HealthStatus {
    Green,
    Yellow,
    Red,
    #[serde(other)]
    #[default]
    Unknown,
}

impl fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HealthStatus::Green => write!(f, "green"),
            HealthStatus::Yellow => write!(f, "yellow"),
            HealthStatus::Red => write!(f, "red"),
            HealthStatus::Unknown => write!(f, "unknown"),
        }
    }
}

/// Feature status values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum FeatureStatus {
    Enabled,
    Disabled,
    #[serde(other)]
    #[default]
    Unknown,
}

impl fmt::Display for FeatureStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FeatureStatus::Enabled => write!(f, "enabled"),
            FeatureStatus::Disabled => write!(f, "disabled"),
            FeatureStatus::Unknown => write!(f, "unknown"),
        }
    }
}

/// Server mode values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum ServerMode {
    Standalone,
    Peer,
    #[serde(other)]
    #[default]
    Unknown,
}

impl fmt::Display for ServerMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServerMode::Standalone => write!(f, "standalone"),
            ServerMode::Peer => write!(f, "peer"),
            ServerMode::Unknown => write!(f, "unknown"),
        }
    }
}

/// Server information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerInfo {
    #[serde(rename = "serverName")]
    pub server_name: String,
    pub version: String,
    pub build: String,
    pub mode: Option<ServerMode>,
    #[serde(rename = "serverRoles", default)]
    pub server_roles: Vec<String>,
    #[serde(rename = "osName")]
    pub os_name: Option<String>,
}

/// Health feature information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HealthFeature {
    pub health: HealthStatus,
    pub status: FeatureStatus,
    pub disabled: i32,
    pub reasons: Vec<String>,
}

/// System-wide health information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SplunkHealth {
    pub health: HealthStatus,
    #[serde(default)]
    pub features: HashMap<String, HealthFeature>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // HealthStatus tests
    #[test]
    fn health_status_default_is_unknown() {
        assert_eq!(HealthStatus::default(), HealthStatus::Unknown);
    }

    #[test]
    fn health_status_display_returns_lowercase() {
        assert_eq!(HealthStatus::Green.to_string(), "green");
        assert_eq!(HealthStatus::Yellow.to_string(), "yellow");
        assert_eq!(HealthStatus::Red.to_string(), "red");
        assert_eq!(HealthStatus::Unknown.to_string(), "unknown");
    }

    #[test]
    fn health_status_deserializes_from_lowercase() {
        let green: HealthStatus = serde_json::from_str("\"green\"").unwrap();
        let yellow: HealthStatus = serde_json::from_str("\"yellow\"").unwrap();
        let red: HealthStatus = serde_json::from_str("\"red\"").unwrap();

        assert_eq!(green, HealthStatus::Green);
        assert_eq!(yellow, HealthStatus::Yellow);
        assert_eq!(red, HealthStatus::Red);
    }

    #[test]
    fn health_status_deserializes_unknown_for_unrecognized() {
        let unknown: HealthStatus = serde_json::from_str("\"blue\"").unwrap();
        assert_eq!(unknown, HealthStatus::Unknown);
    }

    #[test]
    fn health_status_serializes_to_lowercase() {
        assert_eq!(
            serde_json::to_string(&HealthStatus::Green).unwrap(),
            "\"green\""
        );
        assert_eq!(
            serde_json::to_string(&HealthStatus::Yellow).unwrap(),
            "\"yellow\""
        );
        assert_eq!(
            serde_json::to_string(&HealthStatus::Red).unwrap(),
            "\"red\""
        );
        assert_eq!(
            serde_json::to_string(&HealthStatus::Unknown).unwrap(),
            "\"unknown\""
        );
    }

    #[test]
    fn health_status_is_copy() {
        let status = HealthStatus::Green;
        let copied = status;
        // If this compiles, HealthStatus is Copy
        let _ = status;
        assert_eq!(copied, HealthStatus::Green);
    }

    // FeatureStatus tests
    #[test]
    fn feature_status_default_is_unknown() {
        assert_eq!(FeatureStatus::default(), FeatureStatus::Unknown);
    }

    #[test]
    fn feature_status_display_returns_lowercase() {
        assert_eq!(FeatureStatus::Enabled.to_string(), "enabled");
        assert_eq!(FeatureStatus::Disabled.to_string(), "disabled");
        assert_eq!(FeatureStatus::Unknown.to_string(), "unknown");
    }

    #[test]
    fn feature_status_deserializes_from_lowercase() {
        let enabled: FeatureStatus = serde_json::from_str("\"enabled\"").unwrap();
        let disabled: FeatureStatus = serde_json::from_str("\"disabled\"").unwrap();

        assert_eq!(enabled, FeatureStatus::Enabled);
        assert_eq!(disabled, FeatureStatus::Disabled);
    }

    #[test]
    fn feature_status_deserializes_unknown_for_unrecognized() {
        let unknown: FeatureStatus = serde_json::from_str("\"pending\"").unwrap();
        assert_eq!(unknown, FeatureStatus::Unknown);
    }

    #[test]
    fn feature_status_serializes_to_lowercase() {
        assert_eq!(
            serde_json::to_string(&FeatureStatus::Enabled).unwrap(),
            "\"enabled\""
        );
        assert_eq!(
            serde_json::to_string(&FeatureStatus::Disabled).unwrap(),
            "\"disabled\""
        );
        assert_eq!(
            serde_json::to_string(&FeatureStatus::Unknown).unwrap(),
            "\"unknown\""
        );
    }

    #[test]
    fn feature_status_is_copy() {
        let status = FeatureStatus::Enabled;
        let copied = status;
        // If this compiles, FeatureStatus is Copy
        let _ = status;
        assert_eq!(copied, FeatureStatus::Enabled);
    }

    // ServerMode tests
    #[test]
    fn server_mode_default_is_unknown() {
        assert_eq!(ServerMode::default(), ServerMode::Unknown);
    }

    #[test]
    fn server_mode_display_returns_lowercase() {
        assert_eq!(ServerMode::Standalone.to_string(), "standalone");
        assert_eq!(ServerMode::Peer.to_string(), "peer");
        assert_eq!(ServerMode::Unknown.to_string(), "unknown");
    }

    #[test]
    fn server_mode_deserializes_from_lowercase() {
        let standalone: ServerMode = serde_json::from_str("\"standalone\"").unwrap();
        let peer: ServerMode = serde_json::from_str("\"peer\"").unwrap();

        assert_eq!(standalone, ServerMode::Standalone);
        assert_eq!(peer, ServerMode::Peer);
    }

    #[test]
    fn server_mode_deserializes_unknown_for_unrecognized() {
        let unknown: ServerMode = serde_json::from_str("\"cluster_master\"").unwrap();
        assert_eq!(unknown, ServerMode::Unknown);
    }

    #[test]
    fn server_mode_serializes_to_lowercase() {
        assert_eq!(
            serde_json::to_string(&ServerMode::Standalone).unwrap(),
            "\"standalone\""
        );
        assert_eq!(
            serde_json::to_string(&ServerMode::Peer).unwrap(),
            "\"peer\""
        );
        assert_eq!(
            serde_json::to_string(&ServerMode::Unknown).unwrap(),
            "\"unknown\""
        );
    }

    #[test]
    fn server_mode_is_copy() {
        let mode = ServerMode::Standalone;
        let copied = mode;
        // If this compiles, ServerMode is Copy
        let _ = mode;
        assert_eq!(copied, ServerMode::Standalone);
    }

    // Struct integration tests
    #[test]
    fn server_info_deserializes_with_mode() {
        let json = r#"{
            "serverName": "test-server",
            "version": "9.0.0",
            "build": "abcdef",
            "mode": "standalone"
        }"#;

        let info: ServerInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.server_name, "test-server");
        assert_eq!(info.mode, Some(ServerMode::Standalone));
    }

    #[test]
    fn server_info_deserializes_with_null_mode() {
        let json = r#"{
            "serverName": "test-server",
            "version": "9.0.0",
            "build": "abcdef",
            "mode": null
        }"#;

        let info: ServerInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.mode, None);
    }

    #[test]
    fn health_feature_deserializes() {
        let json = r#"{
            "health": "green",
            "status": "enabled",
            "disabled": 0,
            "reasons": []
        }"#;

        let feature: HealthFeature = serde_json::from_str(json).unwrap();
        assert_eq!(feature.health, HealthStatus::Green);
        assert_eq!(feature.status, FeatureStatus::Enabled);
        assert_eq!(feature.disabled, 0);
    }

    #[test]
    fn splunk_health_deserializes() {
        let json = r#"{
            "health": "yellow",
            "features": {
                "search": {
                    "health": "green",
                    "status": "enabled",
                    "disabled": 0,
                    "reasons": []
                }
            }
        }"#;

        let health: SplunkHealth = serde_json::from_str(json).unwrap();
        assert_eq!(health.health, HealthStatus::Yellow);
        assert_eq!(
            health.features.get("search").unwrap().health,
            HealthStatus::Green
        );
        assert_eq!(
            health.features.get("search").unwrap().status,
            FeatureStatus::Enabled
        );
    }

    #[test]
    fn splunk_health_deserializes_unknown_health() {
        let json = r#"{
            "health": "blue",
            "features": {}
        }"#;

        let health: SplunkHealth = serde_json::from_str(json).unwrap();
        assert_eq!(health.health, HealthStatus::Unknown);
    }
}
