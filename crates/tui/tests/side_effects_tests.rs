//! Async side effect handler tests.
//!
//! This module tests the TUI's async side effect handlers in `side_effects.rs`.
//! It uses wiremock to mock HTTP responses and verifies that handlers send
//! the correct actions back through the channel.
//!
//! # Test Coverage
//! - Data loading handlers (LoadIndexes, LoadJobs, LoadClusterInfo, etc.)
//! - Search operations (RunSearch, LoadMoreSearchResults)
//! - Job operations (CancelJob, DeleteJob, batch operations)
//! - App operations (EnableApp, DisableApp)
//! - Health check aggregation (LoadHealth)
//! - Profile operations (OpenProfileSwitcher, ProfileSelected)
//! - Other operations (SwitchToSettings, ExportData)
//!
//! # Invariants
//! - All handlers send `Loading(true)` before API calls
//! - All handlers send `Loading(false)` after completion (on error paths)
//! - Results are sent back via the action channel

mod common;

use common::*;
use wiremock::matchers::{method, path};

// ============ Data Loading Tests ============

#[tokio::test]
async fn test_load_indexes_success() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Mock the indexes endpoint
    let fixture = load_fixture("indexes/list_indexes.json");
    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&harness.mock_server)
        .await;

    // Handle the action
    let actions = harness
        .handle_and_collect(
            Action::LoadIndexes {
                count: 100,
                offset: 0,
            },
            2,
        )
        .await;

    // Verify actions sent
    assert!(
        actions.iter().any(|a| matches!(a, Action::Loading(true))),
        "Should send Loading(true)"
    );
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::IndexesLoaded(Ok(_)))),
        "Should send IndexesLoaded(Ok)"
    );

    // Verify the loaded data
    let indexes_loaded = actions
        .iter()
        .find_map(|a| match a {
            Action::IndexesLoaded(Ok(indexes)) => Some(indexes),
            _ => None,
        })
        .expect("Should have IndexesLoaded action");

    assert_eq!(indexes_loaded.len(), 3);
    assert_eq!(indexes_loaded[0].name, "main");
    assert_eq!(indexes_loaded[1].name, "_internal");
    assert_eq!(indexes_loaded[2].name, "_audit");
}

#[tokio::test]
async fn test_load_indexes_error() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Mock an error response
    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(
            Action::LoadIndexes {
                count: 100,
                offset: 0,
            },
            2,
        )
        .await;

    // Should still send Loading actions and error result
    assert!(
        actions.iter().any(|a| matches!(a, Action::Loading(true))),
        "Should send Loading(true)"
    );
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::IndexesLoaded(Err(_)))),
        "Should send IndexesLoaded(Err)"
    );
}

#[tokio::test]
async fn test_load_jobs_success() {
    let mut harness = SideEffectsTestHarness::new().await;

    let fixture = load_fixture("jobs/list_jobs.json");
    Mock::given(method("GET"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(
            Action::LoadJobs {
                count: 100,
                offset: 0,
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
            .any(|a| matches!(a, Action::JobsLoaded(Ok(_)))),
        "Should send JobsLoaded(Ok)"
    );

    let jobs = actions
        .iter()
        .find_map(|a| match a {
            Action::JobsLoaded(Ok(jobs)) => Some(jobs),
            _ => None,
        })
        .expect("Should have JobsLoaded action");

    assert_eq!(jobs.len(), 2);
}

#[tokio::test]
async fn test_load_jobs_error() {
    let mut harness = SideEffectsTestHarness::new().await;

    Mock::given(method("GET"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(403).set_body_string("Forbidden"))
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(
            Action::LoadJobs {
                count: 100,
                offset: 0,
            },
            2,
        )
        .await;

    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::JobsLoaded(Err(_)))),
        "Should send JobsLoaded(Err)"
    );
}

#[tokio::test]
async fn test_load_cluster_info_success() {
    let mut harness = SideEffectsTestHarness::new().await;

    let fixture = load_fixture("cluster/get_cluster_info.json");
    Mock::given(method("GET"))
        .and(path("/services/cluster/master/config"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&harness.mock_server)
        .await;

    let actions = harness.handle_and_collect(Action::LoadClusterInfo, 2).await;

    assert!(
        actions.iter().any(|a| matches!(a, Action::Loading(true))),
        "Should send Loading(true)"
    );
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::ClusterInfoLoaded(Ok(_)))),
        "Should send ClusterInfoLoaded(Ok)"
    );
}

