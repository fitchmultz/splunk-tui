//! Search Head Cluster (SHC) models for Splunk SHC management API.

use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

/// Status of an SHC member.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[derive(Default)]
pub enum ShcMemberStatus {
    /// Member is up and operational
    Up,
    /// Member is down
    Down,
    /// Member is pending (joining or initializing)
    Pending,
    /// Member is restarting
    Restarting,
    /// Unknown or unrecognized status
    #[serde(other)]
    #[default]
    Unknown,
}

impl Display for ShcMemberStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Up => "Up",
            Self::Down => "Down",
            Self::Pending => "Pending",
            Self::Restarting => "Restarting",
            Self::Unknown => "Unknown",
        };
        write!(f, "{}", s)
    }
}

/// SHC member information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ShcMember {
    pub id: String,
    pub label: Option<String>,
    pub host: String,
    pub port: u32,
    pub status: ShcMemberStatus,
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
    pub election_epoch: Option<usize>,
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

#[cfg(test)]
mod tests {
    use super::*;

    mod shc_member_status_tests {
        use super::*;

        #[test]
        fn test_deserialize_up() {
            let json = "\"Up\"";
            let status: ShcMemberStatus = serde_json::from_str(json).unwrap();
            assert_eq!(status, ShcMemberStatus::Up);
        }

        #[test]
        fn test_deserialize_down() {
            let json = "\"Down\"";
            let status: ShcMemberStatus = serde_json::from_str(json).unwrap();
            assert_eq!(status, ShcMemberStatus::Down);
        }

        #[test]
        fn test_deserialize_pending() {
            let json = "\"Pending\"";
            let status: ShcMemberStatus = serde_json::from_str(json).unwrap();
            assert_eq!(status, ShcMemberStatus::Pending);
        }

        #[test]
        fn test_deserialize_restarting() {
            let json = "\"Restarting\"";
            let status: ShcMemberStatus = serde_json::from_str(json).unwrap();
            assert_eq!(status, ShcMemberStatus::Restarting);
        }

        #[test]
        fn test_deserialize_unknown_returns_unknown() {
            let json = "\"Unknown\"";
            let status: ShcMemberStatus = serde_json::from_str(json).unwrap();
            assert_eq!(status, ShcMemberStatus::Unknown);
        }

        #[test]
        fn test_deserialize_unrecognized_value_returns_unknown() {
            let json = "\"SomeFutureStatus\"";
            let status: ShcMemberStatus = serde_json::from_str(json).unwrap();
            assert_eq!(status, ShcMemberStatus::Unknown);
        }

        #[test]
        fn test_deserialize_empty_string_returns_unknown() {
            let json = "\"\"";
            let status: ShcMemberStatus = serde_json::from_str(json).unwrap();
            assert_eq!(status, ShcMemberStatus::Unknown);
        }

        #[test]
        fn test_serialize_up() {
            let status = ShcMemberStatus::Up;
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, "\"Up\"");
        }

