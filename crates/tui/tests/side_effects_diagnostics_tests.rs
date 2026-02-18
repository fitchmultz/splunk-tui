//! Connection diagnostics side effect handler tests.
//!
//! This module tests the RunConnectionDiagnostics side effect handler which
//! performs comprehensive connection diagnostics (reachability, auth, TLS, server info).

mod common;

use common::*;
use splunk_tui::action::variants::DiagnosticStatus;
use wiremock::matchers::{method, path};

#[tokio::test]
async fn test_diagnostics_all_healthy() {
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

    let actions = harness
        .handle_and_collect(Action::RunConnectionDiagnostics, 5)
        .await;

    assert!(
        actions.iter().any(|a| matches!(a, Action::Loading(true))),
        "Should send Loading(true)"
    );

    let has_diagnostics = actions.iter().any(|a| {
        if let Action::ConnectionDiagnosticsLoaded(Ok(result)) = a {
            result.overall_status == DiagnosticStatus::Pass
        } else {
            false
        }
    });
    assert!(
        has_diagnostics,
        "Should send ConnectionDiagnosticsLoaded(Ok) with Pass status"
    );
}

#[tokio::test]
async fn test_diagnostics_connection_refused() {
    let mut harness = SideEffectsTestHarness::new().await;

    // No mocks mounted - all requests will fail with connection error

    let actions = harness
        .handle_and_collect(Action::RunConnectionDiagnostics, 5)
        .await;

    let has_diagnostics = actions.iter().any(|a| {
        if let Action::ConnectionDiagnosticsLoaded(Ok(result)) = a {
            result.overall_status == DiagnosticStatus::Fail
                && result.reachable.status == DiagnosticStatus::Fail
        } else {
            false
        }
    });
    assert!(
        has_diagnostics,
        "Should send ConnectionDiagnosticsLoaded with Fail status when connection fails"
    );
}

#[tokio::test]
async fn test_diagnostics_auth_failure() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Server info returns 401 Unauthorized
    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(Action::RunConnectionDiagnostics, 5)
        .await;

    let has_diagnostics = actions.iter().any(|a| {
        if let Action::ConnectionDiagnosticsLoaded(Ok(result)) = a {
            result.overall_status == DiagnosticStatus::Fail
                && result.auth.status == DiagnosticStatus::Fail
        } else {
            false
        }
    });
    assert!(
        has_diagnostics,
        "Should send ConnectionDiagnosticsLoaded with auth failure"
    );
}

#[tokio::test]
async fn test_diagnostics_server_info_succeeds_passes_checks() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Server info succeeds (required for overall success)
    let server_info = load_fixture("server/get_server_info.json");
    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&server_info))
        .mount(&harness.mock_server)
        .await;

    // Other endpoints may fail, but that's OK for this test
    Mock::given(method("GET"))
        .and(path("/services/server/health/splunkd"))
        .respond_with(ResponseTemplate::new(503).set_body_string("Service Unavailable"))
        .mount(&harness.mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/services/licenser/usage"))
        .respond_with(ResponseTemplate::new(503).set_body_string("Service Unavailable"))
        .mount(&harness.mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/services/kvstore/status"))
        .respond_with(ResponseTemplate::new(503).set_body_string("Service Unavailable"))
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(Action::RunConnectionDiagnostics, 10)
        .await;

    // Check that we got diagnostics
    let diagnostics = actions.iter().find_map(|a| {
        if let Action::ConnectionDiagnosticsLoaded(Ok(result)) = a {
            Some(result.clone())
        } else {
            None
        }
    });

    assert!(diagnostics.is_some(), "Should receive diagnostics result");

    let result = diagnostics.unwrap();
    assert_eq!(
        result.reachable.status,
        DiagnosticStatus::Pass,
        "Reachability should pass"
    );
    assert_eq!(
        result.auth.status,
        DiagnosticStatus::Pass,
        "Auth should pass"
    );
    assert_eq!(result.tls.status, DiagnosticStatus::Pass, "TLS should pass");
    assert!(result.server_info.is_some(), "Should have server info");
}

#[tokio::test]
async fn test_diagnostics_includes_server_info() {
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

    let actions = harness
        .handle_and_collect(Action::RunConnectionDiagnostics, 5)
        .await;

    let has_server_info = actions.iter().any(|a| {
        if let Action::ConnectionDiagnosticsLoaded(Ok(result)) = a {
            result.server_info.is_some()
        } else {
            false
        }
    });
    assert!(
        has_server_info,
        "Should include server_info in successful diagnostics"
    );
}
