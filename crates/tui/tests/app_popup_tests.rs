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
use helpers::*;
use splunk_client::models::SearchJobStatus;
use splunk_tui::{
    CurrentScreen, Popup, PopupType, action::Action, action::ExportFormat, app::App,
    app::ConnectionContext,
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
    assert!(matches!(action, Some(Action::DeleteJob(sid)) if sid == expected_sid));
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
