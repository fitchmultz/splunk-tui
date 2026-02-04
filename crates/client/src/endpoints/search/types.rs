//! Search types and options.
//!
//! This module provides types for configuring search jobs and result formats.
//!
//! # What this module handles:
//! - Search job creation options
//! - Search mode (normal/realtime)
//! - Output format for results
//!
//! # What this module does NOT handle:
//! - Search execution logic
//! - Result parsing

use serde::{Deserialize, Serialize};

/// Options for creating a search job.
#[derive(Debug, Clone, Serialize, Default)]
pub struct CreateJobOptions {
    /// Whether to wait for the job to complete.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wait: Option<bool>,
    /// Maximum time to wait for job completion (seconds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exec_time: Option<u64>,
    /// Earliest time for search (e.g., "-24h", "2024-01-01T00:00:00").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub earliest_time: Option<String>,
    /// Latest time for search (e.g., "now", "2024-01-02T00:00:00").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_time: Option<String>,
    /// Maximum number of results to return.
    #[serde(rename = "maxCount", skip_serializing_if = "Option::is_none")]
    pub max_count: Option<u64>,
    /// Output format for results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_mode: Option<OutputMode>,
    /// Search mode (normal or realtime).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_mode: Option<SearchMode>,
    /// Real-time window in seconds (only used when search_mode is Realtime).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub realtime_window: Option<u64>,
}

/// Search mode for search jobs.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SearchMode {
    #[default]
    Normal,
    Realtime,
}

impl std::fmt::Display for SearchMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SearchMode::Normal => "normal",
            SearchMode::Realtime => "realtime",
        };
        write!(f, "{}", s)
    }
}

/// Output format for search results.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OutputMode {
    #[default]
    Json,
    JsonCols,
    JsonRows,
    Xml,
    Csv,
    Raw,
}

impl std::fmt::Display for OutputMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            OutputMode::Json => "json",
            OutputMode::JsonCols => "json_cols",
            OutputMode::JsonRows => "json_rows",
            OutputMode::Xml => "xml",
            OutputMode::Csv => "csv",
            OutputMode::Raw => "raw",
        };
        write!(f, "{}", s)
    }
}
