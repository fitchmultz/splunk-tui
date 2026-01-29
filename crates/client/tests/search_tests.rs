//! Search results and internal logs endpoint tests.
//!
//! This module tests search result retrieval and internal logs fetching:
//! - Getting search results with array-style responses
//! - Getting search results with object-style responses
//! - Fetching internal logs with deterministic sorting
//!
//! # Invariants
//! - Results are returned in the expected format based on output mode
//! - Internal logs are sorted by time (descending), then index_time, then serial
//!
//! # What this does NOT handle
//! - Job creation and lifecycle (see jobs_tests.rs)
//! - Log parsing health checks (see parsing_tests.rs)

mod common;

use common::*;
use wiremock::matchers::{method, path, path_regex, query_param};

#[tokio::test]
async fn test_get_search_results() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/get_results.json");

    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::get_results(
        &client,
        &mock_server.uri(),
        "test-token",
        "test-sid",
        Some(10),
        Some(0),
        endpoints::OutputMode::Json,
        3,
        None,
    )
    .await;

    if let Err(ref e) = result {
        eprintln!("Get results error: {:?}", e);
    }
    assert!(result.is_ok());
    let results = result.unwrap();
    assert_eq!(results.results.len(), 3);
    assert_eq!(results.results[0]["message"], "Test event 1");
}

#[tokio::test]
async fn test_get_search_results_object_style() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/get_results_object.json");

    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::get_results(
        &client,
        &mock_server.uri(),
        "test-token",
        "test-sid",
        Some(10),
        Some(0),
        endpoints::OutputMode::Json,
        3,
        None,
    )
    .await;

    if let Err(ref e) = result {
        eprintln!("Get results error: {:?}", e);
    }
    assert!(result.is_ok());
    let results = result.unwrap();
    assert_eq!(results.results.len(), 1);
    assert_eq!(
        results.results[0]["message"],
        "Test event from object response"
    );
    assert!(!results.preview);
    assert_eq!(results.total, Some(1));
}

#[tokio::test]
async fn test_get_internal_logs_with_sorting() {
    let mock_server = MockServer::start().await;

    let create_job_fixture = serde_json::json!({
        "entry": [{
            "content": {
                "sid": "test-logs-sid"
            }
        }]
    });

    let results_fixture = serde_json::json!([
        {
            "_time": "2025-01-24T12:00:05.000Z",
            "_indextime": "2025-01-24T12:00:06.000Z",
            "_serial": 103,
            "log_level": "INFO",
            "component": "ComponentA",
            "_raw": "Third log"
        },
        {
            "_time": "2025-01-24T12:00:05.000Z",
            "_indextime": "2025-01-24T12:00:05.500Z",
            "_serial": 102,
            "log_level": "INFO",
            "component": "ComponentA",
            "_raw": "Second log (same time)"
        },
        {
            "_time": "2025-01-24T12:00:05.000Z",
            "_indextime": "2025-01-24T12:00:05.000Z",
            "_serial": 101,
            "log_level": "INFO",
            "component": "ComponentA",
            "_raw": "First log (same time)"
        }
    ]);

    // Mock create job - accept any search query
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&create_job_fixture))
        .mount(&mock_server)
        .await;

    // Mock results
    Mock::given(method("GET"))
        .and(path_regex(r"/services/search/jobs/[^/]+/results"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&results_fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result =
        endpoints::get_internal_logs(&client, &mock_server.uri(), "test-token", 10, None, 3, None)
            .await;

    assert!(result.is_ok());
    let logs = result.unwrap();
    assert_eq!(logs.len(), 3);

    // Verify ordering is deterministic (descending by time, then index_time, then serial)
    assert_eq!(logs[0].serial, Some(103));
    assert_eq!(logs[1].serial, Some(102));
    assert_eq!(logs[2].serial, Some(101));

    // Verify all have the same _time (first sort key)
    assert_eq!(logs[0].time, "2025-01-24T12:00:05.000Z");
    assert_eq!(logs[1].time, "2025-01-24T12:00:05.000Z");
    assert_eq!(logs[2].time, "2025-01-24T12:00:05.000Z");

    // Verify _indextime is descending (second sort key)
    assert!(logs[0].index_time >= logs[1].index_time);
    assert!(logs[1].index_time >= logs[2].index_time);
}
