//! Tests for error handling, message tests, and error context helpers.
//!
//! This module tests:
//! - Cancel/delete job error clears loading state (RQ-0021)
//! - Toast notification and pruning
//! - Progress updates
//! - Error message mapping for different error types
//! - Error context building with query, operation, SID
//! - Unified auth/TLS error classification across TUI flows (RQ-0455)
//!
//! ## Invariants
//! - Loading state must be cleared on error
//! - Error toasts must have Error level
//! - Error messages must map to user-friendly strings
//! - Equivalent auth/TLS failures render consistent messaging across flows

use splunk_tui::{
    ToastLevel, action::Action, app::App, app::ConnectionContext, error_details,
    error_details::AuthRecoveryKind,
};
use std::sync::Arc;

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
    use std::time::{Duration, Instant};

    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = splunk_tui::CurrentScreen::Jobs;

    // Add a toast
    app.toasts
        .push(splunk_tui::Toast::error("Test".to_string()));

    // Manually expire it
    app.toasts[0].ttl = Duration::from_millis(1);
    app.toasts[0].created_at = Instant::now() - Duration::from_secs(1);

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
    let error = splunk_client::ClientError::OperationTimeout {
        operation: "search",
        timeout: std::time::Duration::from_secs(300),
    };
    let message = error_details::search_error_message(&error);
    assert_eq!(
        message, "Request timeout",
        "Timeout should map to 'Request timeout' (unified classifier)"
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
fn test_search_error_message_tls_error() {
    let error = splunk_client::ClientError::TlsError("certificate verify failed".to_string());
    let message = error_details::search_error_message(&error);
    assert_eq!(
        message, "TLS certificate error",
        "TlsError should map to 'TLS certificate error' (unified classifier)"
    );
}

#[test]
fn test_search_error_message_unauthorized() {
    let error = splunk_client::ClientError::Unauthorized("Access denied".to_string());
    let message = error_details::search_error_message(&error);
    assert_eq!(
        message, "Access denied",
        "Unauthorized should map to 'Access denied' (unified classifier)"
    );
}

#[test]
fn test_build_search_error_details_includes_all_context() {
    let error = splunk_client::ClientError::OperationTimeout {
        operation: "search",
        timeout: std::time::Duration::from_secs(300),
    };
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

// Unified Error Classification Tests (RQ-0455)

/// Test that auth errors produce consistent messaging across all flows
#[test]
fn test_auth_error_consistency_across_flows() {
    // Same auth error used in different contexts
    let auth_error = splunk_client::ClientError::AuthFailed("Invalid credentials".to_string());

    // Search flow
    let search_details = error_details::ErrorDetails::from_client_error(&auth_error);
    let search_message = error_details::search_error_message(&auth_error);

    // Data loading flow (uses same classifier)
    let data_load_details = error_details::ErrorDetails::from_client_error(&auth_error);

    // All flows should produce the same title/summary
    assert_eq!(search_message, "Authentication failed");
    assert_eq!(search_details.summary, "Authentication failed");
    assert_eq!(data_load_details.summary, "Authentication failed");

    // All should have auth recovery info
    assert!(search_details.auth_recovery.is_some());
    assert!(data_load_details.auth_recovery.is_some());

    // Recovery kind should be consistent
    assert_eq!(
        search_details.auth_recovery.as_ref().unwrap().kind,
        AuthRecoveryKind::InvalidCredentials
    );
    assert_eq!(
        data_load_details.auth_recovery.as_ref().unwrap().kind,
        AuthRecoveryKind::InvalidCredentials
    );

    // Status code should be consistent
    assert_eq!(search_details.status_code, Some(401));
    assert_eq!(data_load_details.status_code, Some(401));
}

/// Test that session expired errors are consistent across flows
#[test]
fn test_session_expired_consistency_across_flows() {
    let session_error = splunk_client::ClientError::SessionExpired {
        username: "admin".to_string(),
    };

    let details = error_details::ErrorDetails::from_client_error(&session_error);
    let message = error_details::search_error_message(&session_error);

    assert_eq!(message, "Session expired");
    assert_eq!(details.summary, "Session expired");
    assert_eq!(details.status_code, Some(401));

    let recovery = details.auth_recovery.expect("Should have auth recovery");
    assert_eq!(recovery.kind, AuthRecoveryKind::SessionExpired);
    assert!(recovery.diagnosis.contains("admin"));
}

/// Test that TLS errors produce consistent recovery guidance
#[test]
fn test_tls_error_consistency_across_flows() {
    let tls_error = splunk_client::ClientError::TlsError("certificate verify failed".to_string());

    let details = error_details::ErrorDetails::from_client_error(&tls_error);
    let message = error_details::search_error_message(&tls_error);

    assert_eq!(message, "TLS certificate error");
    assert_eq!(details.summary, "TLS certificate error");

    let recovery = details.auth_recovery.expect("Should have auth recovery");
    assert_eq!(recovery.kind, AuthRecoveryKind::TlsOrCertificate);
    assert!(recovery.diagnosis.contains("certificate"));
    // Should have specific guidance for certificate issues
    assert!(
        recovery
            .next_steps
            .iter()
            .any(|s| s.contains("certificate"))
    );
}

/// Test that API error 401 is classified as auth error
#[test]
fn test_api_error_401_classification() {
    let api_error = splunk_client::ClientError::ApiError {
        status: 401,
        url: "https://localhost:8089/services".to_string(),
        message: "Unauthorized".to_string(),
        request_id: Some("req-123".to_string()),
    };

    let details = error_details::ErrorDetails::from_client_error(&api_error);

    // Should be classified as auth error
    assert!(details.auth_recovery.is_some());
    assert_eq!(
        details.auth_recovery.unwrap().kind,
        AuthRecoveryKind::InvalidCredentials
    );
    assert_eq!(details.status_code, Some(401));
    // Should preserve request ID
    assert_eq!(details.request_id, Some("req-123".to_string()));
}

/// Test that API error 403 is classified as auth error
#[test]
fn test_api_error_403_classification() {
    let api_error = splunk_client::ClientError::ApiError {
        status: 403,
        url: "https://localhost:8089/services".to_string(),
        message: "Forbidden".to_string(),
        request_id: None,
    };

    let details = error_details::ErrorDetails::from_client_error(&api_error);

    assert!(details.auth_recovery.is_some());
    assert_eq!(
        details.auth_recovery.unwrap().kind,
        AuthRecoveryKind::InvalidCredentials
    );
    assert_eq!(details.status_code, Some(403));
}

/// Test connection error classification
#[test]
fn test_connection_error_classification() {
    let conn_error = splunk_client::ClientError::ConnectionRefused("localhost:8089".to_string());

    let details = error_details::ErrorDetails::from_client_error(&conn_error);
    let message = error_details::search_error_message(&conn_error);

    assert_eq!(message, "Connection refused");
    assert!(details.auth_recovery.is_some());
    assert_eq!(
        details.auth_recovery.unwrap().kind,
        AuthRecoveryKind::ConnectionRefused
    );
}

/// Test timeout error classification
#[test]
fn test_timeout_error_classification() {
    let timeout_error = splunk_client::ClientError::OperationTimeout {
        operation: "search",
        timeout: std::time::Duration::from_secs(30),
    };

    let details = error_details::ErrorDetails::from_client_error(&timeout_error);
    let message = error_details::search_error_message(&timeout_error);

    assert_eq!(message, "Request timeout");
    assert!(details.auth_recovery.is_some());
    assert_eq!(
        details.auth_recovery.unwrap().kind,
        AuthRecoveryKind::Timeout
    );
}

/// Test that ClientError::is_auth_error includes ApiError 401/403
#[test]
fn test_is_auth_error_includes_api_errors() {
    let err_401 = splunk_client::ClientError::ApiError {
        status: 401,
        url: "https://localhost:8089".to_string(),
        message: "Unauthorized".to_string(),
        request_id: None,
    };
    assert!(err_401.is_auth_error(), "401 should be auth error");

    let err_403 = splunk_client::ClientError::ApiError {
        status: 403,
        url: "https://localhost:8089".to_string(),
        message: "Forbidden".to_string(),
        request_id: None,
    };
    assert!(err_403.is_auth_error(), "403 should be auth error");

    let err_500 = splunk_client::ClientError::ApiError {
        status: 500,
        url: "https://localhost:8089".to_string(),
        message: "Server Error".to_string(),
        request_id: None,
    };
    assert!(!err_500.is_auth_error(), "500 should not be auth error");
}

/// Test unified user-facing failure structure
#[test]
fn test_user_facing_failure_structure() {
    let error = splunk_client::ClientError::AuthFailed("test".to_string());
    let failure = error.to_user_facing_failure();

    assert_eq!(
        failure.category,
        splunk_client::FailureCategory::AuthInvalidCredentials
    );
    assert_eq!(failure.title, "Authentication failed");
    assert!(!failure.diagnosis.is_empty());
    assert!(!failure.action_hints.is_empty());
    assert_eq!(failure.status_code, Some(401));
}

/// Test that data loading errors set current_error (regression test for RQ-0455)
#[test]
fn test_data_load_error_sets_current_error() {
    let mut app = App::new(None, ConnectionContext::default());

    let error = Arc::new(splunk_client::ClientError::AuthFailed("test".to_string()));
    app.handle_data_loading_action(Action::IndexesLoaded(Err(error)));

    assert!(app.current_error.is_some(), "Should set current_error");
    assert!(!app.loading, "Should clear loading state");
    assert_eq!(app.toasts.len(), 1, "Should show toast");
}

/// Test that search errors set current_error and trigger auth recovery popup
#[test]
fn test_search_error_sets_current_error() {
    let mut app = App::new(None, ConnectionContext::default());

    let error = splunk_client::ClientError::AuthFailed("test".to_string());
    let details = error_details::ErrorDetails::from_client_error(&error);

    app.handle_search_action(Action::SearchComplete(Err((
        "Search failed".to_string(),
        details,
    ))));

    assert!(app.current_error.is_some(), "Should set current_error");
    assert!(!app.loading, "Should clear loading state");
    assert_eq!(app.toasts.len(), 1, "Should show toast");
}

/// Test that profile switch error sets current_error with unified classification (RQ-0455)
#[test]
fn test_profile_switch_error_sets_current_error() {
    use splunk_client::ClientError;

    let mut app = App::new(None, ConnectionContext::default());

    // Use ClientError for unified classification (not just a string)
    let error = Arc::new(ClientError::AuthFailed("Invalid credentials".to_string()));
    app.handle_profile_action(Action::ProfileSwitchResult(Err(error)));

    assert!(app.current_error.is_some(), "Should set current_error");
    assert_eq!(app.toasts.len(), 1, "Should show toast");
    assert!(
        app.toasts[0].message.contains("Failed to switch profile"),
        "Toast should have correct message"
    );
    assert!(
        app.toasts[0].message.contains("Authentication failed"),
        "Toast should use unified error title"
    );
    // Should have auth recovery for auth errors
    let recovery = app.current_error.as_ref().unwrap().auth_recovery.as_ref();
    assert!(
        recovery.is_some(),
        "Should have auth recovery for auth errors"
    );
    assert_eq!(
        recovery.unwrap().kind,
        AuthRecoveryKind::InvalidCredentials,
        "Should classify as InvalidCredentials"
    );
    // Should clear loading state
    assert!(!app.loading, "Should clear loading state");
}
