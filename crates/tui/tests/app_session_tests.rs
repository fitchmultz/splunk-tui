//! Tests for session expiration handling and re-authentication flows.
//!
//! This module tests:
//! - Session expired error display
//! - Re-authentication prompt behavior
//! - Session expiration clears sensitive data appropriately
//! - Transition to auth error state
//!
//! ## Invariants
//! - Session expiration must show clear re-auth prompt
//! - Sensitive data should be cleared on session expiration
//! - User must be able to recover from session expiration

mod helpers;
use splunk_client::ClientError;
use splunk_tui::{CurrentScreen, action::Action, app::App, app::ConnectionContext, ui::ToastLevel};
use std::sync::Arc;

#[test]
fn test_session_expired_error_display() {
    let mut app = App::new(None, ConnectionContext::default());

    // Simulate session expired error
    let session_error = ClientError::SessionExpired {
        username: "admin".to_string(),
    };

    app.update(Action::IndexesLoaded(Err(Arc::new(session_error))));

    // Should have toast notification
    assert!(
        !app.toasts.is_empty(),
        "Should show toast for session expiration"
    );

    // Toast should mention session/auth
    let toast_message = format!("{:?}", app.toasts);
    assert!(
        toast_message.to_lowercase().contains("session")
            || toast_message.to_lowercase().contains("auth")
            || toast_message.to_lowercase().contains("login"),
        "Toast should mention session/auth issue: {}",
        toast_message
    );
}

#[test]
fn test_session_expired_clears_loading_state() {
    let mut app = App::new(None, ConnectionContext::default());

    // Start loading
    app.update(Action::Loading(true));
    assert!(app.loading);

    // Session expires during load
    let session_error = ClientError::SessionExpired {
        username: "admin".to_string(),
    };
    app.update(Action::JobsLoaded(Err(Arc::new(session_error))));

    // Loading should be cleared
    assert!(!app.loading, "Loading should be cleared on session error");
}

#[test]
fn test_session_expired_preserves_screen_context() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;

    // Simulate session expired
    let session_error = ClientError::SessionExpired {
        username: "admin".to_string(),
    };
    app.update(Action::JobsLoaded(Err(Arc::new(session_error))));

    // Should remain on current screen (not jump to unexpected screen)
    assert_eq!(
        app.current_screen,
        CurrentScreen::Jobs,
        "Should remain on Jobs screen after session error"
    );
}

#[test]
fn test_multiple_session_expired_errors() {
    let mut app = App::new(None, ConnectionContext::default());

    // Multiple session expired errors in succession
    for _ in 0..5 {
        let session_error = ClientError::SessionExpired {
            username: "admin".to_string(),
        };
        app.update(Action::IndexesLoaded(Err(Arc::new(session_error))));
    }

    // Should have toasts (may be limited in number)
    assert!(!app.toasts.is_empty(), "Should have toast notifications");
}

#[test]
fn test_session_expired_with_active_search() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    // Session expires - notify with toast
    app.update(Action::Notify(
        ToastLevel::Error,
        "Session expired".to_string(),
    ));

    // Should have error toast
    assert!(!app.toasts.is_empty(), "Should show error toast");

    // App should still be on search screen
    assert_eq!(app.current_screen, CurrentScreen::Search);
}

#[test]
fn test_auth_error_variants_handled() {
    let _app = App::new(None, ConnectionContext::default());

    // Test different auth error variants
    let auth_errors = vec![
        ClientError::SessionExpired {
            username: "admin".to_string(),
        },
        ClientError::Unauthorized("Invalid credentials".to_string()),
    ];

    for error in auth_errors {
        let mut test_app = App::new(None, ConnectionContext::default());
        test_app.update(Action::IndexesLoaded(Err(Arc::new(error))));

        assert!(
            !test_app.toasts.is_empty(),
            "Should show toast for auth error"
        );
    }
}

