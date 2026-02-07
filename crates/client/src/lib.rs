//! Splunk REST API client.
//!
//! This crate provides a type-safe client for interacting with the Splunk
//! Enterprise REST API v9+. It supports both session token and API token
//! authentication with automatic session renewal.

mod auth;
pub mod client;
pub mod error;
pub mod metrics;
pub mod models;
mod name_merge;
mod serde_helpers;

pub mod endpoints;

pub use auth::{AuthStrategy, SessionManager};
pub use client::SplunkClient;
pub use client::builder::SplunkClientBuilder;
pub use client::health::AggregatedHealth;
pub use client::macros::{MacroCreateParams, MacroUpdateParams};
pub use error::{ClientError, Result};
pub use metrics::{ErrorCategory, MetricsCollector};
pub use models::{
    AddShcMemberParams, App, AppListResponse, Capability, CapabilityListResponse, ClusterInfo,
    ClusterManagementResponse, ClusterPeer, CreateIndexParams, CreatePoolParams, CreateRoleParams,
    CreateUserParams, Dashboard, DashboardEntry, DashboardListResponse, DecommissionPeerParams,
    Forwarder, ForwarderListResponse, HealthCheckOutput, HecAckRequest, HecAckStatus,
    HecBatchResponse, HecError, HecEvent, HecHealth, HecResponse, Index, IndexListResponse,
    InstalledLicense, KvStoreMember, KvStoreReplicationStatus, KvStoreStatus,
    LicenseActivationResult, LicenseInstallResult, LicensePool, LicenseStack, LicenseUsage,
    LogEntry, LogParsingHealth, LookupTable, LookupTableEntry, LookupTableListResponse, Macro,
    MacroEntry, MacroListResponse, MaintenanceModeParams, ModifyIndexParams, ModifyPoolParams,
    ModifyRoleParams, ModifyUserParams, RemovePeersParams, RemoveShcMemberParams, Role,
    RoleListResponse, RollingRestartParams, SavedSearch, SearchJob, SearchJobListResponse,
    SearchJobResults, SearchJobStatus, SendBatchParams, ServerInfo, SetCaptainParams, ShcCaptain,
    ShcConfig, ShcManagementResponse, ShcMember, ShcStatus, SplunkHealth, SplunkResponse,
    UploadLookupParams, User, UserListResponse, WorkloadPool, WorkloadRule,
};

// Re-export search types for CLI/TUI use
pub use client::search::SearchRequest;
pub use endpoints::search::{CreateJobOptions, OutputMode, SearchMode};