        #[test]
        fn test_serialize_down() {
            let status = ShcMemberStatus::Down;
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, "\"Down\"");
        }

        #[test]
        fn test_serialize_pending() {
            let status = ShcMemberStatus::Pending;
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, "\"Pending\"");
        }

        #[test]
        fn test_serialize_restarting() {
            let status = ShcMemberStatus::Restarting;
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, "\"Restarting\"");
        }

        #[test]
        fn test_serialize_unknown() {
            let status = ShcMemberStatus::Unknown;
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, "\"Unknown\"");
        }

        #[test]
        fn test_display_up() {
            assert_eq!(ShcMemberStatus::Up.to_string(), "Up");
        }

        #[test]
        fn test_display_down() {
            assert_eq!(ShcMemberStatus::Down.to_string(), "Down");
        }

        #[test]
        fn test_display_pending() {
            assert_eq!(ShcMemberStatus::Pending.to_string(), "Pending");
        }

        #[test]
        fn test_display_restarting() {
            assert_eq!(ShcMemberStatus::Restarting.to_string(), "Restarting");
        }

        #[test]
        fn test_display_unknown() {
            assert_eq!(ShcMemberStatus::Unknown.to_string(), "Unknown");
        }

        #[test]
        fn test_default_is_unknown() {
            let status = ShcMemberStatus::default();
            assert_eq!(status, ShcMemberStatus::Unknown);
        }

        #[test]
        fn test_copy_trait() {
            let status = ShcMemberStatus::Up;
            let copied = status;
            // If this compiles, Copy trait works
            assert_eq!(status, copied);
        }

        #[test]
        fn test_clone_trait() {
            let status = ShcMemberStatus::Up;
            let cloned = status;
            assert_eq!(status, cloned);
        }

        #[test]
        fn test_partial_eq() {
            assert_eq!(ShcMemberStatus::Up, ShcMemberStatus::Up);
            assert_ne!(ShcMemberStatus::Up, ShcMemberStatus::Down);
        }

        #[test]
        fn test_eq_trait() {
            // Ensure Eq trait is implemented (this is a compile-time check)
            fn assert_eq<T: Eq>() {}
            assert_eq::<ShcMemberStatus>();
        }
    }

    mod shc_member_integration_tests {
        use super::*;

        #[test]
        fn test_deserialize_member_with_up_status() {
            let json = r#"{
                "id": "member1",
                "host": "splunk-sh1",
                "port": 8089,
                "status": "Up",
                "is_captain": true,
                "guid": "abc123"
            }"#;
            let member: ShcMember = serde_json::from_str(json).unwrap();
            assert_eq!(member.id, "member1");
            assert_eq!(member.status, ShcMemberStatus::Up);
            assert!(member.is_captain);
        }

        #[test]
        fn test_deserialize_member_with_down_status() {
            let json = r#"{
                "id": "member2",
                "host": "splunk-sh2",
                "port": 8089,
                "status": "Down",
                "is_captain": false,
                "guid": "def456"
            }"#;
            let member: ShcMember = serde_json::from_str(json).unwrap();
            assert_eq!(member.status, ShcMemberStatus::Down);
            assert!(!member.is_captain);
        }

        #[test]
        fn test_deserialize_member_with_pending_status() {
            let json = r#"{
                "id": "member3",
                "host": "splunk-sh3",
                "port": 8089,
                "status": "Pending",
                "is_captain": false,
                "guid": "ghi789"
            }"#;
            let member: ShcMember = serde_json::from_str(json).unwrap();
            assert_eq!(member.status, ShcMemberStatus::Pending);
        }

        #[test]
        fn test_deserialize_member_with_restarting_status() {
            let json = r#"{
                "id": "member4",
                "host": "splunk-sh4",
                "port": 8089,
                "status": "Restarting",
                "is_captain": false,
                "guid": "jkl012"
            }"#;
            let member: ShcMember = serde_json::from_str(json).unwrap();
            assert_eq!(member.status, ShcMemberStatus::Restarting);
        }

        #[test]
        fn test_deserialize_member_with_unknown_status() {
            let json = r#"{
                "id": "member5",
                "host": "splunk-sh5",
                "port": 8089,
                "status": "Unknown",
                "is_captain": false,
                "guid": "mno345"
            }"#;
            let member: ShcMember = serde_json::from_str(json).unwrap();
            assert_eq!(member.status, ShcMemberStatus::Unknown);
        }

        #[test]
        fn test_deserialize_member_with_unrecognized_status_defaults_to_unknown() {
            let json = r#"{
                "id": "member6",
                "host": "splunk-sh6",
                "port": 8089,
                "status": "FutureStatus",
                "is_captain": false,
                "guid": "pqr678"
            }"#;
            let member: ShcMember = serde_json::from_str(json).unwrap();
            assert_eq!(member.status, ShcMemberStatus::Unknown);
        }

        #[test]
        fn test_serialize_member_with_status() {
            let member = ShcMember {
                id: "member1".to_string(),
                label: Some("SH1".to_string()),
                host: "splunk-sh1".to_string(),
                port: 8089,
                status: ShcMemberStatus::Up,
                is_captain: true,
                is_dynamic_captain: Some(true),
                guid: "abc123".to_string(),
                site: Some("site1".to_string()),
                replication_port: Some(9887),
                last_heartbeat: Some("2024-01-01T00:00:00Z".to_string()),
                pending_job_count: Some(0),
            };
            let json = serde_json::to_string(&member).unwrap();
            assert!(json.contains("\"status\":\"Up\""));
        }

        #[test]
        fn test_shc_member_clone() {
            let member = ShcMember {
                id: "member1".to_string(),
                label: None,
                host: "splunk-sh1".to_string(),
                port: 8089,
                status: ShcMemberStatus::Up,
                is_captain: false,
                is_dynamic_captain: None,
                guid: "abc".to_string(),
                site: None,
                replication_port: None,
                last_heartbeat: None,
                pending_job_count: None,
            };
            let cloned = member.clone();
            assert_eq!(member.id, cloned.id);
            assert_eq!(member.status, cloned.status);
        }
    }
}
