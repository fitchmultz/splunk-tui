//! Tests for popup open/close, confirm/cancel flows, and delete confirmation.
//!
//! This module tests:
//! - Cancel flow popups (open, cancel, confirm)
//! - Delete confirmation popups
//! - Help popup open/close
//! - Job inspect help popup
//! - Export popup flow
//! - App enable/disable popups
//!
//! ## Invariants
//! - Popups must preserve selection state when canceled
//! - Confirm actions must return the correct Action type
//! - Escape and 'n' keys must cancel popups without returning actions
//!
//! ## Test Organization
//! Tests are grouped by popup type and flow.

mod helpers;
use crossterm::event::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use helpers::*;
use splunk_client::models::SearchJobStatus;
use splunk_tui::{
    CurrentScreen, Popup, PopupType, action::Action, action::ExportFormat, app::App,
    app::ConnectionContext, undo::UndoableOperation,
};

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
fn test_popup_cancel_flow() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(3))));
    app.jobs_state.select(Some(1));

    // Open cancel popup by pressing 'c'
    let action = app.handle_input(key('c'));
    assert!(action.is_none(), "Opening popup should not return action");
    assert!(app.popup.is_some(), "Popup should be open");
    assert!(
        matches!(
            app.popup.as_ref().map(|p| &p.kind),
            Some(PopupType::ConfirmCancel(_))
        ),
        "Should be ConfirmCancel popup"
    );

    // Press 'n' to cancel
    let action = app.handle_input(key('n'));
    assert!(action.is_none(), "Canceling popup should not return action");
    assert!(app.popup.is_none(), "Popup should be closed");

    // Verify selection is preserved
    assert_eq!(
        app.jobs_state.selected(),
        Some(1),
        "Selection should be preserved"
    );
}

#[test]
fn test_popup_cancel_with_escape() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(3))));
    app.jobs_state.select(Some(1));

    // Open cancel popup
    app.handle_input(key('c'));
    assert!(app.popup.is_some());

    // Press Esc to cancel
    let action = app.handle_input(esc_key());
    assert!(
        action.is_none(),
        "Canceling with Esc should not return action"
    );
    assert!(app.popup.is_none(), "Popup should be closed");
}

#[test]
fn test_popup_confirm_cancel_action() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(3))));
    app.jobs_state.select(Some(1));
    let expected_sid = "sid_1".to_string();

    // Open cancel popup
    app.handle_input(key('c'));
    assert!(app.popup.is_some());

    // Press 'y' to confirm
    let action = app.handle_input(key('y'));
    assert!(action.is_some(), "Confirming should return action");
    assert!(matches!(action, Some(Action::CancelJob(sid)) if sid == expected_sid));
    assert!(app.popup.is_none(), "Popup should be closed after confirm");
}

#[test]
fn test_popup_confirm_with_enter() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(3))));
    app.jobs_state.select(Some(1));
    let expected_sid = "sid_1".to_string();

    // Open cancel popup
    app.handle_input(key('c'));

    // Press Enter to confirm
    let action = app.handle_input(enter_key());
    assert!(
        action.is_some(),
        "Confirming with Enter should return action"
    );
    assert!(matches!(action, Some(Action::CancelJob(sid)) if sid == expected_sid));
}

#[test]
fn test_popup_delete_confirm_action() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(3))));
    app.jobs_state.select(Some(2));
    let expected_sid = "sid_2".to_string();

    // Open delete popup by pressing 'd'
    app.handle_input(key('d'));
    assert!(matches!(
        app.popup.as_ref().map(|p| &p.kind),
        Some(PopupType::ConfirmDelete(_))
    ));

    // Press 'y' to confirm
    let action = app.handle_input(key('y'));
    assert!(action.is_some(), "Confirming delete should return action");
    assert!(
        matches!(
            &action,
            Some(Action::QueueUndoableOperation {
                operation: UndoableOperation::DeleteJob { sid },
                description,
            }) if sid == &expected_sid && description.contains("Delete job")
        ),
        "Expected QueueUndoableOperation for DeleteJob, got {:?}",
        action
    );
}

