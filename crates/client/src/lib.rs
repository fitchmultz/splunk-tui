//! Splunk REST API client.
//!
//! This crate provides a type-safe client for interacting with the Splunk
//! Enterprise REST API v9+. It supports both session token and API token
//! authentication with automatic session renewal.

mod auth;
pub mod client;
pub mod error;
pub mod models;
mod name_merge;
mod serde_helpers;

pub mod endpoints;

pub use auth::{AuthStrategy, SessionManager};
pub use client::SplunkClient;
pub use client::builder::SplunkClientBuilder;
pub use error::{ClientError, Result};
pub use models::{
    App, AppListResponse, ClusterInfo, ClusterPeer, HealthCheckOutput, Index, IndexListResponse,
    KvStoreMember, KvStoreReplicationStatus, KvStoreStatus, LicensePool, LicenseStack,
    LicenseUsage, LogParsingHealth, SavedSearch, SearchJob, SearchJobListResponse,
    SearchJobResults, SearchJobStatus, ServerInfo, SplunkHealth, SplunkResponse, User,
    UserListResponse,
};
