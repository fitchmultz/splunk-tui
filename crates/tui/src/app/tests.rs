//! Core App struct tests.
//!
//! Tests the main App struct methods and behavior. These were extracted from
//! app.rs to reduce file size (RQ-0277).

use crate::action::Action;
use crate::app::state::CurrentScreen;
use crate::app::{App, ConnectionContext};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use splunk_config::{ListDefaults, PersistedState};

#[test]
fn test_app_new_default() {
    let app = App::new(None, ConnectionContext::default());
    assert_eq!(app.current_screen, CurrentScreen::Search);
    assert!(app.indexes_state.selected().is_some());
    assert!(app.jobs_state.selected().is_some());
}

#[test]
fn test_add_to_history() {
    let mut app = App::new(None, ConnectionContext::default());

    app.add_to_history("query1".to_string());
    assert_eq!(app.search_history.len(), 1);
    assert_eq!(app.search_history[0], "query1");

    // Add same query again - should move to front
    app.add_to_history("query2".to_string());
    app.add_to_history("query1".to_string());
    assert_eq!(app.search_history.len(), 2);
    assert_eq!(app.search_history[0], "query1");
    assert_eq!(app.search_history[1], "query2");
}

#[test]
fn test_clipboard_preview() {
    let short = "short text";
    assert_eq!(App::clipboard_preview(short), "short text");

    let long = "this is a very long text that should be truncated";
    let preview = App::clipboard_preview(long);
    assert!(preview.len() <= 33); // 30 + "..."
    assert!(preview.ends_with("..."));

    let with_newlines = "line1\nline2\nline3";
    assert_eq!(App::clipboard_preview(with_newlines), "line1 line2 line3");
}

#[test]
fn test_load_action_for_screen() {
    let mut app = App::new(None, ConnectionContext::default());

    app.current_screen = CurrentScreen::Indexes;
    let action = app.load_action_for_screen();
    assert!(action.is_some());
    assert!(matches!(
        action.unwrap(),
        Action::LoadIndexes {
            count: _,
            offset: _
        }
    ));

    app.current_screen = CurrentScreen::Jobs;
    let action = app.load_action_for_screen();
    assert!(action.is_some());
    assert!(matches!(
        action.unwrap(),
        Action::LoadJobs {
            count: _,
            offset: _
        }
    ));

    app.current_screen = CurrentScreen::Search;
    assert!(app.load_action_for_screen().is_none());
}

#[test]
fn test_load_action_for_shc_skips_when_shc_unavailable() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Shc;

    app.shc_unavailable = false;
    assert!(matches!(
        app.load_action_for_screen(),
        Some(Action::LoadShcStatus)
    ));

    app.shc_unavailable = true;
    assert!(app.load_action_for_screen().is_none());
}

#[test]
fn test_global_e_keybinding_shows_error_details() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_error = Some(crate::error_details::ErrorDetails::from_error_string(
        "Test error",
    ));

    // Press 'e' key with no modifiers
    let key = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE);
    let action = app.handle_input(key);

    assert!(
        matches!(action, Some(Action::ShowErrorDetailsFromCurrent)),
        "Pressing 'e' when error exists should return ShowErrorDetailsFromCurrent action"
    );
}

#[test]
fn test_global_e_keybinding_no_error_does_nothing() {
    let mut app = App::new(None, ConnectionContext::default());
    // No error set
    app.current_error = None;

    // Press 'e' key on Apps screen (where 'e' normally enables selected app)
    app.current_screen = CurrentScreen::Apps;
    let key = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE);
    let action = app.handle_input(key);

    // Should NOT return ShowErrorDetailsFromCurrent since no error
    assert!(
        !matches!(action, Some(Action::ShowErrorDetailsFromCurrent)),
        "Pressing 'e' when no error exists should NOT return ShowErrorDetailsFromCurrent"
    );
}

