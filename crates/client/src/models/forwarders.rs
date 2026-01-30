//! Forwarder models for Splunk deployment server API.
//!
//! This module contains types for listing and managing Splunk forwarders
//! (deployment clients) that have checked in with the deployment server.
//!
//! # What this module handles:
//! - Deserialization of forwarder/deployment client data from Splunk REST API
//! - Type-safe representation of forwarder metadata
//!
//! # What this module does NOT handle:
//! - Direct HTTP API calls (see [`crate::endpoints::forwarders`])
//! - Client-side filtering or searching of forwarders
//! - Modifying forwarder configuration (not supported by this API)

use serde::{Deserialize, Serialize};

/// Forwarder (deployment client) information.
///
/// Represents a Splunk forwarder that has checked in with the deployment server.
/// The deployment server tracks forwarders that request configuration updates.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Forwarder {
    /// The forwarder name (typically the client name or hostname).
    #[serde(default)]
    pub name: String,
    /// The hostname of the forwarder.
    #[serde(rename = "hostname")]
    pub hostname: Option<String>,
    /// The client name configured on the forwarder.
    #[serde(rename = "clientName")]
    pub client_name: Option<String>,
    /// The IP address of the forwarder.
    #[serde(rename = "ipAddress")]
    pub ip_address: Option<String>,
    /// System information (utsname) from the forwarder.
    #[serde(rename = "utsname")]
    pub utsname: Option<String>,
    /// The Splunk version running on the forwarder.
    #[serde(rename = "version")]
    pub version: Option<String>,
    /// The last time the forwarder checked in (phone home).
    #[serde(rename = "lastPhone")]
    pub last_phone: Option<String>,
    /// The repository location for this forwarder's configuration.
    #[serde(rename = "repositoryLocation")]
    pub repository_location: Option<String>,
    /// The server classes this forwarder belongs to.
    #[serde(rename = "serverClasses")]
    pub server_classes: Option<Vec<String>>,
}

/// Forwarder list response.
///
/// Wrapper struct for deserializing the Splunk API response when listing forwarders.
#[derive(Debug, Deserialize, Clone)]
pub struct ForwarderListResponse {
    /// The list of forwarder entries returned by the API.
    pub entry: Vec<ForwarderEntry>,
}

/// A single forwarder entry in the list response.
///
/// Splunk's REST API wraps each resource in an entry structure containing
/// metadata and the actual content.
#[derive(Debug, Deserialize, Clone)]
pub struct ForwarderEntry {
    /// The entry name (forwarder identifier).
    pub name: String,
    /// The forwarder content/data.
    pub content: Forwarder,
}
