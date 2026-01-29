//! Integration tests for profile switching functionality.
//!
//! Tests cover:
//! - Profile selector popup opening
//! - Profile selection navigation (up/down)
//! - Profile switch confirmation
//! - State clearing after profile switch
//! - Error handling for profile switch failures

mod helpers;
use helpers::*;
use splunk_tui::{PopupType, action::Action, app::App, app::ConnectionContext};

// ============================================================================
// Profile Selector Popup Tests
// ============================================================================

#[test]
fn test_profile_selector_popup_opens_with_list() {
    let mut app = App::new(None, ConnectionContext::default());

    // Initially no popup
    assert!(app.popup.is_none());

    // Open profile selector with list of profiles
    let profiles = vec!["dev".to_string(), "prod".to_string(), "staging".to_string()];
    app.update(Action::OpenProfileSelectorWithList(profiles));

    // Popup should now be open
    assert!(app.popup.is_some());
    let popup = app.popup.as_ref().unwrap();
    assert!(matches!(popup.kind, PopupType::ProfileSelector { .. }));
}

#[test]
fn test_profile_selector_popup_empty_list_does_nothing() {
    let mut app = App::new(None, ConnectionContext::default());

    // Open profile selector with empty list
    app.update(Action::OpenProfileSelectorWithList(vec![]));

    // Popup should not be open
    assert!(app.popup.is_none());
}

#[test]
fn test_profile_selector_navigation_up() {
    let mut app = App::new(None, ConnectionContext::default());

    // Open profile selector
    let profiles = vec!["dev".to_string(), "prod".to_string(), "staging".to_string()];
    app.update(Action::OpenProfileSelectorWithList(profiles.clone()));

    // Navigate down first
    let action = app.handle_popup_input(key('j'));
    assert!(action.is_none()); // Navigation doesn't emit actions

    // Navigate up
    let action = app.handle_popup_input(key('k'));
    assert!(action.is_none());

    // Popup should still be open
    assert!(app.popup.is_some());
}

#[test]
fn test_profile_selector_navigation_down() {
    let mut app = App::new(None, ConnectionContext::default());

    // Open profile selector
    let profiles = vec!["dev".to_string(), "prod".to_string(), "staging".to_string()];
    app.update(Action::OpenProfileSelectorWithList(profiles.clone()));

    // Navigate down
    let action = app.handle_popup_input(key('j'));
    assert!(action.is_none());

    // Popup should still be open
    assert!(app.popup.is_some());
}

#[test]
fn test_profile_selector_confirm_selection() {
    let mut app = App::new(None, ConnectionContext::default());

    // Open profile selector
    let profiles = vec!["dev".to_string(), "prod".to_string()];
    app.update(Action::OpenProfileSelectorWithList(profiles.clone()));

    // Confirm selection with Enter
    let action = app.handle_popup_input(enter_key());

    // Should emit ProfileSelected action
    assert!(matches!(action, Some(Action::ProfileSelected(ref name)) if name == "dev"));

    // Popup should be closed
    assert!(app.popup.is_none());
}

#[test]
fn test_profile_selector_cancel_with_esc() {
    let mut app = App::new(None, ConnectionContext::default());

    // Open profile selector
    let profiles = vec!["dev".to_string(), "prod".to_string()];
    app.update(Action::OpenProfileSelectorWithList(profiles.clone()));

    // Cancel with Esc
    let action = app.handle_popup_input(esc_key());

    // Should not emit any action
    assert!(action.is_none());

    // Popup should be closed
    assert!(app.popup.is_none());
}

#[test]
fn test_profile_selector_cancel_with_q() {
    let mut app = App::new(None, ConnectionContext::default());

    // Open profile selector
    let profiles = vec!["dev".to_string(), "prod".to_string()];
    app.update(Action::OpenProfileSelectorWithList(profiles.clone()));

    // Cancel with 'q'
    let action = app.handle_popup_input(key('q'));

    // Should not emit any action (q is not a cancel key for ProfileSelector)
    // Note: ProfileSelector only closes on Esc, not 'q'
    // This test verifies the behavior
    assert!(action.is_none());
}

// ============================================================================
// Profile Switch Result Tests
// ============================================================================

#[test]
fn test_profile_switch_success_updates_context() {
    let mut app = App::new(None, ConnectionContext::default());

    // Initial state
    assert!(app.profile_name.is_none());

    // Simulate successful profile switch
    let ctx = ConnectionContext {
        profile_name: Some("production".to_string()),
        base_url: "https://splunk.prod.example.com:8089".to_string(),
        auth_mode: "token".to_string(),
    };
    app.update(Action::ProfileSwitchResult(Ok(ctx)));

    // Connection context should be updated
    assert_eq!(app.profile_name, Some("production".to_string()));
    assert_eq!(
        app.base_url,
        Some("https://splunk.prod.example.com:8089".to_string())
    );
    assert_eq!(app.auth_mode, Some("token".to_string()));

    // Should have a success toast
    assert!(!app.toasts.is_empty());
    assert!(app.toasts[0].message.contains("Switched to profile"));
}

