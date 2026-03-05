//! Tests for job management action redaction.

use splunk_client::models::SearchJobStatus;

use crate::action::tests::redacted_debug;
use crate::action::variants::Action;

#[test]
fn test_show_cancel_job_sid() {
    let action = Action::CancelJob("search_job_12345_789".to_string());
    let output = redacted_debug(&action);

    assert!(output.contains("CancelJob"), "Should contain action name");
    assert!(
        output.contains("search_job_12345_789"),
        "Should show SID for debugging"
    );
}

#[test]
fn test_show_delete_job_sid() {
    let action = Action::DeleteJob("search_job_98765_4321".to_string());
    let output = redacted_debug(&action);

    assert!(output.contains("DeleteJob"), "Should contain action name");
    assert!(
        output.contains("search_job_98765_4321"),
        "Should show SID for debugging"
    );
}

#[test]
fn test_show_batch_operation_counts() {
    let sids = vec!["job1".to_string(), "job2".to_string(), "job3".to_string()];
    let action = Action::CancelJobsBatch(sids);
    let output = redacted_debug(&action);

    assert!(
        output.contains("CancelJobsBatch"),
        "Should contain action name"
    );
    assert!(
        output.contains("3 job(s)"),
        "Should show count but not SIDs"
    );
    assert!(!output.contains("job1"), "Should not show individual SIDs");
}

#[test]
fn test_redact_jobs_loaded() {
    let jobs = vec![
        SearchJobStatus {
            sid: "job1".to_string(),
            is_done: true,
            is_finalized: true,
            done_progress: 1.0,
            run_duration: 1.0,
            cursor_time: None,
            scan_count: 100,
            event_count: 50,
            result_count: 25,
            disk_usage: 1024,
            priority: None,
            label: None,
        },
        SearchJobStatus {
            sid: "job2".to_string(),
            is_done: false,
            is_finalized: false,
            done_progress: 0.5,
            run_duration: 0.5,
            cursor_time: None,
            scan_count: 50,
            event_count: 25,
            result_count: 10,
            disk_usage: 512,
            priority: None,
            label: None,
        },
    ];
    let action = Action::JobsLoaded(Ok(jobs));
    let output = redacted_debug(&action);

    assert!(!output.contains("job1"), "Should not contain job SID");
    assert!(!output.contains("job2"), "Should not contain job SID");
    assert!(output.contains("JobsLoaded"), "Should contain action name");
    assert!(output.contains("2 items"), "Should show item count");
}
