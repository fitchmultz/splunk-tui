//! Search defaults tests for the configuration loader builder.
//!
//! Responsibilities:
//! - Test search defaults environment variable handling.
//! - Test search defaults merging with persisted values.
//! - Test validation of search defaults values.

use crate::loader::builder::ConfigLoader;
use crate::loader::error::ConfigError;
use crate::persistence::SearchDefaults;
use serial_test::serial;

use super::env_lock;

#[test]
#[serial]
fn test_search_defaults_env_vars() {
    let _lock = env_lock().lock().unwrap();

    temp_env::with_vars(
        [
            ("SPLUNK_EARLIEST_TIME", Some("-48h")),
            ("SPLUNK_LATEST_TIME", Some("2024-01-01T00:00:00")),
            ("SPLUNK_MAX_RESULTS", Some("500")),
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("test-token")),
        ],
        || {
            let loader = ConfigLoader::new().from_env().unwrap();

            assert_eq!(loader.earliest_time(), Some(&"-48h".to_string()));
            assert_eq!(
                loader.latest_time(),
                Some(&"2024-01-01T00:00:00".to_string())
            );
            assert_eq!(loader.max_results(), Some(500));
        },
    );
}

#[test]
#[serial]
fn test_search_defaults_env_vars_empty_ignored() {
    let _lock = env_lock().lock().unwrap();

    temp_env::with_vars(
        [
            ("SPLUNK_EARLIEST_TIME", Some("")),
            ("SPLUNK_LATEST_TIME", Some("   ")),
            ("SPLUNK_MAX_RESULTS", Some("")),
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("test-token")),
        ],
        || {
            let loader = ConfigLoader::new().from_env().unwrap();

            // Empty/whitespace values should be treated as None
            assert_eq!(loader.earliest_time(), None);
            assert_eq!(loader.latest_time(), None);
            assert_eq!(loader.max_results(), None);
        },
    );
}

#[test]
#[serial]
fn test_build_search_defaults_with_persisted() {
    let _lock = env_lock().lock().unwrap();

    temp_env::with_vars(
        [
            ("SPLUNK_EARLIEST_TIME", Some("-7d")),
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("test-token")),
        ],
        || {
            let loader = ConfigLoader::new().from_env().unwrap();

            let persisted = SearchDefaults {
                earliest_time: "-24h".to_string(),
                latest_time: "now".to_string(),
                max_results: 1000,
            };

            let defaults = loader.build_search_defaults(Some(persisted));

            // Env var should override persisted
            assert_eq!(defaults.earliest_time, "-7d");
            // Non-env values should use persisted
            assert_eq!(defaults.latest_time, "now");
            assert_eq!(defaults.max_results, 1000);
        },
    );
}

#[test]
#[serial]
fn test_build_search_defaults_without_persisted() {
    let _lock = env_lock().lock().unwrap();

    temp_env::with_vars(
        [
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("test-token")),
        ],
        || {
            let loader = ConfigLoader::new().from_env().unwrap();

            // Build without persisted defaults - should use hardcoded defaults
            let defaults = loader.build_search_defaults(None);

            assert_eq!(defaults.earliest_time, "-24h");
            assert_eq!(defaults.latest_time, "now");
            assert_eq!(defaults.max_results, 1000);
        },
    );
}

#[test]
#[serial]
fn test_search_defaults_env_vars_override_persisted() {
    let _lock = env_lock().lock().unwrap();

    temp_env::with_vars(
        [
            ("SPLUNK_EARLIEST_TIME", Some("-1h")),
            ("SPLUNK_LATEST_TIME", Some("-5m")),
            ("SPLUNK_MAX_RESULTS", Some("100")),
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("test-token")),
        ],
        || {
            let loader = ConfigLoader::new().from_env().unwrap();

            let persisted = SearchDefaults {
                earliest_time: "-48h".to_string(),
                latest_time: "2024-01-01T00:00:00".to_string(),
                max_results: 5000,
            };

            let defaults = loader.build_search_defaults(Some(persisted));

            // All env vars should override persisted values
            assert_eq!(defaults.earliest_time, "-1h");
            assert_eq!(defaults.latest_time, "-5m");
            assert_eq!(defaults.max_results, 100);
        },
    );
}