#[test]
fn test_profile_switch_failure_shows_error() {
    let mut app = App::new(None, ConnectionContext::default());

    // Simulate failed profile switch
    app.update(Action::ProfileSwitchResult(Err(
        "Authentication failed".to_string()
    )));

    // Should have an error toast
    assert!(!app.toasts.is_empty());
    assert!(app.toasts[0].message.contains("Failed to switch profile"));
    assert!(app.toasts[0].message.contains("Authentication failed"));
}

#[test]
fn test_profile_switch_clears_server_info() {
    let mut app = App::new(None, ConnectionContext::default());

    // Set some server info
    app.server_version = Some("9.1.0".to_string());
    app.server_build = Some("abc123".to_string());

    // Simulate successful profile switch
    let ctx = ConnectionContext {
        profile_name: Some("dev".to_string()),
        base_url: "https://splunk.dev.example.com:8089".to_string(),
        auth_mode: "session (admin)".to_string(),
    };
    app.update(Action::ProfileSwitchResult(Ok(ctx)));

    // Server info should be cleared
    assert!(app.server_version.is_none());
    assert!(app.server_build.is_none());
}

// ============================================================================
// ClearAllData Tests
// ============================================================================

#[test]
fn test_clear_all_data_clears_cached_data() {
    let mut app = App::new(None, ConnectionContext::default());

    // Populate some data
    app.indexes = Some(vec![]);
    app.jobs = Some(vec![]);
    app.saved_searches = Some(vec![]);
    app.internal_logs = Some(vec![]);
    // cluster_info and health_info don't have Default impl, use None
    app.cluster_info = None;
    app.cluster_peers = Some(vec![]);
    app.health_info = None;
    app.apps = Some(vec![]);
    app.users = Some(vec![]);
    app.search_results = vec![serde_json::json!({"test": "data"})];
    app.search_sid = Some("test_sid".to_string());
    app.search_results_total_count = Some(100);
    app.search_has_more_results = true;

    // Clear all data
    app.update(Action::ClearAllData);

    // All data should be cleared
    assert!(app.indexes.is_none());
    assert!(app.jobs.is_none());
    assert!(app.saved_searches.is_none());
    assert!(app.internal_logs.is_none());
    assert!(app.cluster_info.is_none());
    assert!(app.cluster_peers.is_none());
    assert!(app.health_info.is_none());
    assert!(app.apps.is_none());
    assert!(app.users.is_none());
    assert!(app.search_results.is_empty());
    assert!(app.search_sid.is_none());
    assert!(app.search_results_total_count.is_none());
    assert!(!app.search_has_more_results);
}

#[test]
fn test_clear_all_data_resets_list_states() {
    let mut app = App::new(None, ConnectionContext::default());

    // Set non-zero selections
    app.indexes_state.select(Some(5));
    app.jobs_state.select(Some(3));
    app.saved_searches_state.select(Some(2));

    // Clear all data
    app.update(Action::ClearAllData);

    // List states should be reset to 0
    assert_eq!(app.indexes_state.selected(), Some(0));
    assert_eq!(app.jobs_state.selected(), Some(0));
    assert_eq!(app.saved_searches_state.selected(), Some(0));
}

// ============================================================================
// Settings Screen Keybinding Tests
// ============================================================================

#[test]
fn test_settings_screen_opens_profile_switcher() {
    use splunk_tui::CurrentScreen;

    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Settings;

    // Press 'p' to open profile switcher
    let action = app.handle_input(key('p'));

    // Should emit OpenProfileSwitcher action
    assert!(matches!(action, Some(Action::OpenProfileSwitcher)));
}

// ============================================================================
// Screen Reload Tests
// ============================================================================

#[test]
fn test_load_action_for_screen_after_profile_switch() {
    use splunk_tui::CurrentScreen;

    let mut app = App::new(None, ConnectionContext::default());

    // Test that each screen returns the correct load action
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

    app.current_screen = CurrentScreen::Cluster;
    assert!(matches!(
        app.load_action_for_screen(),
        Some(Action::LoadClusterInfo)
    ));

    app.current_screen = CurrentScreen::Health;
    assert!(matches!(
        app.load_action_for_screen(),
        Some(Action::LoadHealth)
    ));

    app.current_screen = CurrentScreen::Settings;
    assert!(matches!(
        app.load_action_for_screen(),
        Some(Action::SwitchToSettings)
    ));

    // Search screen should return None (no pre-loading needed)
    app.current_screen = CurrentScreen::Search;
    assert!(app.load_action_for_screen().is_none());
}

#[test]
fn test_profile_switch_updates_context_for_header() {
    use splunk_tui::CurrentScreen;

    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Indexes;

    // Simulate having some data loaded
    app.indexes = Some(vec![]);
    app.server_version = Some("9.0.0".to_string());

    // Simulate successful profile switch
    let ctx = ConnectionContext {
        profile_name: Some("new_profile".to_string()),
        base_url: "https://new.splunk.com:8089".to_string(),
        auth_mode: "token".to_string(),
    };
    app.update(Action::ProfileSwitchResult(Ok(ctx)));

    // Verify context is updated
    assert_eq!(app.profile_name, Some("new_profile".to_string()));
    assert_eq!(
        app.base_url,
        Some("https://new.splunk.com:8089".to_string())
    );

    // Server info should be cleared until new health check loads
    assert!(app.server_version.is_none());
}