#[tokio::test]
async fn test_load_cluster_info_error() {
    let mut harness = SideEffectsTestHarness::new().await;

    Mock::given(method("GET"))
        .and(path("/services/cluster/master/config"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&harness.mock_server)
        .await;

    let actions = harness.handle_and_collect(Action::LoadClusterInfo, 2).await;

    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::ClusterInfoLoaded(Err(_)))),
        "Should send ClusterInfoLoaded(Err)"
    );
}

#[tokio::test]
async fn test_load_cluster_peers_success() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Cluster peers uses the same fixture structure
    let fixture = serde_json::json!({
        "entry": [
            {
                "name": "peer1",
                "content": {
                    "id": "peer-01",
                    "label": "Peer 1",
                    "status": "Up"
                }
            }
        ]
    });

    Mock::given(method("GET"))
        .and(path("/services/cluster/master/peers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(Action::LoadClusterPeers, 2)
        .await;

    assert!(
        actions.iter().any(|a| matches!(a, Action::Loading(true))),
        "Should send Loading(true)"
    );
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::ClusterPeersLoaded(Ok(_)))),
        "Should send ClusterPeersLoaded(Ok)"
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
async fn test_load_apps_success() {
    let mut harness = SideEffectsTestHarness::new().await;

    let fixture = serde_json::json!({
        "entry": [
            {
                "name": "search",
                "content": {
                    "label": "Search & Reporting",
                    "version": "1.0",
                    "disabled": false
                }
            },
            {
                "name": "launcher",
                "content": {
                    "label": "Home",
                    "version": "1.0",
                    "disabled": false
                }
            }
        ]
    });

    Mock::given(method("GET"))
        .and(path("/services/apps/local"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(
            Action::LoadApps {
                count: 100,
                offset: 0,
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
            .any(|a| matches!(a, Action::AppsLoaded(Ok(_)))),
        "Should send AppsLoaded(Ok)"
    );
}

#[tokio::test]
async fn test_load_users_success() {
    let mut harness = SideEffectsTestHarness::new().await;

    let fixture = serde_json::json!({
        "entry": [
            {
                "name": "admin",
                "content": {
                    "realname": "Administrator",
                    "email": "admin@example.com",
                    "roles": ["admin"]
                }
            }
        ]
    });

    Mock::given(method("GET"))
        .and(path("/services/authentication/users"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(
            Action::LoadUsers {
                count: 100,
                offset: 0,
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
            .any(|a| matches!(a, Action::UsersLoaded(Ok(_)))),
        "Should send UsersLoaded(Ok)"
    );
}

// ============ Search Operation Tests ============

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

// ============ Job Operation Tests ============

#[tokio::test]
async fn test_cancel_job_success() {
    let mut harness = SideEffectsTestHarness::new().await;

    let fixture = load_fixture("jobs/cancel_job_success.json");
    Mock::given(method("POST"))
        .and(path("/services/search/jobs/test-job-sid/control"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&harness.mock_server)
        .await;

    // Also need to mock the job list reload
    let jobs_fixture = load_fixture("jobs/list_jobs.json");
    Mock::given(method("GET"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&jobs_fixture))
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(Action::CancelJob("test-job-sid".to_string()), 2)
        .await;

    assert!(
        actions.iter().any(|a| matches!(a, Action::Loading(true))),
        "Should send Loading(true)"
    );
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::JobOperationComplete(_))),
        "Should send JobOperationComplete"
    );
    assert!(
        actions.iter().any(|a| matches!(a, Action::LoadJobs { .. })),
        "Should send LoadJobs to refresh"
    );
}

#[tokio::test]
async fn test_cancel_job_error() {
    let mut harness = SideEffectsTestHarness::new().await;

    Mock::given(method("POST"))
        .and(path("/services/search/jobs/test-job-sid/control"))
        .respond_with(ResponseTemplate::new(404).set_body_string("Job not found"))
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(Action::CancelJob("test-job-sid".to_string()), 2)
        .await;

    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::Notify(splunk_tui::ui::ToastLevel::Error, _))),
        "Should send error notification"
    );
    assert!(
        actions.iter().any(|a| matches!(a, Action::Loading(false))),
        "Should send Loading(false) on error"
    );
}

#[tokio::test]
async fn test_delete_job_success() {
    let mut harness = SideEffectsTestHarness::new().await;

    Mock::given(method("DELETE"))
        .and(path("/services/search/jobs/test-job-sid"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&harness.mock_server)
        .await;

    // Also need to mock the job list reload
    let jobs_fixture = load_fixture("jobs/list_jobs.json");
    Mock::given(method("GET"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&jobs_fixture))
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(Action::DeleteJob("test-job-sid".to_string()), 2)
        .await;

    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::JobOperationComplete(_))),
        "Should send JobOperationComplete"
    );
    assert!(
        actions.iter().any(|a| matches!(a, Action::LoadJobs { .. })),
        "Should send LoadJobs to refresh"
    );
}