#[test]
#[serial]
fn test_invalid_max_results_env_var() {
    let _lock = env_lock().lock().unwrap();

    temp_env::with_vars(
        [
            ("SPLUNK_MAX_RESULTS", Some("not-a-number")),
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("test-token")),
        ],
        || {
            let result = ConfigLoader::new().from_env();

            match result {
                Err(ConfigError::InvalidValue { var, .. }) => {
                    assert_eq!(var, "SPLUNK_MAX_RESULTS");
                }
                Ok(_) => panic!("Expected an error for invalid SPLUNK_MAX_RESULTS"),
                Err(_) => panic!("Expected InvalidValue error for SPLUNK_MAX_RESULTS"),
            }
        },
    );
}

#[test]
fn test_search_defaults_sanitize_empty_earliest_time() {
    let defaults = SearchDefaults {
        earliest_time: "".to_string(),
        latest_time: "now".to_string(),
        max_results: 1000,
    };

    let sanitized = defaults.sanitize();
    assert_eq!(sanitized.earliest_time, "-24h");
    assert_eq!(sanitized.latest_time, "now");
    assert_eq!(sanitized.max_results, 1000);
}

#[test]
fn test_search_defaults_sanitize_whitespace_earliest_time() {
    let defaults = SearchDefaults {
        earliest_time: "   ".to_string(),
        latest_time: "now".to_string(),
        max_results: 1000,
    };

    let sanitized = defaults.sanitize();
    assert_eq!(sanitized.earliest_time, "-24h");
    assert_eq!(sanitized.latest_time, "now");
    assert_eq!(sanitized.max_results, 1000);
}

#[test]
fn test_search_defaults_sanitize_empty_latest_time() {
    let defaults = SearchDefaults {
        earliest_time: "-24h".to_string(),
        latest_time: "".to_string(),
        max_results: 1000,
    };

    let sanitized = defaults.sanitize();
    assert_eq!(sanitized.earliest_time, "-24h");
    assert_eq!(sanitized.latest_time, "now");
    assert_eq!(sanitized.max_results, 1000);
}

#[test]
fn test_search_defaults_sanitize_whitespace_latest_time() {
    let defaults = SearchDefaults {
        earliest_time: "-24h".to_string(),
        latest_time: "   ".to_string(),
        max_results: 1000,
    };

    let sanitized = defaults.sanitize();
    assert_eq!(sanitized.earliest_time, "-24h");
    assert_eq!(sanitized.latest_time, "now");
    assert_eq!(sanitized.max_results, 1000);
}

#[test]
fn test_search_defaults_sanitize_zero_max_results() {
    let defaults = SearchDefaults {
        earliest_time: "-24h".to_string(),
        latest_time: "now".to_string(),
        max_results: 0,
    };

    let sanitized = defaults.sanitize();
    assert_eq!(sanitized.earliest_time, "-24h");
    assert_eq!(sanitized.latest_time, "now");
    assert_eq!(sanitized.max_results, 1000);
}

#[test]
fn test_search_defaults_sanitize_multiple_invalid() {
    let defaults = SearchDefaults {
        earliest_time: "".to_string(),
        latest_time: "   ".to_string(),
        max_results: 0,
    };

    let sanitized = defaults.sanitize();
    assert_eq!(sanitized.earliest_time, "-24h");
    assert_eq!(sanitized.latest_time, "now");
    assert_eq!(sanitized.max_results, 1000);
}

#[test]
fn test_search_defaults_sanitize_valid_values_unchanged() {
    let defaults = SearchDefaults {
        earliest_time: "-7d".to_string(),
        latest_time: "2024-01-01T00:00:00".to_string(),
        max_results: 500,
    };

    let sanitized = defaults.sanitize();
    assert_eq!(sanitized.earliest_time, "-7d");
    assert_eq!(sanitized.latest_time, "2024-01-01T00:00:00");
    assert_eq!(sanitized.max_results, 500);
}
