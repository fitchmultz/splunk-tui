//! Index models for Splunk index management API.
//!
//! This module contains types for listing and managing Splunk indexes.

use serde::{Deserialize, Serialize};

/// Parameters for creating a new index.
#[derive(Debug, Clone, Default)]
pub struct CreateIndexParams {
    /// The name of the index to create (required).
    pub name: String,
    /// Maximum data size in MB.
    pub max_data_size_mb: Option<usize>,
    /// Maximum number of hot buckets.
    pub max_hot_buckets: Option<usize>,
    /// Maximum number of warm DBs.
    pub max_warm_db_count: Option<usize>,
    /// Frozen time period in seconds.
    pub frozen_time_period_in_secs: Option<usize>,
    /// Home path for the index.
    pub home_path: Option<String>,
    /// Cold DB path for the index.
    pub cold_db_path: Option<String>,
    /// Thawed path for the index.
    pub thawed_path: Option<String>,
    /// Cold to frozen directory.
    pub cold_to_frozen_dir: Option<String>,
}

/// Parameters for modifying an existing index.
#[derive(Debug, Clone, Default)]
pub struct ModifyIndexParams {
    /// Maximum data size in MB.
    pub max_data_size_mb: Option<usize>,
    /// Maximum number of hot buckets.
    pub max_hot_buckets: Option<usize>,
    /// Maximum number of warm DBs.
    pub max_warm_db_count: Option<usize>,
    /// Frozen time period in seconds.
    pub frozen_time_period_in_secs: Option<usize>,
    /// Home path for the index.
    pub home_path: Option<String>,
    /// Cold DB path for the index.
    pub cold_db_path: Option<String>,
    /// Thawed path for the index.
    pub thawed_path: Option<String>,
    /// Cold to frozen directory.
    pub cold_to_frozen_dir: Option<String>,
}

/// Index information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Index {
    #[serde(default)]
    pub name: String,
    #[serde(
        rename = "maxTotalDataSizeMB",
        default,
        deserialize_with = "crate::serde_helpers::opt_usize_from_string_or_number"
    )]
    pub max_total_data_size_mb: Option<usize>,
    #[serde(
        rename = "currentDBSizeMB",
        deserialize_with = "crate::serde_helpers::usize_from_string_or_number"
    )]
    pub current_db_size_mb: usize,
    #[serde(
        rename = "totalEventCount",
        deserialize_with = "crate::serde_helpers::usize_from_string_or_number"
    )]
    pub total_event_count: usize,
    #[serde(
        rename = "maxWarmDBCount",
        default,
        deserialize_with = "crate::serde_helpers::opt_usize_from_string_or_number"
    )]
    pub max_warm_db_count: Option<usize>,
    #[serde(
        rename = "maxHotBuckets",
        default,
        deserialize_with = "crate::serde_helpers::opt_string_from_number_or_string"
    )]
    pub max_hot_buckets: Option<String>,
    #[serde(
        rename = "frozenTimePeriodInSecs",
        default,
        deserialize_with = "crate::serde_helpers::opt_usize_from_string_or_number"
    )]
    pub frozen_time_period_in_secs: Option<usize>,
    #[serde(rename = "coldDBPath")]
    pub cold_db_path: Option<String>,
    #[serde(rename = "homePath")]
    pub home_path: Option<String>,
    #[serde(rename = "thawedPath")]
    pub thawed_path: Option<String>,
    #[serde(rename = "coldToFrozenDir")]
    pub cold_to_frozen_dir: Option<String>,
    #[serde(rename = "primaryIndex")]
    pub primary_index: Option<bool>,
}

/// Index list response.
#[derive(Debug, Deserialize, Clone)]
pub struct IndexListResponse {
    pub entry: Vec<IndexEntry>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct IndexEntry {
    pub name: String,
    pub content: Index,
}
