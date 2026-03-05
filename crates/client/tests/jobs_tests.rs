//! Job management endpoint tests.
//!
//! This module tests the Splunk search job lifecycle, including:
//! - Creating search jobs with various response formats
//! - Getting job status and progress
//! - Listing all jobs
//! - Canceling and deleting jobs
//!
//! # Invariants
//! - Job creation returns a valid SID (search ID)
//! - Job status reflects completion state and progress metrics
//! - Job cancellation and deletion return success for valid SIDs
//!
//! # What this does NOT handle
//! - Search result retrieval (see search_tests.rs)
//! - Log parsing health checks (see parsing_tests.rs)

mod common;

use common::*;
use wiremock::matchers::{method, path, query_param};

#[tokio::test]
async fn test_create_search_job() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/create_job_success.json");

    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let options = endpoints::CreateJobOptions {
        wait: Some(false),
        exec_time: Some(60),
        ..Default::default()
    };

    let result = endpoints::create_job(
        &client,
        &mock_server.uri(),
        "test-token",
        "search index=main",
        &options,
        3,
        None,
        None,
    )
    .await;

    assert!(result.is_ok());
    let sid = result.unwrap();
    assert!(sid.contains("scheduler__admin__search"));
}

#[tokio::test]
async fn test_create_search_job_sid_only_response() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/create_job_success_sid_only.json");

    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let options = endpoints::CreateJobOptions {
        wait: Some(false),
        exec_time: Some(60),
        ..Default::default()
    };

    let result = endpoints::create_job(
        &client,
        &mock_server.uri(),
        "test-token",
        "search index=main",
        &options,
        3,
        None,
        None,
    )
    .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "1769478517.49");
}

#[tokio::test]
async fn test_get_job_status() {
    let mock_server = MockServer::start().await;

    let fixture = serde_json::json!({
        "entry": [{
            "content": {
                "sid": "test-sid-123",
                "isDone": true,
                "isFinalized": false,
                "doneProgress": 1.0,
                "runDuration": 5.5,
                "cursorTime": "2024-01-15T10:30:00.000-05:00",
                "scanCount": 1000,
                "eventCount": 500,
                "resultCount": 250,
                "diskUsage": 1024
            }
        }]
    });

    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid-123"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::get_job_status(
        &client,
        &mock_server.uri(),
        "test-token",
        "test-sid-123",
        3,
        None,
        None,
    )
    .await;

    assert!(result.is_ok());
    let status = result.unwrap();
    assert_eq!(status.sid, "test-sid-123");
    assert!(status.is_done);
    assert_eq!(status.result_count, 250);
}

#[tokio::test]
async fn test_list_jobs() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("jobs/list_jobs.json");

    Mock::given(method("GET"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::list_jobs(
        &client,
        &mock_server.uri(),
        "test-token",
        Some(10),
        Some(0),
        3,
        None,
        None,
    )
    .await;

    assert!(result.is_ok());
    let jobs = result.unwrap();
    assert_eq!(jobs.len(), 2);
    assert!(!jobs[0].is_done);
    assert!(jobs[1].is_done);

    // Verify field preservation from fixture
    assert_eq!(jobs[0].disk_usage, 1024);
    assert_eq!(jobs[0].priority, Some(5));
    assert_eq!(jobs[0].label, Some("Test Job 1".to_string()));

    assert_eq!(jobs[1].disk_usage, 2048);
    assert_eq!(jobs[1].priority, Some(3));
    assert_eq!(jobs[1].label, None);
}

#[tokio::test]
async fn test_cancel_job() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/services/search/jobs/test-sid/control"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::cancel_job(
        &client,
        &mock_server.uri(),
        "test-token",
        "test-sid",
        3,
        None,
        None,
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_delete_job() {
    let mock_server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/services/search/jobs/test-sid"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::delete_job(
        &client,
        &mock_server.uri(),
        "test-token",
        "test-sid",
        3,
        None,
        None,
    )
    .await;

    assert!(result.is_ok());
}