#[test]
fn test_global_shift_d_keybinding_dismisses_onboarding_item() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Settings;

    let key = KeyEvent::new(KeyCode::Char('D'), KeyModifiers::SHIFT);
    let action = app.handle_input(key);

    assert!(
        matches!(action, Some(Action::DismissOnboardingItem)),
        "Pressing 'D' while onboarding checklist is visible should dismiss current onboarding item"
    );
}

#[test]
fn test_global_ctrl_d_keybinding_dismisses_onboarding_all() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Settings;

    let key = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL);
    let action = app.handle_input(key);

    assert!(
        matches!(action, Some(Action::DismissOnboardingAll)),
        "Pressing 'Ctrl+d' while onboarding checklist is visible should dismiss all onboarding items"
    );
}

#[test]
fn test_toggle_shc_view_mode_action_updates_state() {
    let mut app = App::new(None, ConnectionContext::default());
    assert_eq!(app.shc_view_mode, crate::app::state::ShcViewMode::Summary);

    app.update(Action::ToggleShcViewMode);
    assert_eq!(app.shc_view_mode, crate::app::state::ShcViewMode::Members);
}

#[test]
fn test_load_more_action_respects_max_items_cap() {
    // Create app with default list_defaults (max_items = 1000)
    let mut app = App::new(None, ConnectionContext::default());

    // Test Indexes screen
    app.current_screen = CurrentScreen::Indexes;

    // Simulate loading items up to the cap
    app.indexes_pagination.update_loaded(1000);
    assert_eq!(app.indexes_pagination.total_loaded, 1000);
    assert!(
        app.indexes_pagination.has_more,
        "has_more should still be true after loading full page"
    );

    // load_more_action should return None because we're at the cap
    let action = app.load_more_action_for_current_screen();
    assert!(
        action.is_none(),
        "load_more_action should return None when max_items cap is reached"
    );

    // Test with a lower cap to verify boundary
    let mut app_low_cap = App::new(
        Some(PersistedState {
            list_defaults: ListDefaults {
                page_size: 10,
                max_items: 50,
                indexes_page_size: None,
                jobs_page_size: None,
                apps_page_size: None,
                users_page_size: None,
                roles_page_size: None,
            },
            ..PersistedState::default()
        }),
        ConnectionContext::default(),
    );

    app_low_cap.current_screen = CurrentScreen::Jobs;

    // Load 40 items (under cap)
    app_low_cap.jobs_pagination.update_loaded(40);
    let action = app_low_cap.load_more_action_for_current_screen();
    assert!(
        action.is_some(),
        "load_more_action should return Some when under cap"
    );

    // Load 10 more items (at cap)
    app_low_cap.jobs_pagination.update_loaded(10);
    assert_eq!(app_low_cap.jobs_pagination.total_loaded, 50);

    // load_more_action should return None at cap
    let action = app_low_cap.load_more_action_for_current_screen();
    assert!(
        action.is_none(),
        "load_more_action should return None when at max_items cap"
    );
}

#[test]
fn test_load_more_action_works_normally_under_cap() {
    let mut app = App::new(None, ConnectionContext::default());

    // Test Jobs screen with items under cap
    app.current_screen = CurrentScreen::Jobs;
    app.jobs_pagination.update_loaded(100); // 100 items loaded, default cap is 1000

    let action = app.load_more_action_for_current_screen();
    assert!(
        action.is_some(),
        "load_more_action should return Some when under cap and has_more is true"
    );

    // Verify the action has correct pagination parameters
    match action {
        Some(Action::LoadJobs { count, offset }) => {
            assert_eq!(count, app.jobs_pagination.page_size);
            assert_eq!(offset, 100);
        }
        _ => panic!("Expected LoadJobs action"),
    }
}