#[tokio::test]
async fn test_delete_job_error() {
    let mut harness = SideEffectsTestHarness::new().await;

    Mock::given(method("DELETE"))
        .and(path("/services/search/jobs/test-job-sid"))
        .respond_with(ResponseTemplate::new(403).set_body_string("Permission denied"))
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(Action::DeleteJob("test-job-sid".to_string()), 2)
        .await;

    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::Notify(splunk_tui::ui::ToastLevel::Error, _))),
        "Should send error notification"
    );
    assert!(
        actions.iter().any(|a| matches!(a, Action::Loading(false))),
        "Should send Loading(false) on error"
    );
}

#[tokio::test]
async fn test_cancel_jobs_batch_all_success() {
    let mut harness = SideEffectsTestHarness::new().await;

    let fixture = load_fixture("jobs/cancel_job_success.json");
    Mock::given(method("POST"))
        .and(path("/services/search/jobs/job1/control"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&harness.mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/services/search/jobs/job2/control"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&harness.mock_server)
        .await;

    // Mock job list reload
    let jobs_fixture = load_fixture("jobs/list_jobs.json");
    Mock::given(method("GET"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&jobs_fixture))
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(
            Action::CancelJobsBatch(vec!["job1".to_string(), "job2".to_string()]),
            2,
        )
        .await;

    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::JobOperationComplete(msg) if msg.contains("2"))),
        "Should report 2 jobs cancelled"
    );
    assert!(
        actions.iter().any(|a| matches!(a, Action::LoadJobs { .. })),
        "Should send LoadJobs to refresh"
    );
}

#[tokio::test]
async fn test_cancel_jobs_batch_partial_failure() {
    let mut harness = SideEffectsTestHarness::new().await;

    let fixture = load_fixture("jobs/cancel_job_success.json");
    Mock::given(method("POST"))
        .and(path("/services/search/jobs/job1/control"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&harness.mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/services/search/jobs/job2/control"))
        .respond_with(ResponseTemplate::new(404).set_body_string("Job not found"))
        .mount(&harness.mock_server)
        .await;

    // Mock job list reload
    let jobs_fixture = load_fixture("jobs/list_jobs.json");
    Mock::given(method("GET"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&jobs_fixture))
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(
            Action::CancelJobsBatch(vec!["job1".to_string(), "job2".to_string()]),
            2,
        )
        .await;

    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::JobOperationComplete(msg) if msg.contains("1"))),
        "Should report 1 job cancelled"
    );
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::Notify(splunk_tui::ui::ToastLevel::Error, _))),
        "Should send error notification for failed job"
    );
}

