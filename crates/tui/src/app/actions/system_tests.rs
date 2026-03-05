//! Purpose: Unit tests for system-level TUI actions and state transitions.
//! Responsibilities: Verify action handlers for loading state, UI notifications, dialogs, and error/pop-up behavior.
//! Non-scope: Does not perform API/network integration; tests only in-memory `App` state mutation.
//! Invariants/Assumptions: Tests are deterministic and isolated per test case.

use super::*;
use crate::ConnectionContext;

#[test]
fn test_loading_sets_progress_to_zero() {
    let mut app = App::new(None, ConnectionContext::default());
    app.progress = 0.5;

    app.handle_system_action(Action::Loading(true));

    assert!(app.loading);
    assert_eq!(app.progress, 0.0);
}

#[test]
fn test_progress_updates_value() {
    let mut app = App::new(None, ConnectionContext::default());

    app.handle_system_action(Action::Progress(0.75));

    assert_eq!(app.progress, 0.75);
}

#[test]
fn test_notify_adds_toast() {
    let mut app = App::new(None, ConnectionContext::default());

    app.handle_system_action(Action::Notify(
        crate::ui::ToastLevel::Info,
        "Test message".to_string(),
    ));

    assert_eq!(app.toasts.len(), 1);
    assert_eq!(app.toasts[0].message, "Test message");
}

#[test]
fn test_clipboard_error_toast_is_deduplicated_while_active() {
    let _guard = crate::app::clipboard::install_failing_clipboard("Clipboard unavailable");
    let mut app = App::new(None, ConnectionContext::default());

    app.handle_system_action(Action::CopyToClipboard("first".to_string()));
    app.handle_system_action(Action::CopyToClipboard("second".to_string()));

    let error_toasts: Vec<_> = app
        .toasts
        .iter()
        .filter(|t| t.level == crate::ui::ToastLevel::Error)
        .collect();
    assert_eq!(error_toasts.len(), 1);
    assert!(
        error_toasts[0]
            .message
            .contains("Clipboard error: Clipboard unavailable"),
        "Unexpected error toast message: {}",
        error_toasts[0].message
    );
}

#[test]
fn test_clipboard_error_toast_can_reappear_after_expiry() {
    let _guard = crate::app::clipboard::install_failing_clipboard("Clipboard unavailable");
    let mut app = App::new(None, ConnectionContext::default());

    app.handle_system_action(Action::CopyToClipboard("first".to_string()));
    assert_eq!(
        app.toasts
            .iter()
            .filter(|t| t.level == crate::ui::ToastLevel::Error)
            .count(),
        1
    );

    app.toasts[0].created_at = std::time::Instant::now() - std::time::Duration::from_secs(120);
    app.handle_system_action(Action::CopyToClipboard("second".to_string()));

    assert_eq!(
        app.toasts
            .iter()
            .filter(|t| t.level == crate::ui::ToastLevel::Error)
            .count(),
        2
    );
}

#[test]
fn test_info_toast_is_deduplicated_while_active() {
    let mut app = App::new(None, ConnectionContext::default());

    app.push_info_toast_once("Nothing to copy");
    app.push_info_toast_once("Nothing to copy");

    let info_toasts: Vec<_> = app
        .toasts
        .iter()
        .filter(|t| t.level == crate::ui::ToastLevel::Info && t.message == "Nothing to copy")
        .collect();
    assert_eq!(info_toasts.len(), 1);
}

#[test]
fn test_info_toast_can_reappear_after_expiry() {
    let mut app = App::new(None, ConnectionContext::default());

    app.push_info_toast_once("Nothing to copy");
    assert_eq!(
        app.toasts
            .iter()
            .filter(|t| t.level == crate::ui::ToastLevel::Info && t.message == "Nothing to copy")
            .count(),
        1
    );

    app.toasts[0].created_at = std::time::Instant::now() - std::time::Duration::from_secs(120);
    app.push_info_toast_once("Nothing to copy");

    assert_eq!(
        app.toasts
            .iter()
            .filter(|t| t.level == crate::ui::ToastLevel::Info && t.message == "Nothing to copy")
            .count(),
        2
    );
}