#[test]
fn test_help_popup_open_close() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    // Switch to ResultsFocused mode first (help only works in this mode on Search screen)
    app.search_input_mode = splunk_tui::SearchInputMode::ResultsFocused;

    // Open help popup
    let action = app.handle_input(key('?'));
    assert!(
        matches!(action, Some(Action::OpenHelpPopup)),
        "Opening help should return OpenHelpPopup action"
    );
    app.update(action.unwrap());
    assert!(
        matches!(app.popup.as_ref().map(|p| &p.kind), Some(PopupType::Help)),
        "Should open Help popup"
    );

    // Close with Esc
    let action = app.handle_input(esc_key());
    assert!(action.is_none(), "Closing help should not return action");
    assert!(app.popup.is_none(), "Popup should be closed");

    // Reopen with '?' (still in ResultsFocused mode)
    let action = app.handle_input(key('?'));
    assert!(matches!(action, Some(Action::OpenHelpPopup)));
    app.update(action.unwrap());
    assert!(matches!(
        app.popup.as_ref().map(|p| &p.kind),
        Some(PopupType::Help)
    ));

    // Close with 'q' (still in ResultsFocused mode)
    let action = app.handle_input(key('q'));
    assert!(
        action.is_none(),
        "Closing help with 'q' should not return action"
    );
    assert!(app.popup.is_none(), "Popup should be closed");
}

#[test]
fn test_job_inspect_help_popup() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::JobInspect;

    // Open help popup with '?'
    let action = app.handle_input(key('?'));
    assert!(matches!(action, Some(Action::OpenHelpPopup)));
    app.update(action.unwrap());
    assert!(
        matches!(app.popup.as_ref().map(|p| &p.kind), Some(PopupType::Help)),
        "Should open Help popup"
    );

    // Close with Esc
    let action = app.handle_input(esc_key());
    assert!(action.is_none(), "Closing help should not return action");
    assert!(app.popup.is_none(), "Popup should be closed");
}

#[test]
fn test_export_search_popup_flow() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.set_search_results(vec![serde_json::json!({"foo": "bar"})]);

    // Press Ctrl+e to open export popup
    app.handle_input(ctrl_key('e'));
    assert!(app.popup.is_some());
    assert!(matches!(
        app.popup.as_ref().map(|p| &p.kind),
        Some(PopupType::ExportSearch)
    ));
    assert_eq!(app.export_input.value(), "results.json");
    assert_eq!(app.export_format, ExportFormat::Json);

    // Press Tab to toggle format: Json -> Csv
    app.handle_input(tab_key());
    assert_eq!(app.export_format, ExportFormat::Csv);
    assert_eq!(app.export_input.value(), "results.csv");

    // Toggle to Ndjson
    app.handle_input(tab_key());
    assert_eq!(app.export_format, ExportFormat::Ndjson);
    assert_eq!(app.export_input.value(), "results.ndjson");

    // Toggle to Yaml
    app.handle_input(tab_key());
    assert_eq!(app.export_format, ExportFormat::Yaml);
    assert_eq!(app.export_input.value(), "results.yaml");

    // Toggle to Markdown
    app.handle_input(tab_key());
    assert_eq!(app.export_format, ExportFormat::Markdown);
    assert_eq!(app.export_input.value(), "results.md");

    // Toggle back to Json
    app.handle_input(tab_key());
    assert_eq!(app.export_format, ExportFormat::Json);
    assert_eq!(app.export_input.value(), "results.json");

    // Toggle back to Csv for further testing
    app.handle_input(tab_key());
    assert_eq!(app.export_format, ExportFormat::Csv);
    assert_eq!(app.export_input.value(), "results.csv");

    // Backspace and type new filename
    for _ in 0..12 {
        app.handle_input(backspace_key());
    }
    app.handle_input(key('d'));
    app.handle_input(key('a'));
    app.handle_input(key('t'));
    app.handle_input(key('a'));
    app.handle_input(key('.'));
    app.handle_input(key('c'));
    app.handle_input(key('s'));
    app.handle_input(key('v'));
    assert_eq!(app.export_input.value(), "data.csv");

    // Press Enter to confirm export
    let action = app.handle_input(enter_key());
    assert!(action.is_some());
    if let Some(Action::ExportData(data, path, format)) = action {
        assert!(data.is_array());
        assert_eq!(path.to_str().unwrap(), "data.csv");
        assert_eq!(format, ExportFormat::Csv);
    } else {
        panic!("Should return ExportData action");
    }
    assert!(app.popup.is_none());
}