#[tokio::test]
async fn test_delete_jobs_batch_all_success() {
    let mut harness = SideEffectsTestHarness::new().await;

    Mock::given(method("DELETE"))
        .and(path("/services/search/jobs/job1"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&harness.mock_server)
        .await;

    Mock::given(method("DELETE"))
        .and(path("/services/search/jobs/job2"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&harness.mock_server)
        .await;

    // Mock job list reload
    let jobs_fixture = load_fixture("jobs/list_jobs.json");
    Mock::given(method("GET"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&jobs_fixture))
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(
            Action::DeleteJobsBatch(vec!["job1".to_string(), "job2".to_string()]),
            2,
        )
        .await;

    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::JobOperationComplete(msg) if msg.contains("2"))),
        "Should report 2 jobs deleted"
    );
}

#[tokio::test]
async fn test_delete_jobs_batch_all_failure() {
    let mut harness = SideEffectsTestHarness::new().await;

    Mock::given(method("DELETE"))
        .and(path("/services/search/jobs/job1"))
        .respond_with(ResponseTemplate::new(403).set_body_string("Permission denied"))
        .mount(&harness.mock_server)
        .await;

    Mock::given(method("DELETE"))
        .and(path("/services/search/jobs/job2"))
        .respond_with(ResponseTemplate::new(403).set_body_string("Permission denied"))
        .mount(&harness.mock_server)
        .await;

    // Mock job list reload
    let jobs_fixture = load_fixture("jobs/list_jobs.json");
    Mock::given(method("GET"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&jobs_fixture))
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(
            Action::DeleteJobsBatch(vec!["job1".to_string(), "job2".to_string()]),
            2,
        )
        .await;

    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::JobOperationComplete(msg) if msg.contains("No jobs"))),
        "Should report no jobs deleted"
    );
    // Should have 2 error notifications
    let error_count = actions
        .iter()
        .filter(|a| matches!(a, Action::Notify(splunk_tui::ui::ToastLevel::Error, _)))
        .count();
    assert_eq!(error_count, 2, "Should send 2 error notifications");
}

// ============ App Operation Tests ============

#[tokio::test]
async fn test_enable_app_success() {
    let mut harness = SideEffectsTestHarness::new().await;

    Mock::given(method("POST"))
        .and(path("/services/apps/local/test-app/enable"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&harness.mock_server)
        .await;

    // Mock apps list reload
    let apps_fixture = serde_json::json!({
        "entry": [{"name": "test-app", "content": {"disabled": false}}]
    });
    Mock::given(method("GET"))
        .and(path("/services/apps/local"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&apps_fixture))
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(Action::EnableApp("test-app".to_string()), 2)
        .await;

    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::Notify(splunk_tui::ui::ToastLevel::Success, _))),
        "Should send success notification"
    );
    assert!(
        actions.iter().any(|a| matches!(a, Action::LoadApps { .. })),
        "Should send LoadApps to refresh"
    );
}

#[tokio::test]
async fn test_enable_app_error() {
    let mut harness = SideEffectsTestHarness::new().await;

    Mock::given(method("POST"))
        .and(path("/services/apps/local/test-app/enable"))
        .respond_with(ResponseTemplate::new(404).set_body_string("App not found"))
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(Action::EnableApp("test-app".to_string()), 2)
        .await;

    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::Notify(splunk_tui::ui::ToastLevel::Error, _))),
        "Should send error notification"
    );
    assert!(
        actions.iter().any(|a| matches!(a, Action::Loading(false))),
        "Should send Loading(false) on error"
    );
}

#[tokio::test]
async fn test_disable_app_success() {
    let mut harness = SideEffectsTestHarness::new().await;

    Mock::given(method("POST"))
        .and(path("/services/apps/local/test-app/disable"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&harness.mock_server)
        .await;

    // Mock apps list reload
    let apps_fixture = serde_json::json!({
        "entry": [{"name": "test-app", "content": {"disabled": true}}]
    });
    Mock::given(method("GET"))
        .and(path("/services/apps/local"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&apps_fixture))
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(Action::DisableApp("test-app".to_string()), 2)
        .await;

    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::Notify(splunk_tui::ui::ToastLevel::Success, _))),
        "Should send success notification"
    );
    assert!(
        actions.iter().any(|a| matches!(a, Action::LoadApps { .. })),
        "Should send LoadApps to refresh"
    );
}

