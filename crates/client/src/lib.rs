//! Splunk REST API client.
//!
//! This crate provides a type-safe client for interacting with the Splunk
//! Enterprise REST API v9+. It supports both session token and API token
//! authentication with automatic session renewal.

mod auth;
mod client;
mod error;
pub mod models;

pub mod endpoints;

pub use auth::{AuthStrategy, SessionManager};
pub use client::SplunkClient;
pub use error::{ClientError, Result};
pub use models::{
    ClusterInfo, ClusterPeer, Index, IndexListResponse, SearchJob, SearchJobListResponse,
    SearchJobResults, SearchJobStatus, SplunkResponse,
};