#[test]
fn test_tick_prunes_expired_toasts() {
    let mut app = App::new(None, ConnectionContext::default());
    // Add a toast that's already expired (using created_at field)
    let mut expired_toast = Toast::info("Expired");
    expired_toast.created_at = std::time::Instant::now() - std::time::Duration::from_secs(100);
    app.toasts.push(expired_toast);

    // Add a fresh toast
    app.toasts.push(Toast::info("Fresh"));

    app.handle_system_action(Action::Tick);

    // Only the fresh toast should remain
    assert_eq!(app.toasts.len(), 1);
    assert_eq!(app.toasts[0].message, "Fresh");
}

#[test]
fn test_enter_search_mode_saves_current_filter() {
    let mut app = App::new(None, ConnectionContext::default());
    app.search_filter = Some("existing filter".to_string());

    app.handle_system_action(Action::EnterSearchMode);

    assert!(app.is_filtering);
    assert_eq!(app.filter_before_edit, Some("existing filter".to_string()));
    assert_eq!(app.filter_input.value(), "existing filter");
}

#[test]
fn test_enter_search_mode_with_no_filter() {
    let mut app = App::new(None, ConnectionContext::default());
    app.search_filter = None;

    app.handle_system_action(Action::EnterSearchMode);

    assert!(app.is_filtering);
    assert!(app.filter_before_edit.is_none());
    assert!(app.filter_input.is_empty());
}

#[test]
fn test_search_input_appends_character() {
    let mut app = App::new(None, ConnectionContext::default());
    app.filter_input.set_value("hel");

    app.handle_system_action(Action::SearchInput('l'));
    app.handle_system_action(Action::SearchInput('o'));

    assert_eq!(app.filter_input.value(), "hello");
}

#[test]
fn test_clear_search_clears_filter() {
    let mut app = App::new(None, ConnectionContext::default());
    app.search_filter = Some("test".to_string());

    app.handle_system_action(Action::ClearSearch);

    assert!(app.search_filter.is_none());
}

#[test]
fn test_cycle_theme_changes_theme() {
    let mut app = App::new(None, ConnectionContext::default());
    let initial_theme = app.color_theme;

    app.handle_system_action(Action::CycleTheme);

    assert_ne!(app.color_theme, initial_theme);
    assert_eq!(app.toasts.len(), 1);
    assert!(app.toasts[0].message.contains("Theme:"));
}

#[test]
fn test_spl_validation_result_updates_state() {
    let mut app = App::new(None, ConnectionContext::default());

    app.handle_system_action(Action::SplValidationResult {
        valid: true,
        errors: vec![],
        warnings: vec!["Warning 1".to_string()],
        request_id: 0,
    });

    assert_eq!(app.spl_validation_state.valid, Some(true));
    assert!(
        app.spl_validation_state
            .warnings
            .contains(&"Warning 1".to_string())
    );
    assert!(!app.spl_validation_pending);
}

#[test]
fn test_show_error_details_from_current_with_no_error() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_error = None;

    app.handle_system_action(Action::ShowErrorDetailsFromCurrent);

    assert!(app.popup.is_none());
}

#[test]
fn test_clear_error_details_clears_state() {
    let mut app = App::new(None, ConnectionContext::default());
    use crate::ui::popup::{Popup, PopupType};
    app.current_error = Some(crate::error_details::ErrorDetails::from_error_string(
        "Error",
    ));
    app.popup = Some(Popup::builder(PopupType::ErrorDetails).build());

    app.handle_system_action(Action::ClearErrorDetails);

    assert!(app.current_error.is_none());
    assert!(app.popup.is_none());
}

#[test]
fn test_job_operation_complete_clears_selection() {
    let mut app = App::new(None, ConnectionContext::default());
    app.selected_jobs.insert("job1".to_string());
    app.selected_jobs.insert("job2".to_string());

    app.handle_system_action(Action::JobOperationComplete("Jobs finalized".to_string()));

    assert!(app.selected_jobs.is_empty());
    assert_eq!(app.search_status, "Jobs finalized");
    assert!(!app.loading);
}

#[test]
fn test_resize_updates_last_area() {
    let mut app = App::new(None, ConnectionContext::default());

    app.handle_system_action(Action::Resize(100, 50));

    assert_eq!(app.last_area.width, 100);
    assert_eq!(app.last_area.height, 50);
}