#[tokio::test]
async fn test_disable_app_error() {
    let mut harness = SideEffectsTestHarness::new().await;

    Mock::given(method("POST"))
        .and(path("/services/apps/local/test-app/disable"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Server error"))
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(Action::DisableApp("test-app".to_string()), 2)
        .await;

    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::Notify(splunk_tui::ui::ToastLevel::Error, _))),
        "Should send error notification"
    );
    assert!(
        actions.iter().any(|a| matches!(a, Action::Loading(false))),
        "Should send Loading(false) on error"
    );
}

// ============ Health Check Tests ============

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

// ============ Profile Operation Tests ============

#[tokio::test]
async fn test_open_profile_switcher_with_profiles() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Add a profile to the config using ConfigManager's save_profile method
    {
        let mut cm = harness.config_manager.lock().await;
        let profile = splunk_config::ProfileConfig {
            base_url: Some("https://test.example.com:8089".to_string()),
            username: Some("admin".to_string()),
            password: Some(splunk_config::SecureValue::Plain(
                secrecy::SecretString::new("password".to_string().into()),
            )),
            api_token: None,
            skip_verify: Some(true),
            timeout_seconds: Some(30),
            max_retries: Some(3),
            session_expiry_buffer_seconds: Some(60),
            session_ttl_seconds: Some(3600),
            health_check_interval_seconds: Some(60),
        };
        cm.save_profile("test-profile", profile)
            .expect("Failed to save profile");
    }

    let actions = harness
        .handle_and_collect(Action::OpenProfileSwitcher, 1)
        .await;

    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::OpenProfileSelectorWithList(profiles) if profiles.contains(&"test-profile".to_string()))),
        "Should send OpenProfileSelectorWithList with test-profile"
    );
}

#[tokio::test]
async fn test_open_profile_switcher_no_profiles() {
    let mut harness = SideEffectsTestHarness::new().await;

    // No profiles added to config
    let actions = harness
        .handle_and_collect(Action::OpenProfileSwitcher, 1)
        .await;

    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::Notify(splunk_tui::ui::ToastLevel::Error, _))),
        "Should send error notification when no profiles configured"
    );
}

#[tokio::test]
async fn test_switch_to_settings() {
    let mut harness = SideEffectsTestHarness::new().await;

    let actions = harness
        .handle_and_collect(Action::SwitchToSettings, 1)
        .await;

    assert!(
        actions.iter().any(|a| matches!(a, Action::Loading(true))),
        "Should send Loading(true)"
    );
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::SettingsLoaded(_))),
        "Should send SettingsLoaded"
    );
}

// ============ Export Tests ============

#[tokio::test]
async fn test_export_data_success() {
    let mut harness = SideEffectsTestHarness::new().await;

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let export_path = temp_dir.path().join("export.json");

    let data = serde_json::json!([{"name": "test", "value": 123}]);

    let actions = harness
        .handle_and_collect(
            Action::ExportData(
                data,
                export_path.clone(),
                splunk_tui::action::ExportFormat::Json,
            ),
            1,
        )
        .await;

    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::Notify(splunk_tui::ui::ToastLevel::Info, _))),
        "Should send info notification on success"
    );

    // Verify file was created
    assert!(export_path.exists(), "Export file should exist");
}

#[tokio::test]
async fn test_export_data_error() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Use an invalid path (directory that doesn't exist and can't be created)
    let export_path = std::path::PathBuf::from("/nonexistent/directory/export.json");

    let data = serde_json::json!([{"name": "test"}]);

    let actions = harness
        .handle_and_collect(
            Action::ExportData(
                data,
                export_path.clone(),
                splunk_tui::action::ExportFormat::Json,
            ),
            1,
        )
        .await;

    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::Notify(splunk_tui::ui::ToastLevel::Error, _))),
        "Should send error notification on failure"
    );
}
