//! Log parsing health check endpoint tests.
//!
//! This module tests the Splunk log parsing health check API:
//! - Checking for parsing errors in internal logs
//! - Verifying health status when no errors exist
//! - SplunkClient interface for parsing health checks
//! - Session retry behavior during parsing health checks
//!
//! # Invariants
//! - Parsing health check returns error count and list of errors by component
//! - Empty results indicate healthy parsing state
//! - Session retry works correctly when authentication expires during check
//!
//! # What this does NOT handle
//! - Direct parsing of raw log files
//! - Configuration of parsing rules

mod common;

use common::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use wiremock::matchers::{method, path, query_param};

#[tokio::test]
async fn test_check_log_parsing_health() {
    let mock_server = MockServer::start().await;

    let create_job_fixture = serde_json::json!({
        "entry": [{
            "content": {
                "sid": "test-parsing-sid-123"
            }
        }]
    });

    let job_status_fixture = serde_json::json!({
        "entry": [{
            "content": {
                "sid": "test-parsing-sid-123",
                "isDone": true,
                "doneProgress": 1.0,
                "runDuration": 2.5,
                "scanCount": 100,
                "eventCount": 3,
                "resultCount": 3,
                "diskUsage": 512
            }
        }]
    });

    let results_fixture = load_fixture("parsing/check_health.json");

    // Mock create job
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&create_job_fixture))
        .mount(&mock_server)
        .await;

    // Mock job status check
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-parsing-sid-123"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&job_status_fixture))
        .mount(&mock_server)
        .await;

    // Mock get results
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-parsing-sid-123/results"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&results_fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::check_log_parsing_health(
        &client,
        &mock_server.uri(),
        "test-token",
        3,
        None,
        None,
    )
    .await;

    assert!(result.is_ok());
    let health = result.unwrap();
    assert!(!health.is_healthy);
    assert_eq!(health.total_errors, 3);
    assert_eq!(health.time_window, "-24h");
    assert_eq!(health.errors.len(), 3);
    assert_eq!(health.errors[0].component, "DateParserVerbose");
    assert_eq!(health.errors[1].component, "DateParserVerbose");
    assert_eq!(health.errors[2].component, "TuningParser");
}

#[tokio::test]
async fn test_check_log_parsing_health_no_errors() {
    let mock_server = MockServer::start().await;

    let create_job_fixture = serde_json::json!({
        "entry": [{
            "content": {
                "sid": "test-parsing-sid-empty"
            }
        }]
    });

    let job_status_fixture = serde_json::json!({
        "entry": [{
            "content": {
                "sid": "test-parsing-sid-empty",
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

    // Empty results
    let results_fixture: serde_json::Value = serde_json::json!([]);

    // Mock create job
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&create_job_fixture))
        .mount(&mock_server)
        .await;

    // Mock job status check
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-parsing-sid-empty"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&job_status_fixture))
        .mount(&mock_server)
        .await;

    // Mock get results
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-parsing-sid-empty/results"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&results_fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::check_log_parsing_health(
        &client,
        &mock_server.uri(),
        "test-token",
        3,
        None,
        None,
    )
    .await;

    assert!(result.is_ok());
    let health = result.unwrap();
    assert!(health.is_healthy);
    assert_eq!(health.total_errors, 0);
    assert_eq!(health.time_window, "-24h");
    assert!(health.errors.is_empty());
}

#[tokio::test]
async fn test_splunk_client_check_log_parsing_health() {
    let mock_server = MockServer::start().await;

    let create_job_fixture = serde_json::json!({
        "entry": [{
            "content": {
                "sid": "test-parsing-sid-client"
            }
        }]
    });

    let job_status_fixture = serde_json::json!({
        "entry": [{
            "content": {
                "sid": "test-parsing-sid-client",
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

    let results_fixture = load_fixture("parsing/check_health.json");

    // Mock create job
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&create_job_fixture))
        .mount(&mock_server)
        .await;

    // Mock job status check
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-parsing-sid-client"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&job_status_fixture))
        .mount(&mock_server)
        .await;

    // Mock get results
    Mock::given(method("GET"))
        .and(path(
            "/services/search/jobs/test-parsing-sid-client/results",
        ))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&results_fixture))
        .mount(&mock_server)
        .await;

    use secrecy::SecretString;
    use splunk_client::{AuthStrategy, SplunkClient};

    let strategy = AuthStrategy::ApiToken {
        token: SecretString::new("test-token".to_string().into()),
    };

    let client = SplunkClient::builder()
        .base_url(mock_server.uri())
        .auth_strategy(strategy)
        .build()
        .unwrap();

    let result = client.check_log_parsing_health().await;

    assert!(result.is_ok());
    let health = result.unwrap();
    assert!(!health.is_healthy);
    assert_eq!(health.total_errors, 3);
}

#[tokio::test]
async fn test_splunk_client_check_log_parsing_health_session_retry() {
    let mock_server = MockServer::start().await;

    let login_fixture = load_fixture("auth/login_success.json");

    let create_job_fixture = serde_json::json!({
        "entry": [{
            "content": {
                "sid": "test-parsing-sid-retry"
            }
        }]
    });

    let job_status_fixture = serde_json::json!({
        "entry": [{
            "content": {
                "sid": "test-parsing-sid-retry",
                "isDone": true,
                "doneProgress": 1.0,
                "runDuration": 2.0,
                "scanCount": 50,
                "eventCount": 1,
                "resultCount": 1,
                "diskUsage": 128
            }
        }]
    });

    let results_fixture = serde_json::json!([
        {
            "_time": "2025-01-20T10:30:00.000-05:00",
            "source": "/opt/splunk/var/log/splunk/metrics.log",
            "sourcetype": "splunkd",
            "message": "Failed to parse timestamp",
            "log_level": "ERROR",
            "component": "DateParserVerbose"
        }
    ]);

    // Track login requests
    let login_count = Arc::new(AtomicUsize::new(0));
    let login_count_clone = login_count.clone();

    // Mock login endpoint
    Mock::given(method("POST"))
        .and(path("/services/auth/login"))
        .and(query_param("output_mode", "json"))
        .respond_with(move |_: &wiremock::Request| {
            login_count_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(200).set_body_json(&login_fixture)
        })
        .mount(&mock_server)
        .await;

    // First create job call returns 401, second returns 200
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Session expired"}]
        })))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&create_job_fixture))
        .mount(&mock_server)
        .await;

    // Mock job status check
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-parsing-sid-retry"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&job_status_fixture))
        .mount(&mock_server)
        .await;

    // Mock get results
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-parsing-sid-retry/results"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&results_fixture))
        .mount(&mock_server)
        .await;

    use secrecy::SecretString;
    use splunk_client::{AuthStrategy, SplunkClient};

    let strategy = AuthStrategy::SessionToken {
        username: "admin".to_string(),
        password: SecretString::new("testpassword".to_string().into()),
    };

    let client = SplunkClient::builder()
        .base_url(mock_server.uri())
        .auth_strategy(strategy)
        .skip_verify(true)
        .build()
        .unwrap();

    // Initial login
    client.login().await.unwrap();
    assert_eq!(login_count.load(Ordering::SeqCst), 1);

    // This should trigger a retry with re-login
    let result = client.check_log_parsing_health().await;

    assert!(result.is_ok());
    let health = result.unwrap();
    assert_eq!(health.total_errors, 1);

    // Should have called login twice (initial + retry)
    assert_eq!(login_count.load(Ordering::SeqCst), 2);
}
