//! Jobs side effect handler tests.
//!
//! This module tests job-related side effect handlers including
//! LoadJobs, CancelJob, DeleteJob, and batch operations.

mod common;

use common::*;
use wiremock::matchers::{method, path};

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
