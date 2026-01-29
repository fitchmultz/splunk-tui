//! Tests for multi-selection and batch operations (RQ-0050).
//!
//! This module tests:
//! - Spacebar toggles job selection
//! - Multiple job selection
//! - Batch cancel popup with selection
//! - Batch delete popup with selection
//! - Batch cancel/delete action generation
//! - Single cancel/delete with no selection
//! - Selection cleared after job operation
//! - Selection persistence across jobs loaded
//! - Batch popup cancel with 'n' and Esc
//! - Batch confirm with Enter
//!
//! ## Invariants
//! - Spacebar must toggle selection for currently selected job
//! - Selection must persist across jobs loaded (tracked by SID)
//! - Selection must be cleared after JobOperationComplete
//! - Batch popups must show correct job count
//!
//! ## Test Organization
//! Tests are grouped by: selection, batch popups, actions, persistence.

mod helpers;
use helpers::*;
use splunk_client::models::SearchJobStatus;
use splunk_tui::{CurrentScreen, PopupType, action::Action, app::App, app::ConnectionContext};

fn create_mock_jobs(count: usize) -> Vec<SearchJobStatus> {
    (0..count)
        .map(|i| SearchJobStatus {
            sid: format!("sid_{}", i),
            is_done: i % 2 == 0,
            is_finalized: false,
            done_progress: 0.5,
            run_duration: 10.0,
            disk_usage: 1024,
            scan_count: 100,
            event_count: 50,
            result_count: 25,
            cursor_time: None,
            priority: None,
            label: None,
        })
        .collect()
}

#[test]
fn test_spacebar_toggles_selection() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(3))));
    app.jobs_state.select(Some(1));

    let sid = "sid_1";

    // Select job with spacebar
    let action = app.handle_input(key(' '));
    assert!(action.is_none(), "Spacebar should not return action");
    assert!(
        app.selected_jobs.contains(sid),
        "Job should be selected after pressing spacebar"
    );

    // Toggle off with spacebar
    let action = app.handle_input(key(' '));
    assert!(action.is_none(), "Spacebar should not return action");
    assert!(
        !app.selected_jobs.contains(sid),
        "Job should be deselected after pressing spacebar again"
    );
}

#[test]
fn test_multiple_jobs_selection() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(5))));

    // Select first job
    app.jobs_state.select(Some(0));
    app.handle_input(key(' '));
    assert!(app.selected_jobs.contains("sid_0"));

    // Select third job
    app.jobs_state.select(Some(2));
    app.handle_input(key(' '));
    assert!(app.selected_jobs.contains("sid_2"));

    // Select fifth job
    app.jobs_state.select(Some(4));
    app.handle_input(key(' '));
    assert!(app.selected_jobs.contains("sid_4"));

    // Verify all three jobs are selected
    assert_eq!(
        app.selected_jobs.len(),
        3,
        "Should have exactly 3 jobs selected"
    );
    assert!(app.selected_jobs.contains("sid_0"));
    assert!(app.selected_jobs.contains("sid_2"));
    assert!(app.selected_jobs.contains("sid_4"));
}

#[test]
fn test_batch_cancel_popup_with_selection() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(5))));

    // Select multiple jobs
    app.jobs_state.select(Some(0));
    app.handle_input(key(' '));
    app.jobs_state.select(Some(2));
    app.handle_input(key(' '));

    // Press 'c' to open batch cancel popup
    let action = app.handle_input(key('c'));
    assert!(action.is_none(), "Opening popup should not return action");
    assert!(app.popup.is_some(), "Popup should be open");

    // Verify it's a batch cancel popup with 2 jobs
    assert!(
        matches!(
            app.popup.as_ref().map(|p| &p.kind),
            Some(PopupType::ConfirmCancelBatch(sids)) if sids.len() == 2
        ),
        "Should be ConfirmCancelBatch with 2 SIDs"
    );
}

#[test]
fn test_batch_delete_popup_with_selection() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(5))));

    // Select multiple jobs
    app.jobs_state.select(Some(1));
    app.handle_input(key(' '));
    app.jobs_state.select(Some(3));
    app.handle_input(key(' '));

    // Press 'd' to open batch delete popup
    let action = app.handle_input(key('d'));
    assert!(action.is_none(), "Opening popup should not return action");
    assert!(app.popup.is_some(), "Popup should be open");

    // Verify it's a batch delete popup with 2 jobs
    assert!(
        matches!(
            app.popup.as_ref().map(|p| &p.kind),
            Some(PopupType::ConfirmDeleteBatch(sids)) if sids.len() == 2
        ),
        "Should be ConfirmDeleteBatch with 2 SIDs"
    );
}

#[test]
fn test_batch_cancel_action_generated() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(3))));

    // Select two jobs
    app.jobs_state.select(Some(0));
    app.handle_input(key(' '));
    app.jobs_state.select(Some(1));
    app.handle_input(key(' '));

    // Open batch cancel popup
    app.handle_input(key('c'));
    assert!(app.popup.is_some());

    // Press 'y' to confirm
    let action = app.handle_input(key('y'));
    assert!(action.is_some(), "Confirming should return action");

    // Verify it's CancelJobsBatch with correct SIDs
    assert!(
        matches!(
            action,
            Some(Action::CancelJobsBatch(sids)) if sids.len() == 2
        ),
        "Should be CancelJobsBatch with 2 SIDs"
    );
    assert!(app.popup.is_none(), "Popup should be closed after confirm");
}