#[test]
fn test_export_search_disabled_when_no_results() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_results = Vec::new();

    // Press Ctrl+e - should not open popup
    app.handle_input(ctrl_key('e'));
    assert!(app.popup.is_none());
}

#[test]
fn test_export_search_cancel_with_esc() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.set_search_results(vec![serde_json::json!({"foo": "bar"})]);

    app.handle_input(ctrl_key('e'));
    assert!(app.popup.is_some());

    app.handle_input(esc_key());
    assert!(app.popup.is_none());
}

#[test]
fn test_popup_confirm_enable_app() {
    let mut app = App::new(None, ConnectionContext::default());
    app.popup = Some(Popup::builder(PopupType::ConfirmEnableApp("test-app".to_string())).build());

    // Confirm with 'y'
    let action = app.handle_popup_input(key('y'));
    assert!(matches!(action, Some(Action::EnableApp(name)) if name == "test-app"));
    assert!(app.popup.is_none());
}

#[test]
fn test_popup_confirm_disable_app() {
    let mut app = App::new(None, ConnectionContext::default());
    app.popup = Some(Popup::builder(PopupType::ConfirmDisableApp("test-app".to_string())).build());

    // Confirm with Enter
    let action = app.handle_popup_input(enter_key());
    assert!(matches!(action, Some(Action::DisableApp(name)) if name == "test-app"));
    assert!(app.popup.is_none());
}

#[test]
fn test_popup_cancel_enable_app() {
    let mut app = App::new(None, ConnectionContext::default());
    app.popup = Some(Popup::builder(PopupType::ConfirmEnableApp("test-app".to_string())).build());

    // Cancel with 'n'
    let action = app.handle_popup_input(key('n'));
    assert!(action.is_none());
    assert!(app.popup.is_none());
}

#[test]
fn test_popup_cancel_disable_app() {
    let mut app = App::new(None, ConnectionContext::default());
    app.popup = Some(Popup::builder(PopupType::ConfirmDisableApp("test-app".to_string())).build());

    // Cancel with Esc
    let action = app.handle_popup_input(esc_key());
    assert!(action.is_none());
    assert!(app.popup.is_none());
}

// ============================================================================
// Popup Mouse Interaction Tests
// ============================================================================

fn mouse_click(col: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: col,
        row,
        modifiers: KeyModifiers::empty(),
    }
}

#[test]
fn test_popup_confirm_cancel_mouse_click_inside() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(3))));
    app.jobs_state.select(Some(1));
    app.last_area = ratatui::layout::Rect::new(0, 0, 80, 24);

    // Open cancel popup by pressing 'c'
    app.handle_input(key('c'));
    assert!(app.popup.is_some());

    // Popup is 60% x 50% centered, so in 80x24 terminal:
    // - popup_area: x=16, y=6, width=48, height=12
    // Click anywhere inside popup - should confirm
    let popup_center_x = 16 + 24; // center of popup horizontally
    let popup_center_y = 6 + 6; // center of popup vertically

    let event = mouse_click(popup_center_x, popup_center_y);
    let action = app.handle_mouse(event);

    assert!(matches!(action, Some(Action::CancelJob(sid)) if sid == "sid_1"));
    assert!(app.popup.is_none());
}

#[test]
fn test_popup_confirm_cancel_mouse_click_outside() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(3))));
    app.jobs_state.select(Some(1));
    app.last_area = ratatui::layout::Rect::new(0, 0, 80, 24);

    // Open cancel popup by pressing 'c'
    app.handle_input(key('c'));
    assert!(app.popup.is_some());

    // Click outside popup (top-left corner) - should cancel (close without action)
    let event = mouse_click(0, 0);
    let action = app.handle_mouse(event);

    assert!(action.is_none());
    assert!(
        app.popup.is_none(),
        "Popup should close when clicking outside"
    );
}