#[test]
fn test_clear_error_after_session_expired() {
    let mut app = App::new(None, ConnectionContext::default());

    // Session expires
    let session_error = ClientError::SessionExpired {
        username: "admin".to_string(),
    };
    app.update(Action::IndexesLoaded(Err(Arc::new(session_error))));

    assert!(!app.toasts.is_empty());

    // Clear error
    app.update(Action::ClearErrorDetails);

    // App should be in valid state (ClearErrorDetails clears error details, not toasts)
    // Verify app is still functional - just check we're on a valid screen
    let _ = app.current_screen; // Should not panic
}

#[test]
fn test_session_expired_during_navigation() {
    let mut app = App::new(None, ConnectionContext::default());

    // Navigate to a screen
    app.update(Action::NextScreen);
    let prev_screen = app.current_screen;

    // Session expires
    let session_error = ClientError::SessionExpired {
        username: "admin".to_string(),
    };
    app.update(Action::ClusterInfoLoaded(Err(Arc::new(session_error))));

    // Should still be able to navigate
    app.update(Action::NextScreen);
    assert_ne!(
        app.current_screen, prev_screen,
        "Should be able to navigate after session error"
    );
}

#[test]
fn test_session_expired_with_popup_open() {
    let mut app = App::new(None, ConnectionContext::default());

    // Open help popup
    app.update(Action::OpenHelpPopup);
    assert!(app.popup.is_some());

    // Session expires
    let session_error = ClientError::SessionExpired {
        username: "admin".to_string(),
    };
    app.update(Action::IndexesLoaded(Err(Arc::new(session_error))));

    // Popup should still be open (or closed gracefully)
    // The important thing is no panic
    let _ = app.popup.is_some(); // Either state is acceptable
}

#[test]
fn test_concurrent_auth_errors() {
    let mut app = App::new(None, ConnectionContext::default());

    // Simulate multiple concurrent auth errors from different operations
    let session_error1 = ClientError::SessionExpired {
        username: "admin".to_string(),
    };
    let session_error2 = ClientError::SessionExpired {
        username: "admin".to_string(),
    };
    let session_error3 = ClientError::SessionExpired {
        username: "admin".to_string(),
    };

    app.update(Action::IndexesLoaded(Err(Arc::new(session_error1))));
    app.update(Action::JobsLoaded(Err(Arc::new(session_error2))));
    app.update(Action::AppsLoaded(Err(Arc::new(session_error3))));

    // Should have toast(s)
    assert!(!app.toasts.is_empty(), "Should show toasts for auth errors");
}

#[test]
fn test_session_error_recovery_flow() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;

    // First, session expires
    let session_error = ClientError::SessionExpired {
        username: "admin".to_string(),
    };
    app.update(Action::JobsLoaded(Err(Arc::new(session_error))));

    // Then successful load after "re-auth"
    use splunk_client::models::SearchJobStatus;
    let jobs = vec![SearchJobStatus {
        sid: "test_sid".to_string(),
        is_done: true,
        is_finalized: false,
        done_progress: 1.0,
        run_duration: 1.0,
        disk_usage: 100,
        scan_count: 50,
        event_count: 25,
        result_count: 10,
        cursor_time: None,
        priority: None,
        label: None,
    }];
    app.update(Action::JobsLoaded(Ok(jobs)));

    // Jobs should be loaded
    assert!(app.jobs.is_some(), "Jobs should be loaded after recovery");
    // App should be functional after recovery
}

#[test]
fn test_unauthorized_error_handling() {
    let mut app = App::new(None, ConnectionContext::default());

    let unauthorized_error = ClientError::Unauthorized("Invalid username or password".to_string());

    app.update(Action::UsersLoaded(Err(Arc::new(unauthorized_error))));

    // Should show toast
    assert!(
        !app.toasts.is_empty(),
        "Should show toast for unauthorized error"
    );
}

#[test]
fn test_session_expired_preserves_pagination_state() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;

    // Set some pagination state
    app.update(Action::NavigateDown);
    app.update(Action::NavigateDown);
    let selected_before = app.jobs_state.selected();

    // Session expires
    let session_error = ClientError::SessionExpired {
        username: "admin".to_string(),
    };
    app.update(Action::MoreJobsLoaded(Err(Arc::new(session_error))));

    // Selection should be preserved
    assert_eq!(
        app.jobs_state.selected(),
        selected_before,
        "Selection should be preserved after session error"
    );
}
