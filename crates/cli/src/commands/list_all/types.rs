//! Shared list-all types re-exported from the client workflow layer.
//!
//! Purpose:
//! - Keep CLI type imports stable while sourcing multi-profile models from `splunk-client`.
//!
//! Responsibilities:
//! - Re-export shared list-all aggregation types and supported resource constants.
//!
//! Scope:
//! - Type surface only; fetching and formatting live elsewhere.
//!
//! Usage:
//! - Imported by CLI list-all fetch/output modules and tests.
//!
//! Invariants/Assumptions:
//! - `splunk-client::workflows::multi_profile` is the single source of truth.

pub type ListAllMultiOutput = splunk_client::workflows::multi_profile::ListAllMultiOutput;
pub type ProfileResult = splunk_client::workflows::multi_profile::ProfileResult;
#[allow(dead_code)]
pub const VALID_RESOURCES: &[&str] = splunk_client::workflows::multi_profile::VALID_RESOURCES;
