//! Search side effect handler tests.
//!
//! This module tests search-related side effect handlers including
//! RunSearch, LoadMoreSearchResults, LoadInternalLogs, and LoadSavedSearches.

mod common;

use common::*;
use wiremock::matchers::{method, path};

#[tokio::test]
async fn test_run_search_success() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Mock job creation - return simple SID format
    let create_job_response = serde_json::json!({
        "sid": "test-sid"
    });
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(201).set_body_json(&create_job_response))
        .mount(&harness.mock_server)
        .await;

    // Mock job status polling - needs proper SearchJobStatus fields
    let job_status = serde_json::json!({
        "entry": [{
            "content": {
                "sid": "test-sid",
                "isDone": true,
                "doneProgress": 1.0,
                "runDuration": 0.5,
                "scanCount": 100,
                "eventCount": 100,
                "resultCount": 100,
                "diskUsage": 0
            }
        }]
    });
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&job_status))
        .mount(&harness.mock_server)
        .await;

    // Mock search results
    let results_fixture = load_fixture("search/get_results.json");
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&results_fixture))
        .mount(&harness.mock_server)
        .await;

    let search_defaults = splunk_config::SearchDefaults {
        earliest_time: "-24h".to_string(),
        latest_time: "now".to_string(),
        max_results: 100,
    };

    let actions = harness
        .handle_and_collect(
            Action::RunSearch {
                query: "index=main | head 10".to_string(),
                search_defaults,
            },
            4, // Expect 4 actions: Loading(true), Progress, SearchStarted, SearchComplete
        )
        .await;

    assert!(
        actions.iter().any(|a| matches!(a, Action::Loading(true))),
        "Should send Loading(true)"
    );
    assert!(
        actions.iter().any(|a| matches!(a, Action::Progress(_))),
        "Should send Progress"
    );
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::SearchStarted(_))),
        "Should send SearchStarted"
    );
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::SearchComplete(Ok(_)))),
        "Should send SearchComplete(Ok)"
    );
}

#[tokio::test]
async fn test_run_search_error() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Mock job creation failure
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(400).set_body_string("Invalid search query"))
        .mount(&harness.mock_server)
        .await;

    let search_defaults = splunk_config::SearchDefaults {
        earliest_time: "-24h".to_string(),
        latest_time: "now".to_string(),
        max_results: 100,
    };

    let actions = harness
        .handle_and_collect(
            Action::RunSearch {
                query: "invalid query".to_string(),
                search_defaults,
            },
            2,
        )
        .await;

    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::SearchComplete(Err(_)))),
        "Should send SearchComplete(Err)"
    );
}

#[tokio::test]
async fn test_load_more_search_results_success() {
    let mut harness = SideEffectsTestHarness::new().await;

    let results_fixture = load_fixture("search/get_results.json");
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&results_fixture))
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(
            Action::LoadMoreSearchResults {
                sid: "test-sid".to_string(),
                offset: 100,
                count: 100,
            },
            2,
        )
        .await;

    assert!(
        actions.iter().any(|a| matches!(a, Action::Loading(true))),
        "Should send Loading(true)"
    );
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::MoreSearchResultsLoaded(Ok(_)))),
        "Should send MoreSearchResultsLoaded(Ok)"
    );
}

#[tokio::test]
async fn test_load_more_search_results_error() {
    let mut harness = SideEffectsTestHarness::new().await;

    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .respond_with(ResponseTemplate::new(404).set_body_string("Job not found"))
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(
            Action::LoadMoreSearchResults {
                sid: "test-sid".to_string(),
                offset: 100,
                count: 100,
            },
            2,
        )
        .await;

    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::MoreSearchResultsLoaded(Err(_)))),
        "Should send MoreSearchResultsLoaded(Err)"
    );
}

#[tokio::test]
async fn test_load_internal_logs_success() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Internal logs uses a search job, so we need to mock job creation and results
    // Use a simple SID response that create_job can parse
    let create_job_response = serde_json::json!({
        "sid": "test-sid"
    });
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(201).set_body_json(&create_job_response))
        .mount(&harness.mock_server)
        .await;

    // Mock job status polling - needs proper SearchJobStatus fields
    let job_status = serde_json::json!({
        "entry": [{
            "content": {
                "sid": "test-sid",
                "isDone": true,
                "doneProgress": 1.0,
                "runDuration": 0.5,
                "scanCount": 100,
                "eventCount": 100,
                "resultCount": 100,
                "diskUsage": 0
            }
        }]
    });
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&job_status))
        .mount(&harness.mock_server)
        .await;

    // Mock search results
    let results_fixture = load_fixture("search/get_results.json");
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&results_fixture))
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(
            Action::LoadInternalLogs {
                count: 100,
                earliest: "-15m".to_string(),
            },
            3,
        )
        .await;

    assert!(
        actions.iter().any(|a| matches!(a, Action::Loading(true))),
        "Should send Loading(true)"
    );
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::InternalLogsLoaded(Ok(_)))),
        "Should send InternalLogsLoaded(Ok)"
    );
}

#[tokio::test]
async fn test_load_saved_searches_success() {
    let mut harness = SideEffectsTestHarness::new().await;

    let fixture = load_fixture("search/list_saved_searches.json");
    Mock::given(method("GET"))
        .and(path("/services/saved/searches"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(Action::LoadSavedSearches, 2)
        .await;

    assert!(
        actions.iter().any(|a| matches!(a, Action::Loading(true))),
        "Should send Loading(true)"
    );
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::SavedSearchesLoaded(Ok(_)))),
        "Should send SavedSearchesLoaded(Ok)"
    );

    let searches = actions
        .iter()
        .find_map(|a| match a {
            Action::SavedSearchesLoaded(Ok(searches)) => Some(searches),
            _ => None,
        })
        .expect("Should have SavedSearchesLoaded action");

    assert_eq!(searches.len(), 2);
}
