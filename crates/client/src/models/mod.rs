//! Data models for Splunk API responses.
//!
//! This module provides types for deserializing Splunk REST API responses.
//! Types are organized by resource in submodules and re-exported here
//! for convenient access.

pub mod alerts;
pub mod apps;
pub mod auth;
pub mod capabilities;
pub mod cluster;
pub mod common;
pub mod configs;
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
pub mod users;

// Re-exports for backward compatibility
pub use alerts::{AlertConfig, FiredAlert, FiredAlertEntry, FiredAlertListResponse};
pub use apps::{App, AppEntry, AppListResponse};
pub use auth::AuthResponse;
pub use capabilities::{Capability, CapabilityEntry, CapabilityListResponse};
pub use cluster::{
    ClusterInfo, ClusterManagementResponse, ClusterPeer, DecommissionPeerParams,
    MaintenanceModeParams, RemovePeersParams,
};
pub use common::{Acl, Entry, Perms, SplunkMessage, SplunkMessages, SplunkResponse};
pub use configs::{
    ConfigFile, ConfigListResponse, ConfigStanza, ConfigStanzaEntry, SUPPORTED_CONFIG_FILES,
};
pub use forwarders::{Forwarder, ForwarderEntry, ForwarderListResponse};
pub use hec::{
    HecAckRequest, HecAckStatus, HecBatchResponse, HecError, HecEvent, HecHealth, HecResponse,
    SendBatchParams,
};
pub use indexes::{CreateIndexParams, Index, IndexEntry, IndexListResponse, ModifyIndexParams};
pub use inputs::{Input, InputEntry, InputListResponse};
pub use jobs::{
    JobContent, JobEntry, SearchJob, SearchJobListResponse, SearchJobResults, SearchJobStatus,
    SplError, SplWarning, ValidateSplRequest, ValidateSplResponse,
};
pub use kvstore::{
    CollectionEntry, CollectionListResponse, CreateCollectionParams, KvStoreCollection,
    KvStoreMember, KvStoreRecord, KvStoreReplicationStatus, KvStoreStatus, ModifyCollectionParams,
};
pub use license::{
    CreatePoolParams, InstalledLicense, LicenseActivationResult, LicenseInstallResult, LicensePool,
    LicenseStack, LicenseUsage, ModifyPoolParams, SlavesUsageBytes,
};
pub use logs::{HealthCheckOutput, LogEntry, LogParsingError, LogParsingHealth};
pub use lookups::{LookupTable, LookupTableEntry, LookupTableListResponse};
pub use macros::{Macro, MacroEntry, MacroListResponse};
pub use roles::{CreateRoleParams, ModifyRoleParams, Role, RoleEntry, RoleListResponse};
pub use saved_searches::{SavedSearch, SavedSearchEntry, SavedSearchListResponse};
pub use search_peers::{SearchPeer, SearchPeerEntry, SearchPeerListResponse};
pub use server::{HealthFeature, ServerInfo, SplunkHealth};
pub use users::{CreateUserParams, ModifyUserParams, User, UserEntry, UserListResponse};
