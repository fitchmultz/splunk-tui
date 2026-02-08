//! Data models for Splunk API responses.
//!
//! This module provides types for deserializing Splunk REST API responses.
//! Types are organized by resource in submodules and re-exported here
//! for convenient access.

pub mod alerts;
pub mod apps;
pub mod audit;
pub mod auth;
pub mod capabilities;
pub mod cluster;
pub mod common;
pub mod configs;
pub mod dashboards;
pub mod datamodels;
pub mod forwarders;
pub mod hec;
pub mod indexes;
pub mod inputs;
pub mod jobs;
pub mod kvstore;
pub mod license;
pub mod logs;
pub mod lookups;
pub mod macros;
pub mod roles;
pub mod saved_searches;
pub mod search_peers;
pub mod server;
pub mod shc;
pub mod users;
pub mod workload;

// Re-exports for backward compatibility
pub use alerts::{AlertConfig, AlertSeverity, FiredAlert, FiredAlertEntry, FiredAlertListResponse};
pub use apps::{App, AppEntry, AppListResponse};
pub use audit::{
    AuditAction, AuditEvent, AuditEventEntry, AuditEventListResponse, AuditResult,
    ListAuditEventsParams,
};
#[cfg(test)]
pub use auth::AuthResponse;
pub use capabilities::{Capability, CapabilityEntry, CapabilityListResponse};
pub use cluster::{
    ClusterInfo, ClusterManagementResponse, ClusterMode, ClusterPeer, ClusterStatus,
    DecommissionPeerParams, MaintenanceModeParams, PeerState, PeerStatus, RemovePeersParams,
    ReplicationStatus,
};
pub use common::{Acl, Entry, MessageType, Perms, SplunkMessage, SplunkMessages, SplunkResponse};
pub use configs::{
    ConfigFile, ConfigListResponse, ConfigStanza, ConfigStanzaEntry, SUPPORTED_CONFIG_FILES,
};
pub use dashboards::{Dashboard, DashboardEntry, DashboardListResponse};
pub use datamodels::{DataModel, DataModelEntry, DataModelListResponse};
pub use forwarders::{Forwarder, ForwarderEntry, ForwarderListResponse};
pub use hec::{
    HecAckRequest, HecAckStatus, HecBatchResponse, HecError, HecEvent, HecHealth, HecResponse,
    SendBatchParams,
};
pub use indexes::{CreateIndexParams, Index, IndexEntry, IndexListResponse, ModifyIndexParams};
pub use inputs::{Input, InputEntry, InputListResponse, InputType};
pub use jobs::{
    JobContent, JobEntry, SearchJob, SearchJobListResponse, SearchJobResults, SearchJobStatus,
    SplError, SplWarning, ValidateSplRequest, ValidateSplResponse,
};
pub use kvstore::{
    CollectionEntry, CollectionListResponse, CreateCollectionParams, KvStoreCollection,
    KvStoreMember, KvStoreMemberStatus, KvStoreRecord, KvStoreReplicationStatus, KvStoreStatus,
    ModifyCollectionParams,
};
pub use license::{
    CreatePoolParams, InstalledLicense, LicenseActivationResult, LicenseInstallResult, LicensePool,
    LicenseStack, LicenseStatus, LicenseType, LicenseUsage, ModifyPoolParams, SlavesUsageBytes,
};
pub use logs::{HealthCheckOutput, LogEntry, LogLevel, LogParsingError, LogParsingHealth};
pub use lookups::{LookupTable, LookupTableEntry, LookupTableListResponse, UploadLookupParams};
pub use macros::{Macro, MacroEntry, MacroListResponse};
pub use roles::{CreateRoleParams, ModifyRoleParams, Role, RoleEntry, RoleListResponse};
pub use saved_searches::{SavedSearch, SavedSearchEntry, SavedSearchListResponse};
pub use search_peers::{SearchPeer, SearchPeerEntry, SearchPeerListResponse, SearchPeerStatus};
pub use server::{
    FeatureStatus, HealthFeature, HealthStatus, ServerInfo, ServerMode, SplunkHealth,
};
pub use shc::{
    AddShcMemberParams, RemoveShcMemberParams, RollingRestartParams, SetCaptainParams, ShcCaptain,
    ShcConfig, ShcManagementResponse, ShcMember, ShcMemberStatus, ShcStatus,
};
pub use users::{CreateUserParams, ModifyUserParams, User, UserEntry, UserListResponse, UserType};
pub use workload::{
    SearchType, WorkloadPool, WorkloadPoolEntry, WorkloadPoolListResponse, WorkloadRule,
    WorkloadRuleEntry, WorkloadRuleListResponse,
};
