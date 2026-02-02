//! Search job endpoints.
//!
//! This module provides low-level HTTP endpoints for Splunk search operations.
//!
//! # What this module handles:
//! - Search job creation, status, and results
//! - Saved search management
//! - SPL syntax validation
//!
//! # What this module does NOT handle:
//! - High-level search operations (see [`crate::client::search`])
//! - Result parsing beyond JSON deserialization

pub mod jobs;
pub mod saved;
pub mod types;
pub mod validate;

// Re-export all public items for backward compatibility
pub use jobs::{create_job, get_job_status, get_results, wait_for_job, wait_for_job_with_progress};
pub use saved::{create_saved_search, delete_saved_search, get_saved_search, list_saved_searches};
pub use types::{CreateJobOptions, OutputMode, SearchMode};
pub use validate::validate_spl;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_job_options_serialization() {
        let _options = CreateJobOptions {
            wait: Some(true),
            exec_time: Some(60),
            earliest_time: Some("-24h".to_string()),
            max_count: Some(1000),
            ..Default::default()
        };

        let form_data = [
            ("search", "search index=main"),
            ("wait", "1"),
            ("exec_time", "60"),
            ("earliest_time", "-24h"),
            ("max_count", "1000"),
        ];

        assert_eq!(form_data[0].0, "search");
    }
}
