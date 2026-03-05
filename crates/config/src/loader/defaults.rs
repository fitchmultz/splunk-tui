//! Search default configuration values.
//!
//! Responsibilities:
//! - Define default values for search parameters (earliest_time, latest_time, max_results).
//! - Provide a Default implementation with sensible defaults.
//!
//! Does NOT handle:
//! - Loading or parsing configuration from files or environment variables.
//! - Persisting search defaults back to disk.
//!
//! Invariants:
//! - Default values are: earliest_time="-24h", latest_time="now", max_results=1000.
//! - These defaults are used when no other configuration source provides values.

/// Search default configuration values.
///
/// This is separate from the main `Config` because search defaults
/// are persisted to disk and managed through the TUI settings.
#[derive(Debug, Clone)]
pub struct SearchDefaultConfig {
    /// Earliest time for searches (e.g., "-24h").
    pub earliest_time: String,
    /// Latest time for searches (e.g., "now").
    pub latest_time: String,
    /// Maximum number of results to return per search.
    pub max_results: usize,
}

impl Default for SearchDefaultConfig {
    fn default() -> Self {
        Self {
            earliest_time: "-24h".to_string(),
            latest_time: "now".to_string(),
            max_results: crate::constants::DEFAULT_MAX_RESULTS,
        }
    }
}