#[test]
fn test_open_create_macro_dialog() {
    let mut app = App::new(None, ConnectionContext::default());

    app.handle_system_action(Action::OpenCreateMacroDialog);

    assert!(app.popup.is_some());
    assert!(matches!(
        app.popup,
        Some(crate::ui::popup::Popup {
            kind: crate::ui::popup::PopupType::CreateMacro { .. },
            ..
        })
    ));
}

#[test]
fn test_edit_macro_action_opens_popup() {
    use crate::ui::popup::{MacroField, PopupType};
    use splunk_client::models::Macro;

    let mut app = App::new(None, ConnectionContext::default());

    // Set up test macro data
    app.macros = Some(vec![Macro {
        name: "test_macro".to_string(),
        definition: "index=main".to_string(),
        args: Some("arg1,arg2".to_string()),
        description: Some("Test description".to_string()),
        disabled: false,
        iseval: true,
        validation: None,
        errormsg: None,
    }]);
    app.macros_state.select(Some(0));

    // Trigger edit action
    app.handle_system_action(Action::EditMacro);

    assert!(app.popup.is_some());
    assert!(matches!(
        app.popup,
        Some(crate::ui::popup::Popup {
            kind: PopupType::EditMacro {
                macro_name,
                disabled: false,
                iseval: true,
                selected_field: MacroField::Definition,
                ..
            },
            ..
        }) if macro_name == "test_macro"
    ));
}

#[test]
fn test_edit_macro_action_no_selection() {
    let mut app = App::new(None, ConnectionContext::default());

    // No macros loaded
    app.macros = None;

    // Trigger edit action
    app.handle_system_action(Action::EditMacro);

    // Should show toast and not open popup
    assert!(app.popup.is_none());
    assert_eq!(app.toasts.len(), 1);
    assert_eq!(app.toasts[0].message, "No macro selected");
}

#[test]
fn test_edit_macro_action_no_macro_selected() {
    use splunk_client::models::Macro;

    let mut app = App::new(None, ConnectionContext::default());

    // Set up test macro data but no selection
    app.macros = Some(vec![Macro {
        name: "test_macro".to_string(),
        definition: "index=main".to_string(),
        args: Some("arg1,arg2".to_string()),
        description: Some("Test description".to_string()),
        disabled: false,
        iseval: true,
        validation: None,
        errormsg: None,
    }]);
    // No selection made
    app.macros_state.select(None);

    // Trigger edit action
    app.handle_system_action(Action::EditMacro);

    // Should show toast and not open popup
    assert!(app.popup.is_none());
    assert_eq!(app.toasts.len(), 1);
    assert_eq!(app.toasts[0].message, "No macro selected");
}

#[test]
fn test_dismiss_onboarding_item_marks_first_incomplete_as_dismissed() {
    let mut app = App::new(None, ConnectionContext::default());
    let before = app.onboarding_checklist.dismissed_items.len();

    app.handle_system_action(Action::DismissOnboardingItem);

    assert_eq!(app.onboarding_checklist.dismissed_items.len(), before + 1);
}

#[test]
fn test_dismiss_onboarding_all_sets_global_flag() {
    let mut app = App::new(None, ConnectionContext::default());
    assert!(!app.onboarding_checklist.globally_dismissed);

    app.handle_system_action(Action::DismissOnboardingAll);

    assert!(app.onboarding_checklist.globally_dismissed);
}

#[test]
fn test_shc_rolling_restart_result_updates_loading_and_toast() {
    let mut app = App::new(None, ConnectionContext::default());
    app.loading = true;

    app.handle_system_action(Action::ShcRollingRestarted { result: Ok(()) });
    assert!(!app.loading);
    assert!(
        app.toasts
            .iter()
            .any(|t| t.message.contains("rolling restart"))
    );
}

#[test]
fn test_shc_captain_set_error_updates_loading_and_toast() {
    let mut app = App::new(None, ConnectionContext::default());
    app.loading = true;

    app.handle_system_action(Action::ShcCaptainSet {
        result: Err("boom".to_string()),
    });
    assert!(!app.loading);
    assert!(
        app.toasts
            .iter()
            .any(|t| t.message.contains("Failed to set SHC captain"))
    );
}
