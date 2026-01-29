//! Server and health models for Splunk server info API.
//!
//! This module contains types for server information and health status.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Server information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerInfo {
    #[serde(rename = "serverName")]
    pub server_name: String,
    pub version: String,
    pub build: String,
    pub mode: Option<String>,
    #[serde(rename = "server_roles", default)]
    pub server_roles: Vec<String>,
    #[serde(rename = "os_name")]
    pub os_name: Option<String>,
}

/// Health feature information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HealthFeature {
    pub health: String,
    pub status: String,
    pub disabled: i32,
    pub reasons: Vec<String>,
}

/// System-wide health information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SplunkHealth {
    pub health: String,
    #[serde(default)]
    pub features: HashMap<String, HealthFeature>,
}
