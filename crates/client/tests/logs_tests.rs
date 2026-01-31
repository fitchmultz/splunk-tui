//! Internal logs endpoint tests.
//!
//! This module tests the Splunk internal logs API:
//! - Retrieving internal logs from the _internal index
//! - Testing with various filters (earliest time, count)
//!
//! # Invariants
//! - Internal logs are returned as LogEntry structs with time, level, component, and message
//! - Logs are retrieved via search job API (create job → get results)
//! - Empty results are handled gracefully
//!
//! # What this does NOT handle
//! - Real-time log streaming (not supported by this endpoint)
//! - Log forwarding configuration

mod common;

use common::*;
use wiremock::matchers::{method, path, query_param};

#[tokio::test]
async fn test_get_internal_logs() {
    let mock_server = MockServer::start().await;

    let create_job_fixture = serde_json::json!({
        "entry": [{
            "content": {
                "sid": "test-logs-sid-123"
            }
        }]
    });

    let job_status_fixture = serde_json::json!({
        "entry": [{
            "content": {
                "sid": "test-logs-sid-123",
                "isDone": true,
                "doneProgress": 1.0,
                "runDuration": 2.5,
                "scanCount": 100,
                "eventCount": 2,
                "resultCount": 2,
                "diskUsage": 512
            }
        }]
    });

    let results_fixture = load_fixture("logs/get_internal_logs.json");

    // Mock create job
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&create_job_fixture))
        .mount(&mock_server)
        .await;

    // Mock job status check
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-logs-sid-123"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&job_status_fixture))
        .mount(&mock_server)
        .await;

    // Mock get results
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-logs-sid-123/results"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&results_fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result =
        endpoints::get_internal_logs(&client, &mock_server.uri(), "test-token", 10, None, 3, None)
            .await;

    if let Err(ref e) = result {
        eprintln!("Get internal logs error: {:?}", e);
    }
    assert!(result.is_ok());
    let logs = result.unwrap();
    assert_eq!(logs.len(), 2);
    assert_eq!(logs[0].time, "2025-01-20T10:30:00.000+00:00");
    assert_eq!(logs[0].level, "INFO");
    assert_eq!(logs[0].component, "Metrics");
    assert_eq!(logs[0].serial, Some(42));
    assert_eq!(logs[1].time, "2025-01-20T10:29:00.000+00:00");
    assert_eq!(logs[1].level, "WARN");
    assert_eq!(logs[1].component, "Indexer");
    assert_eq!(logs[1].serial, Some(41));
}

#[tokio::test]
async fn test_get_internal_logs_empty() {
    let mock_server = MockServer::start().await;

    let create_job_fixture = serde_json::json!({
        "entry": [{
            "content": {
                "sid": "test-logs-sid-empty"
            }
        }]
    });

    let job_status_fixture = serde_json::json!({
        "entry": [{
            "content": {
                "sid": "test-logs-sid-empty",
                "isDone": true,
                "doneProgress": 1.0,
                "runDuration": 1.0,
                "scanCount": 0,
                "eventCount": 0,
                "resultCount": 0,
                "diskUsage": 0
            }
        }]
    });

    let results_fixture = load_fixture("logs/get_internal_logs_empty.json");

    // Mock create job
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&create_job_fixture))
        .mount(&mock_server)
        .await;

    // Mock job status check
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-logs-sid-empty"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&job_status_fixture))
        .mount(&mock_server)
        .await;

    // Mock get results
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-logs-sid-empty/results"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&results_fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result =
        endpoints::get_internal_logs(&client, &mock_server.uri(), "test-token", 10, None, 3, None)
            .await;

    assert!(result.is_ok());
    let logs = result.unwrap();
    assert!(logs.is_empty());
}

#[tokio::test]
async fn test_get_internal_logs_with_earliest() {
    let mock_server = MockServer::start().await;

    let create_job_fixture = serde_json::json!({
        "entry": [{
            "content": {
                "sid": "test-logs-sid-earliest"
            }
        }]
    });

    let job_status_fixture = serde_json::json!({
        "entry": [{
            "content": {
                "sid": "test-logs-sid-earliest",
                "isDone": true,
                "doneProgress": 1.0,
                "runDuration": 2.0,
                "scanCount": 50,
                "eventCount": 1,
                "resultCount": 1,
                "diskUsage": 256
            }
        }]
    });

    let results_fixture = load_fixture("logs/get_internal_logs.json");

    // Mock create job - should include earliest_time parameter
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&create_job_fixture))
        .mount(&mock_server)
        .await;

    // Mock job status check
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-logs-sid-earliest"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&job_status_fixture))
        .mount(&mock_server)
        .await;

    // Mock get results
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-logs-sid-earliest/results"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&results_fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::get_internal_logs(
        &client,
        &mock_server.uri(),
        "test-token",
        5,
        Some("-1h"),
        3,
        None,
    )
    .await;

    assert!(result.is_ok());
    let logs = result.unwrap();
    // Verify logs are returned (the earliest filter is passed to the search job)
    assert_eq!(logs.len(), 2);
}

#[tokio::test]
async fn test_splunk_client_get_internal_logs() {
    let mock_server = MockServer::start().await;

    let create_job_fixture = serde_json::json!({
        "entry": [{
            "content": {
                "sid": "test-logs-sid-client"
            }
        }]
    });

    let job_status_fixture = serde_json::json!({
        "entry": [{
            "content": {
                "sid": "test-logs-sid-client",
                "isDone": true,
                "doneProgress": 1.0,
                "runDuration": 2.0,
                "scanCount": 50,
                "eventCount": 2,
                "resultCount": 2,
                "diskUsage": 256
            }
        }]
    });

    let results_fixture = load_fixture("logs/get_internal_logs.json");

    // Mock create job
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&create_job_fixture))
        .mount(&mock_server)
        .await;

    // Mock job status check
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-logs-sid-client"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&job_status_fixture))
        .mount(&mock_server)
        .await;

    // Mock get results
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-logs-sid-client/results"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&results_fixture))
        .mount(&mock_server)
        .await;

    use secrecy::SecretString;
    use splunk_client::{AuthStrategy, SplunkClient};

    let strategy = AuthStrategy::ApiToken {
        token: SecretString::new("test-token".to_string().into()),
    };

    let mut client = SplunkClient::builder()
        .base_url(mock_server.uri())
        .auth_strategy(strategy)
        .build()
        .unwrap();

    let result = client.get_internal_logs(10, None).await;

    assert!(result.is_ok());
    let logs = result.unwrap();
    assert_eq!(logs.len(), 2);
    assert_eq!(logs[0].level, "INFO");
    assert_eq!(logs[1].level, "WARN");
}
