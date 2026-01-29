//! Tests for error handling, message tests, and error context helpers.
//!
//! This module tests:
//! - Cancel/delete job error clears loading state (RQ-0021)
//! - Toast notification and pruning
//! - Progress updates
//! - Error message mapping for different error types
//! - Error context building with query, operation, SID
//!
//! ## Invariants
//! - Loading state must be cleared on error
//! - Error toasts must have Error level
//! - Error messages must map to user-friendly strings
//!
//! ## Test Organization
//! Tests are grouped by: loading state, toasts, error messages, error context.

use splunk_tui::{ToastLevel, action::Action, app::App, app::ConnectionContext, error_details};

// Regression tests for RQ-0021: loading state should be cleared on error

#[test]
fn test_cancel_job_error_clears_loading() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = splunk_tui::CurrentScreen::Jobs;

    // Simulate the sequence of actions when CancelJob fails:
    // 1. Loading(true) is sent before the async operation
    app.update(Action::Loading(true));
    assert!(app.loading, "Loading should be true after Loading(true)");

    // 2. Error notification is sent when operation fails
    app.update(Action::Notify(
        ToastLevel::Error,
        "Failed to cancel job: connection error".to_string(),
    ));

    // 3. Loading(false) is sent to clear the loading state
    app.update(Action::Loading(false));

    assert!(!app.loading, "Loading should be false after Loading(false)");
    assert_eq!(app.toasts.len(), 1, "Should have error toast");
    assert_eq!(
        app.toasts[0].level,
        ToastLevel::Error,
        "Toast should be Error level"
    );
}

#[test]
fn test_delete_job_error_clears_loading() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = splunk_tui::CurrentScreen::Jobs;

    // Simulate the sequence of actions when DeleteJob fails:
    // 1. Loading(true) is sent before the async operation
    app.update(Action::Loading(true));
    assert!(app.loading, "Loading should be true after Loading(true)");

    // 2. Error notification is sent when operation fails
    app.update(Action::Notify(
        ToastLevel::Error,
        "Failed to delete job: not found".to_string(),
    ));

    // 3. Loading(false) is sent to clear the loading state
    app.update(Action::Loading(false));

    assert!(!app.loading, "Loading should be false after Loading(false)");
    assert_eq!(app.toasts.len(), 1, "Should have error toast");
    assert_eq!(
        app.toasts[0].level,
        ToastLevel::Error,
        "Toast should be Error level"
    );
}

#[test]
fn test_notify_adds_toast() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = splunk_tui::CurrentScreen::Jobs;

    // Add a toast notification
    app.update(Action::Notify(ToastLevel::Error, "Test error".to_string()));

    assert_eq!(app.toasts.len(), 1, "Should have one toast");
    assert_eq!(
        app.toasts[0].message, "Test error",
        "Toast message should match"
    );
    assert_eq!(
        app.toasts[0].level,
        ToastLevel::Error,
        "Toast level should be Error"
    );
}

#[test]
fn test_tick_prunes_expired_toasts() {
    use std::time::Duration;

    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = splunk_tui::CurrentScreen::Jobs;

    // Add a toast
    app.toasts
        .push(splunk_tui::Toast::error("Test".to_string()));

    // Manually expire it
    app.toasts[0].ttl = Duration::from_millis(1);
    std::thread::sleep(Duration::from_millis(10));

    // Tick should prune expired toasts
    app.update(Action::Tick);

    assert!(app.toasts.is_empty(), "Expired toasts should be pruned");
}

#[test]
fn test_progress_update() {
    let mut app = App::new(None, ConnectionContext::default());

    // Update progress
    app.update(Action::Progress(0.75));

    assert_eq!(app.progress, 0.75, "Progress should be updated");
}

// Error Context Helper Tests (RQ-0128)

#[test]
fn test_search_error_message_timeout() {
    let error = splunk_client::ClientError::Timeout(std::time::Duration::from_secs(300));
    let message = error_details::search_error_message(&error);
    assert_eq!(
        message, "Search timeout",
        "Timeout should map to 'Search timeout'"
    );
}

#[test]
fn test_search_error_message_auth_failed() {
    let error = splunk_client::ClientError::AuthFailed("Invalid credentials".to_string());
    let message = error_details::search_error_message(&error);
    assert_eq!(
        message, "Authentication failed",
        "AuthFailed should map to 'Authentication failed'"
    );
}

#[test]
fn test_search_error_message_session_expired() {
    let error = splunk_client::ClientError::SessionExpired {
        username: "admin".to_string(),
    };
    let message = error_details::search_error_message(&error);
    assert_eq!(
        message, "Session expired",
        "SessionExpired should map to 'Session expired'"
    );
}

#[test]
fn test_search_error_message_rate_limited() {
    let error = splunk_client::ClientError::RateLimited(Some(std::time::Duration::from_secs(60)));
    let message = error_details::search_error_message(&error);
    assert_eq!(
        message, "Rate limited",
        "RateLimited should map to 'Rate limited'"
    );
}

#[test]
fn test_search_error_message_connection_refused() {
    let error = splunk_client::ClientError::ConnectionRefused("localhost:8089".to_string());
    let message = error_details::search_error_message(&error);
    assert_eq!(
        message, "Connection refused",
        "ConnectionRefused should map to 'Connection refused'"
    );
}

#[test]
fn test_build_search_error_details_includes_all_context() {
    let error = splunk_client::ClientError::Timeout(std::time::Duration::from_secs(300));
    let details = error_details::build_search_error_details(
        &error,
        "index=_internal | head 10".to_string(),
        "search_with_progress".to_string(),
        Some("test_sid_123".to_string()),
    );

    assert_eq!(
        details.context.get("query"),
        Some(&"index=_internal | head 10".to_string()),
        "Should include query in context"
    );
    assert_eq!(
        details.context.get("operation"),
        Some(&"search_with_progress".to_string()),
        "Should include operation in context"
    );
    assert_eq!(
        details.context.get("sid"),
        Some(&"test_sid_123".to_string()),
        "Should include SID in context"
    );
}

#[test]
fn test_build_search_error_details_with_rate_limited() {
    // RateLimited takes Option<Duration>
    let error = splunk_client::ClientError::RateLimited(Some(std::time::Duration::from_secs(60)));
    let details = error_details::build_search_error_details(
        &error,
        "search *".to_string(),
        "search_with_progress".to_string(),
        Some("test_sid_456".to_string()),
    );

    assert_eq!(
        details.context.get("query"),
        Some(&"search *".to_string()),
        "Should include query in context"
    );
    assert_eq!(
        details.context.get("operation"),
        Some(&"search_with_progress".to_string()),
        "Should include operation in context"
    );
}

#[test]
fn test_build_search_error_details_without_sid() {
    let error = splunk_client::ClientError::AuthFailed("Invalid token".to_string());
    let details = error_details::build_search_error_details(
        &error,
        "search *".to_string(),
        "create_search_job".to_string(),
        None,
    );

    assert_eq!(
        details.context.get("query"),
        Some(&"search *".to_string()),
        "Should include query in context"
    );
    assert_eq!(
        details.context.get("operation"),
        Some(&"create_search_job".to_string()),
        "Should include operation in context"
    );
    assert!(
        !details.context.contains_key("sid"),
        "Should not include SID when None"
    );
}