#[test]
fn test_popup_click_outside_closes_popup() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(3))));
    app.jobs_state.select(Some(1));
    app.last_area = ratatui::layout::Rect::new(0, 0, 80, 24);

    // Open cancel popup
    app.handle_input(key('c'));
    assert!(app.popup.is_some());

    // Click outside popup (top-left corner)
    let event = mouse_click(0, 0);
    let action = app.handle_mouse(event);

    assert!(action.is_none());
    assert!(
        app.popup.is_none(),
        "Popup should close when clicking outside"
    );
}

#[test]
fn test_popup_delete_confirm_mouse_click_inside() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(3))));
    app.jobs_state.select(Some(2));
    app.last_area = ratatui::layout::Rect::new(0, 0, 80, 24);

    // Open delete popup by pressing 'd'
    app.handle_input(key('d'));
    assert!(app.popup.is_some());
    assert!(matches!(
        app.popup.as_ref().map(|p| &p.kind),
        Some(PopupType::ConfirmDelete(_))
    ));

    // Click anywhere inside popup - should confirm
    let popup_center_x = 16 + 24;
    let popup_center_y = 6 + 6;

    let event = mouse_click(popup_center_x, popup_center_y);
    let action = app.handle_mouse(event);

    assert!(action.is_some(), "Clicking inside should confirm delete");
    assert!(
        matches!(
            &action,
            Some(Action::QueueUndoableOperation {
                operation: UndoableOperation::DeleteJob { sid },
                description,
            }) if sid == "sid_2" && description.contains("Delete job")
        ),
        "Expected QueueUndoableOperation for DeleteJob, got {:?}",
        action
    );
    assert!(app.popup.is_none());
}

#[test]
fn test_popup_non_confirmation_popup_ignores_mouse() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::ResultsFocused;
    app.last_area = ratatui::layout::Rect::new(0, 0, 80, 24);

    // Open help popup
    let action = app.handle_input(key('?'));
    assert!(matches!(action, Some(Action::OpenHelpPopup)));
    app.update(action.unwrap());
    assert!(matches!(
        app.popup.as_ref().map(|p| &p.kind),
        Some(PopupType::Help)
    ));

    // Click anywhere - should not close the popup (Help is not a confirmation popup)
    let event = mouse_click(40, 12);
    let action = app.handle_mouse(event);

    assert!(action.is_none());
    // Help popup should still be open (only confirmation popups have mouse interaction)
    assert!(app.popup.is_some());
}

#[test]
fn test_popup_batch_cancel_mouse_click_inside() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(5))));
    app.last_area = ratatui::layout::Rect::new(0, 0, 80, 24);

    // Select multiple jobs and open batch cancel popup
    app.jobs_state.select(Some(1));
    app.handle_input(key(' ')); // Toggle selection
    app.jobs_state.select(Some(2));
    app.handle_input(key(' ')); // Toggle selection
    app.handle_input(key('c')); // Open cancel popup

    assert!(app.popup.is_some());
    assert!(matches!(
        app.popup.as_ref().map(|p| &p.kind),
        Some(PopupType::ConfirmCancelBatch(_))
    ));

    // Click inside popup - should confirm batch cancel
    let event = mouse_click(40, 12);
    let action = app.handle_mouse(event);

    assert!(
        matches!(action, Some(Action::CancelJobsBatch(sids)) if sids.len() == 2),
        "Expected CancelJobsBatch with 2 sids"
    );
    assert!(app.popup.is_none());
}

