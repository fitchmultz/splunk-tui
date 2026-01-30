//! Data models for Splunk API responses.
//!
//! This module provides types for deserializing Splunk REST API responses.
//! Types are organized by resource in submodules and re-exported here
//! for convenient access.

pub mod apps;
pub mod auth;
pub mod cluster;
pub mod common;
pub mod configs;
pub mod forwarders;
pub mod indexes;
pub mod inputs;
pub mod jobs;
pub mod kvstore;
pub mod license;
pub mod logs;
pub mod saved_searches;
pub mod search_peers;
pub mod server;
pub mod users;

// Re-exports for backward compatibility
pub use apps::{App, AppEntry, AppListResponse};
pub use auth::AuthResponse;
pub use cluster::{ClusterInfo, ClusterPeer};
pub use common::{Acl, Entry, Perms, SplunkMessage, SplunkMessages, SplunkResponse};
pub use configs::{
    ConfigFile, ConfigListResponse, ConfigStanza, ConfigStanzaEntry, SUPPORTED_CONFIG_FILES,
};
pub use forwarders::{Forwarder, ForwarderEntry, ForwarderListResponse};
pub use indexes::{Index, IndexEntry, IndexListResponse};
pub use inputs::{Input, InputEntry, InputListResponse};
pub use jobs::{
    JobContent, JobEntry, SearchJob, SearchJobListResponse, SearchJobResults, SearchJobStatus,
};
pub use kvstore::{KvStoreMember, KvStoreReplicationStatus, KvStoreStatus};
pub use license::{LicensePool, LicenseStack, LicenseUsage, SlavesUsageBytes};
pub use logs::{HealthCheckOutput, LogEntry, LogParsingError, LogParsingHealth};
pub use saved_searches::{SavedSearch, SavedSearchEntry, SavedSearchListResponse};
pub use search_peers::{SearchPeer, SearchPeerEntry, SearchPeerListResponse};
pub use server::{HealthFeature, ServerInfo, SplunkHealth};
pub use users::{User, UserEntry, UserListResponse};