#[test]
fn test_batch_delete_action_generated() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(3))));

    // Select two jobs
    app.jobs_state.select(Some(0));
    app.handle_input(key(' '));
    app.jobs_state.select(Some(2));
    app.handle_input(key(' '));

    // Open batch delete popup
    app.handle_input(key('d'));
    assert!(app.popup.is_some());

    // Press 'y' to confirm
    let action = app.handle_input(key('y'));
    assert!(action.is_some(), "Confirming should return action");

    // Verify it's DeleteJobsBatch with correct SIDs
    assert!(
        matches!(
            action,
            Some(Action::DeleteJobsBatch(sids)) if sids.len() == 2
        ),
        "Should be DeleteJobsBatch with 2 SIDs"
    );
    assert!(app.popup.is_none(), "Popup should be closed after confirm");
}

#[test]
fn test_single_cancel_with_no_selection() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(3))));
    app.jobs_state.select(Some(1));

    // No jobs selected, pressing 'c' should open single cancel popup
    let action = app.handle_input(key('c'));
    assert!(action.is_none(), "Opening popup should not return action");

    // Verify it's a single cancel popup (not batch)
    assert!(
        matches!(
            app.popup.as_ref().map(|p| &p.kind),
            Some(PopupType::ConfirmCancel(sid)) if sid == "sid_1"
        ),
        "Should be ConfirmCancel popup for single job"
    );
}

#[test]
fn test_single_delete_with_no_selection() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(3))));
    app.jobs_state.select(Some(2));

    // No jobs selected, pressing 'd' should open single delete popup
    let action = app.handle_input(key('d'));
    assert!(action.is_none(), "Opening popup should not return action");

    // Verify it's a single delete popup (not batch)
    assert!(
        matches!(
            app.popup.as_ref().map(|p| &p.kind),
            Some(PopupType::ConfirmDelete(sid)) if sid == "sid_2"
        ),
        "Should be ConfirmDelete popup for single job"
    );
}

#[test]
fn test_selection_cleared_after_job_operation() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(3))));

    // Select two jobs
    app.jobs_state.select(Some(0));
    app.handle_input(key(' '));
    app.jobs_state.select(Some(1));
    app.handle_input(key(' '));

    assert_eq!(app.selected_jobs.len(), 2, "Should have 2 jobs selected");

    // Simulate job operation complete
    app.update(Action::JobOperationComplete(
        "Operation complete".to_string(),
    ));

    // Selection should be cleared
    assert!(
        app.selected_jobs.is_empty(),
        "Selection should be cleared after JobOperationComplete"
    );
    assert_eq!(
        app.search_status, "Operation complete",
        "Status message should be updated"
    );
    assert!(!app.loading, "Loading should be cleared");
}

#[test]
fn test_selection_persists_across_jobs_loaded() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;

    // Load initial jobs and select some
    let jobs1 = create_mock_jobs(5);
    app.update(Action::JobsLoaded(Ok(jobs1)));
    app.jobs_state.select(Some(0));
    app.handle_input(key(' '));
    app.jobs_state.select(Some(2));
    app.handle_input(key(' '));

    let selected_sids = app.selected_jobs.clone();
    assert_eq!(selected_sids.len(), 2);

    // Simulate refresh with new job list (same SIDs)
    let jobs2 = create_mock_jobs(5);
    app.update(Action::JobsLoaded(Ok(jobs2)));

    // Selection should still be present (tracked by SID)
    assert_eq!(
        app.selected_jobs.len(),
        2,
        "Selection should persist across JobsLoaded"
    );
    assert_eq!(
        app.selected_jobs, selected_sids,
        "Same SIDs should still be selected"
    );
}

#[test]
fn test_batch_popup_cancel_with_n() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(3))));

    // Select job
    app.jobs_state.select(Some(0));
    app.handle_input(key(' '));

    // Open batch cancel popup
    app.handle_input(key('c'));
    assert!(app.popup.is_some());

    // Press 'n' to cancel
    let action = app.handle_input(key('n'));
    assert!(action.is_none(), "Canceling popup should not return action");
    assert!(app.popup.is_none(), "Popup should be closed");

    // Selection should still be present
    assert_eq!(app.selected_jobs.len(), 1);
}

#[test]
fn test_batch_popup_cancel_with_esc() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(3))));

    // Select jobs
    app.jobs_state.select(Some(0));
    app.handle_input(key(' '));
    app.jobs_state.select(Some(1));
    app.handle_input(key(' '));

    // Open batch delete popup
    app.handle_input(key('d'));
    assert!(app.popup.is_some());

    // Press Esc to cancel
    let action = app.handle_input(esc_key());
    assert!(action.is_none(), "Canceling popup should not return action");
    assert!(app.popup.is_none(), "Popup should be closed");

    // Selection should still be present
    assert_eq!(app.selected_jobs.len(), 2);
}

#[test]
fn test_batch_confirm_with_enter() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(3))));

    // Select jobs
    app.jobs_state.select(Some(0));
    app.handle_input(key(' '));
    app.jobs_state.select(Some(2));
    app.handle_input(key(' '));

    // Open batch cancel popup
    app.handle_input(key('c'));
    assert!(app.popup.is_some());

    // Press Enter to confirm
    let action = app.handle_input(enter_key());
    assert!(
        action.is_some(),
        "Confirming with Enter should return action"
    );
    assert!(
        matches!(action, Some(Action::CancelJobsBatch(_))),
        "Should be CancelJobsBatch action"
    );
    assert!(app.popup.is_none(), "Popup should be closed after confirm");
}