#[test]
fn test_popup_enable_app_mouse_click_inside() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Apps;
    app.last_area = ratatui::layout::Rect::new(0, 0, 80, 24);

    // Create mock disabled app and open enable confirmation
    let disabled_app = splunk_client::models::App {
        name: "disabled_app".to_string(),
        disabled: true,
        version: Some("1.0.0".to_string()),
        label: Some("Disabled App".to_string()),
        is_configured: None,
        is_visible: None,
        description: None,
        author: None,
    };
    app.update(Action::AppsLoaded(Ok(vec![disabled_app])));
    app.apps_state.select(Some(0));

    // Open enable popup
    app.handle_input(key('e'));
    assert!(app.popup.is_some());
    assert!(matches!(
        app.popup.as_ref().map(|p| &p.kind),
        Some(PopupType::ConfirmEnableApp(_))
    ));

    // Click inside popup - should confirm enable
    let event = mouse_click(40, 12);
    let action = app.handle_mouse(event);

    assert!(
        matches!(action, Some(Action::EnableApp(name)) if name == "disabled_app"),
        "Expected EnableApp action"
    );
    assert!(app.popup.is_none());
}

#[test]
fn test_popup_delete_index_mouse_click_inside() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Indexes;
    app.last_area = ratatui::layout::Rect::new(0, 0, 80, 24);

    // Create mock index
    let index = splunk_client::models::Index {
        name: "test_index".to_string(),
        max_total_data_size_mb: None,
        current_db_size_mb: 0,
        total_event_count: 0,
        max_warm_db_count: None,
        max_hot_buckets: None,
        frozen_time_period_in_secs: None,
        cold_db_path: None,
        home_path: None,
        thawed_path: None,
        cold_to_frozen_dir: None,
        primary_index: None,
    };
    app.update(Action::IndexesLoaded(Ok(vec![index])));
    app.indexes_state.select(Some(0));

    // Open delete confirmation popup via action
    app.update(Action::OpenDeleteIndexConfirm {
        name: "test_index".to_string(),
    });
    assert!(app.popup.is_some());
    assert!(matches!(
        app.popup.as_ref().map(|p| &p.kind),
        Some(PopupType::DeleteIndexConfirm { .. })
    ));

    // Click inside popup - should confirm delete
    let event = mouse_click(40, 12);
    let action = app.handle_mouse(event);

    assert!(
        matches!(
            &action,
            Some(Action::QueueUndoableOperation {
                operation: UndoableOperation::DeleteIndex { name, .. },
                ..
            }) if name == "test_index"
        ),
        "Expected QueueUndoableOperation for DeleteIndex"
    );
    assert!(app.popup.is_none());
}

#[test]
fn test_popup_delete_profile_mouse_click_inside() {
    let mut app = App::new(None, ConnectionContext::default());
    app.last_area = ratatui::layout::Rect::new(0, 0, 80, 24);

    // Open delete profile confirmation popup directly
    app.popup = Some(
        splunk_tui::ui::popup::Popup::builder(
            splunk_tui::ui::popup::PopupType::DeleteProfileConfirm {
                profile_name: "test_profile".to_string(),
            },
        )
        .build(),
    );

    // Click inside popup - should confirm delete
    let event = mouse_click(40, 12);
    let action = app.handle_mouse(event);

    assert!(
        matches!(action, Some(Action::DeleteProfile { name }) if name == "test_profile"),
        "Expected DeleteProfile action"
    );
    assert!(app.popup.is_none());
}

#[test]
fn test_popup_delete_saved_search_mouse_click_inside() {
    let mut app = App::new(None, ConnectionContext::default());
    app.last_area = ratatui::layout::Rect::new(0, 0, 80, 24);

    // Open delete saved search confirmation popup directly
    app.popup = Some(
        splunk_tui::ui::popup::Popup::builder(
            splunk_tui::ui::popup::PopupType::DeleteSavedSearchConfirm {
                search_name: "test_search".to_string(),
            },
        )
        .build(),
    );

    // Click inside popup - should confirm delete
    let event = mouse_click(40, 12);
    let action = app.handle_mouse(event);

    assert!(
        matches!(
            &action,
            Some(Action::QueueUndoableOperation {
                operation: UndoableOperation::DeleteSavedSearch { name, .. },
                ..
            }) if name == "test_search"
        ),
        "Expected QueueUndoableOperation for DeleteSavedSearch"
    );
    assert!(app.popup.is_none());
}