#[test]
fn test_translate_refresh_action_produces_offset_zero() {
    let app = App::new(None, ConnectionContext::default());

    // Test RefreshIndexes produces LoadIndexes with offset=0
    let action = app.translate_refresh_action(Action::RefreshIndexes);
    match action {
        Action::LoadIndexes { count, offset } => {
            assert_eq!(offset, 0, "RefreshIndexes should produce offset=0");
            assert_eq!(count, app.indexes_pagination.page_size);
        }
        _ => panic!("Expected LoadIndexes action, got {:?}", action),
    }

    // Test RefreshJobs produces LoadJobs with offset=0
    let action = app.translate_refresh_action(Action::RefreshJobs);
    match action {
        Action::LoadJobs { count, offset } => {
            assert_eq!(offset, 0, "RefreshJobs should produce offset=0");
            assert_eq!(count, app.jobs_pagination.page_size);
        }
        _ => panic!("Expected LoadJobs action, got {:?}", action),
    }

    // Test RefreshApps produces LoadApps with offset=0
    let action = app.translate_refresh_action(Action::RefreshApps);
    match action {
        Action::LoadApps { count, offset } => {
            assert_eq!(offset, 0, "RefreshApps should produce offset=0");
            assert_eq!(count, app.apps_pagination.page_size);
        }
        _ => panic!("Expected LoadApps action, got {:?}", action),
    }

    // Test RefreshUsers produces LoadUsers with offset=0
    let action = app.translate_refresh_action(Action::RefreshUsers);
    match action {
        Action::LoadUsers { count, offset } => {
            assert_eq!(offset, 0, "RefreshUsers should produce offset=0");
            assert_eq!(count, app.users_pagination.page_size);
        }
        _ => panic!("Expected LoadUsers action, got {:?}", action),
    }

    // Test RefreshDashboards produces LoadDashboards with offset=0
    let action = app.translate_refresh_action(Action::RefreshDashboards);
    match action {
        Action::LoadDashboards { count, offset } => {
            assert_eq!(offset, 0, "RefreshDashboards should produce offset=0");
            assert_eq!(count, app.dashboards_pagination.page_size);
        }
        _ => panic!("Expected LoadDashboards action, got {:?}", action),
    }

    // Test RefreshDataModels produces LoadDataModels with offset=0
    let action = app.translate_refresh_action(Action::RefreshDataModels);
    match action {
        Action::LoadDataModels { count, offset } => {
            assert_eq!(offset, 0, "RefreshDataModels should produce offset=0");
            assert_eq!(count, app.data_models_pagination.page_size);
        }
        _ => panic!("Expected LoadDataModels action, got {:?}", action),
    }

    // Test RefreshInputs produces LoadInputs with offset=0
    let action = app.translate_refresh_action(Action::RefreshInputs);
    match action {
        Action::LoadInputs { count, offset } => {
            assert_eq!(offset, 0, "RefreshInputs should produce offset=0");
            assert_eq!(count, app.inputs_pagination.page_size);
        }
        _ => panic!("Expected LoadInputs action, got {:?}", action),
    }

    // Test RefreshInternalLogs produces LoadInternalLogs with default params
    let action = app.translate_refresh_action(Action::RefreshInternalLogs);
    match action {
        Action::LoadInternalLogs { count, earliest } => {
            assert_eq!(count, app.internal_logs_defaults.count);
            assert_eq!(earliest, app.internal_logs_defaults.earliest_time);
        }
        _ => panic!("Expected LoadInternalLogs action, got {:?}", action),
    }
}

#[test]
fn test_translate_refresh_action_passes_through_unrecognized() {
    let app = App::new(None, ConnectionContext::default());

    // Non-refresh actions should pass through unchanged
    let action = app.translate_refresh_action(Action::Quit);
    assert!(matches!(action, Action::Quit));

    let action = app.translate_refresh_action(Action::NavigateDown);
    assert!(matches!(action, Action::NavigateDown));

    // LoadMore actions should pass through unchanged
    let action = app.translate_refresh_action(Action::LoadMoreJobs);
    assert!(matches!(action, Action::LoadMoreJobs));
}
