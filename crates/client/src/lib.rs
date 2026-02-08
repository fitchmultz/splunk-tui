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

/// Testing utilities for Splunk client tests.
///
/// This module provides helper functions for loading test fixtures and other
/// test-related utilities. It is available when running tests or when the
/// `test-utils` feature is enabled.
///
/// # Example
/// ```ignore
/// use splunk_client::testing::load_fixture;
///
/// let fixture = load_fixture("indexes/list_indexes.json");
/// ```
#[cfg(any(feature = "test-utils", test))]
pub mod testing {
    use std::path::Path;

    /// Load a JSON fixture file from the fixtures directory.
    ///
    /// # Arguments
    /// * `fixture_path` - Relative path within the fixtures directory (e.g., "indexes/list_indexes.json")
    ///
    /// # Panics
    /// - If the fixture file cannot be read
    /// - If the file content is not valid JSON
    pub fn load_fixture(fixture_path: &str) -> serde_json::Value {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let fixture_dir = manifest_dir.join("fixtures");
        let full_path = fixture_dir.join(fixture_path);
        let content = std::fs::read_to_string(&full_path)
            .unwrap_or_else(|_| panic!("Failed to load fixture: {}", full_path.display()));
        serde_json::from_str(&content).expect("Invalid JSON in fixture")
    }
}
