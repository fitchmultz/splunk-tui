//! Splunk REST API client.
//!
//! This crate provides a type-safe client for interacting with the Splunk
//! Enterprise REST API v9+. It supports both session token and API token
//! authentication with automatic session renewal.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

mod auth;
pub mod client;
pub mod error;
pub mod format;
pub mod metrics;
pub mod metrics_exporter;
pub mod models;
mod name_merge;
pub mod tracing;
pub mod transaction;

/// Serde helper functions for deserializing Splunk's inconsistent JSON types.
///
/// These helpers are primarily useful for testing, benchmarking, and custom deserialization
/// scenarios. They are exposed when the `test-utils` feature is enabled.
#[cfg(any(feature = "test-utils", feature = "benchmark-utils"))]
pub mod serde_helpers;

#[cfg(not(any(feature = "test-utils", feature = "benchmark-utils")))]
mod serde_helpers;

pub mod endpoints;

pub use auth::{AuthStrategy, SessionManager};
pub use client::SplunkClient;
pub use client::builder::SplunkClientBuilder;
pub use client::cache::{CacheConfig, CachePolicy, CacheStats, ResponseCache};
pub use client::health::AggregatedHealth;
pub use error::{ClientError, FailureCategory, Result, RollbackFailure, UserFacingFailure};
pub use format::format_bytes;
pub use metrics::{ErrorCategory, MetricsCollector};
pub use metrics_exporter::{MetricsExporter, MetricsExporterError};
pub use models::{
    AddShcMemberParams, App, AppListResponse, Capability, CapabilityListResponse, ClusterInfo,
    ClusterManagementResponse, ClusterPeer, CreateIndexParams, CreatePoolParams, CreateRoleParams,
    CreateUserParams, Dashboard, DashboardEntry, DashboardListResponse, DecommissionPeerParams,
    Forwarder, ForwarderListResponse, HealthCheckOutput, HecAckRequest, HecAckStatus,
    HecBatchResponse, HecError, HecEvent, HecHealth, HecResponse, Index, IndexListResponse,
    InstalledLicense, KvStoreMember, KvStoreReplicationStatus, KvStoreStatus,
    LicenseActivationResult, LicenseInstallResult, LicensePool, LicenseStack, LicenseUsage,
    LogEntry, LogParsingHealth, LookupTable, LookupTableEntry, LookupTableListResponse, Macro,
    MacroCreateParams, MacroEntry, MacroListResponse, MacroUpdateParams, MaintenanceModeParams,
    ModifyIndexParams, ModifyPoolParams, ModifyRoleParams, ModifyUserParams, RemovePeersParams,
    RemoveShcMemberParams, Role, RoleListResponse, RollingRestartParams, SavedSearch, SearchJob,
    SearchJobListResponse, SearchJobResults, SearchJobStatus, SendBatchParams, ServerInfo,
    SetCaptainParams, ShcCaptain, ShcConfig, ShcManagementResponse, ShcMember, ShcStatus,
    SplunkHealth, SplunkResponse, UploadLookupParams, User, UserListResponse, WorkloadPool,
    WorkloadRule,
};
pub use tracing::{
    TracingConfig, TracingError, TracingGuard, extract_trace_context, inject_trace_context,
};

// Re-export search types for CLI/TUI use
pub use client::search::SearchRequest;
pub use endpoints::search::{CreateJobOptions, OutputMode, SearchMode};

/// Redact a query string for logging, showing only length and a short hash prefix.
/// This allows operators to correlate logs without exposing query content.
///
/// Security: Search queries may contain sensitive data (tokens, PII, proprietary SPL).
/// Never log full queries at debug level - use this redaction helper instead.
pub(crate) fn redact_query(query: &str) -> String {
    let mut hasher = DefaultHasher::new();
    query.hash(&mut hasher);
    let hash = hasher.finish();
    format!("<{} chars, hash={:08x}>", query.len(), hash)
}

#[cfg(any(feature = "test-utils", test))]
pub mod testing;
