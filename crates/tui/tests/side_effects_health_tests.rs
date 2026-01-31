//! Health side effect handler tests.
//!
//! This module tests the LoadHealth side effect handler which aggregates
//! health information from multiple Splunk REST API endpoints.

mod common;

use common::*;
use wiremock::matchers::{method, path};

#[tokio::test]
async fn test_load_health_all_healthy() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Mock all health endpoints
    let server_info = load_fixture("server/get_server_info.json");
    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&server_info))
        .mount(&harness.mock_server)
        .await;

    let health = load_fixture("server/get_health.json");
    Mock::given(method("GET"))
        .and(path("/services/server/health/splunkd"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&health))
        .mount(&harness.mock_server)
        .await;

    let license = load_fixture("license/get_usage.json");
    Mock::given(method("GET"))
        .and(path("/services/licenser/usage"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&license))
        .mount(&harness.mock_server)
        .await;

    let kvstore = load_fixture("kvstore/status.json");
    Mock::given(method("GET"))
        .and(path("/services/kvstore/status"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&kvstore))
        .mount(&harness.mock_server)
        .await;

    // Log parsing health uses a search
    let create_job_response = serde_json::json!({
        "sid": "test-sid"
    });
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(201).set_body_json(&create_job_response))
        .mount(&harness.mock_server)
        .await;

    let job_status = serde_json::json!({
        "entry": [{
            "content": {
                "sid": "test-sid",
                "isDone": true,
                "doneProgress": 1.0,
                "runDuration": 0.5,
                "scanCount": 0,
                "eventCount": 0,
                "resultCount": 0,
                "diskUsage": 0
            }
        }]
    });
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&job_status))
        .mount(&harness.mock_server)
        .await;

    let search_results = serde_json::json!({
        "results": [],
        "total": 0
    });
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&search_results))
        .mount(&harness.mock_server)
        .await;

    let actions = harness.handle_and_collect(Action::LoadHealth, 5).await;

    assert!(
        actions.iter().any(|a| matches!(a, Action::Loading(true))),
        "Should send Loading(true)"
    );
    // HealthLoaded contains a Box, so we match differently
    let has_health_loaded = actions.iter().any(|a| {
        if let Action::HealthLoaded(result) = a {
            result.is_ok()
        } else {
            false
        }
    });
    assert!(has_health_loaded, "Should send HealthLoaded(Ok)");
}

#[tokio::test]
async fn test_load_health_partial_failure() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Server info succeeds
    let server_info = load_fixture("server/get_server_info.json");
    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&server_info))
        .mount(&harness.mock_server)
        .await;

    // Health endpoint fails
    Mock::given(method("GET"))
        .and(path("/services/server/health"))
        .respond_with(ResponseTemplate::new(503).set_body_string("Service Unavailable"))
        .mount(&harness.mock_server)
        .await;

    // License succeeds
    let license = load_fixture("license/get_usage.json");
    Mock::given(method("GET"))
        .and(path("/services/licenser/usage"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&license))
        .mount(&harness.mock_server)
        .await;

    // KVStore succeeds
    let kvstore = load_fixture("kvstore/status.json");
    Mock::given(method("GET"))
        .and(path("/services/kvstore/status"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&kvstore))
        .mount(&harness.mock_server)
        .await;

    // Log parsing health uses a search
    let create_job_response = serde_json::json!({
        "sid": "test-sid"
    });
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(201).set_body_json(&create_job_response))
        .mount(&harness.mock_server)
        .await;

    let job_status = serde_json::json!({
        "entry": [{
            "content": {
                "sid": "test-sid",
                "isDone": true,
                "doneProgress": 1.0,
                "runDuration": 0.5,
                "scanCount": 0,
                "eventCount": 0,
                "resultCount": 0,
                "diskUsage": 0
            }
        }]
    });
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&job_status))
        .mount(&harness.mock_server)
        .await;

    let search_results = serde_json::json!({
        "results": [],
        "total": 0
    });
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&search_results))
        .mount(&harness.mock_server)
        .await;

    let actions = harness.handle_and_collect(Action::LoadHealth, 3).await;

    // Should return error because one endpoint failed
    let has_health_error = actions.iter().any(|a| {
        if let Action::HealthLoaded(result) = a {
            result.is_err()
        } else {
            false
        }
    });
    assert!(
        has_health_error,
        "Should send HealthLoaded(Err) when one endpoint fails"
    );
}
