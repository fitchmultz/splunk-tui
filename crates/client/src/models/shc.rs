//! Search Head Cluster (SHC) models for Splunk SHC management API.

use serde::{Deserialize, Serialize};

/// SHC member information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShcMember {
    pub id: String,
    pub label: Option<String>,
    pub host: String,
    pub port: u32,
    pub status: String,
    pub is_captain: bool,
    pub is_dynamic_captain: Option<bool>,
    pub guid: String,
    pub site: Option<String>,
    pub replication_port: Option<u32>,
    pub last_heartbeat: Option<String>,
    pub pending_job_count: Option<u32>,
}

/// SHC captain information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShcCaptain {
    pub id: String,
    pub label: Option<String>,
    pub host: String,
    pub port: u32,
    pub guid: String,
    pub site: Option<String>,
    pub is_dynamic_captain: bool,
    pub election_epoch: Option<u64>,
}

/// SHC cluster status.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShcStatus {
    pub is_captain: bool,
    pub is_searchable: bool,
    pub captain_uri: Option<String>,
    pub member_count: u32,
    pub minimum_member_count: Option<u32>,
    pub election_timeout: Option<u32>,
    pub rolling_restart_flag: Option<bool>,
    pub service_ready_flag: Option<bool>,
}

/// SHC configuration.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShcConfig {
    pub id: String,
    pub label: Option<String>,
    pub replication_factor: Option<u32>,
    pub deployer_push_mode: Option<String>,
    pub captain_uri: Option<String>,
    pub shcluster_label: Option<String>,
}

/// Parameters for adding a member to SHC.
#[derive(Debug, Serialize)]
pub struct AddShcMemberParams {
    /// Target member URI to add
    pub target_uri: String,
}

/// Parameters for removing a member from SHC.
#[derive(Debug, Serialize)]
pub struct RemoveShcMemberParams {
    /// Member GUID to remove
    pub member: String,
}

/// Response from an SHC management operation.
#[derive(Debug, Deserialize)]
pub struct ShcManagementResponse {
    /// Whether the operation was successful
    pub success: bool,
    /// Optional message from the server
    pub message: Option<String>,
}

/// Parameters for triggering a rolling restart.
#[derive(Debug, Serialize)]
pub struct RollingRestartParams {
    /// Force restart even if some members are not ready
    pub force: bool,
}

/// Parameters for setting captain.
#[derive(Debug, Serialize)]
pub struct SetCaptainParams {
    /// Target member GUID to become captain
    pub target_guid: String,
}
