//! Purpose: Regression coverage for shared client error classification and rendering.
//! Responsibilities: Validate retryability, auth classification, status classification, user-facing failures, and clone-safe HTTP error behavior.
//! Scope: Unit tests for `splunk_client::error` only.
//! Usage: Runs under `cargo test -p splunk-client`.
//! Invariants/Assumptions: Tests assert semantic behavior rather than file layout so the split remains refactor-friendly.

use super::*;
use std::time::Duration;

#[test]
fn retryable_errors_include_connection_refused() {
    assert!(ClientError::ConnectionRefused("localhost:8089".to_string()).is_retryable());
    assert!(
        ClientError::OperationTimeout {
            operation: "test",
            timeout: Duration::from_secs(1),
        }
        .is_retryable()
    );
    assert!(!ClientError::AuthFailed("bad creds".to_string()).is_retryable());
}

#[test]
fn auth_error_detection_handles_semantic_permission_failures() {
    assert!(ClientError::AuthFailed("bad creds".to_string()).is_auth_error());
    assert!(
        ClientError::SessionExpired {
            username: "admin".to_string(),
        }
        .is_auth_error()
    );
    assert!(ClientError::Unauthorized("forbidden".to_string()).is_auth_error());
    assert!(
        !ClientError::ApiError {
            status: 403,
            url: "https://localhost:8089".to_string(),
            message: "Forbidden".to_string(),
            request_id: None,
        }
        .is_auth_error()
    );
}

#[test]
fn retryable_statuses_match_contract() {
    assert!(ClientError::is_retryable_status(429));
    assert!(ClientError::is_retryable_status(502));
    assert!(ClientError::is_retryable_status(503));
    assert!(ClientError::is_retryable_status(504));
    assert!(!ClientError::is_retryable_status(400));
    assert!(!ClientError::is_retryable_status(401));
    assert!(!ClientError::is_retryable_status(403));
    assert!(!ClientError::is_retryable_status(404));
}

#[test]
fn status_response_classification_uses_semantic_403() {
    let error = ClientError::from_status_response(
        403,
        "https://localhost:8089".to_string(),
        "Forbidden".to_string(),
        Some("req-123".to_string()),
    );

    assert!(matches!(error, ClientError::Unauthorized(_)));
}

#[test]
fn status_response_classification_preserves_not_found_and_invalid_request() {
    assert!(matches!(
        ClientError::from_status_response(
            404,
            "https://localhost:8089/services/jobs/123".to_string(),
            "not found".to_string(),
            None
        ),
        ClientError::NotFound(_)
    ));
    assert!(matches!(
        ClientError::from_status_response(
            400,
            "https://localhost:8089".to_string(),
            "bad request".to_string(),
            None
        ),
        ClientError::InvalidRequest(_)
    ));
}

#[test]
fn user_facing_failure_maps_unauthorized_to_permission_category() {
    let failure = ClientError::Unauthorized("forbidden".to_string()).to_user_facing_failure();
    assert_eq!(
        failure.category,
        FailureCategory::AuthInsufficientPermissions
    );
    assert_eq!(failure.title, "Access denied");
    assert_eq!(failure.status_code, Some(403));
}

#[test]
fn user_facing_failure_maps_api_error_401() {
    let failure = ClientError::ApiError {
        status: 401,
        url: "https://localhost:8089".to_string(),
        message: "Unauthorized".to_string(),
        request_id: Some("req-456".to_string()),
    }
    .to_user_facing_failure();

    assert_eq!(failure.category, FailureCategory::AuthInvalidCredentials);
    assert_eq!(failure.title, "Authentication required");
    assert_eq!(failure.status_code, Some(401));
    assert_eq!(failure.request_id.as_deref(), Some("req-456"));
}

#[test]
fn user_facing_failure_preserves_operation_name_for_timeouts() {
    let failure = ClientError::OperationTimeout {
        operation: "fetch_jobs",
        timeout: Duration::from_secs(60),
    }
    .to_user_facing_failure();

    assert_eq!(failure.category, FailureCategory::Timeout);
    assert!(failure.diagnosis.contains("fetch_jobs"));
    assert!(
        failure
            .action_hints
            .iter()
            .any(|hint| hint.contains("fetch_jobs"))
    );
}

#[test]
fn rollback_failure_display_includes_operation_and_resource() {
    let failure = RollbackFailure {
        resource_name: "test_index".to_string(),
        operation: "delete_index".to_string(),
        error: ClientError::NotFound("test_index".to_string()),
    };

    let display = failure.to_string();
    assert!(display.contains("delete_index"));
    assert!(display.contains("test_index"));
    assert!(display.contains("rollback"));
}

#[test]
fn with_username_only_replaces_unknown_placeholder() {
    let replaced = ClientError::SessionExpired {
        username: "unknown".to_string(),
    }
    .with_username("admin");
    assert!(matches!(
        replaced,
        ClientError::SessionExpired { ref username } if username == "admin"
    ));

    let preserved = ClientError::SessionExpired {
        username: "existing_user".to_string(),
    }
    .with_username("admin");
    assert!(matches!(
        preserved,
        ClientError::SessionExpired { ref username } if username == "existing_user"
    ));
}

#[test]
fn http_error_snapshot_clone_is_lossless() {
    let reqwest_error = reqwest::Client::new()
        .get("http://[::1")
        .build()
        .expect_err("invalid URL should fail request build");
    let snapshot = HttpErrorSnapshot::from_reqwest_error(&reqwest_error);

    let cloned = snapshot.clone();
    assert_eq!(snapshot.to_string(), cloned.to_string());
    assert_eq!(snapshot.status(), cloned.status());
    assert_eq!(snapshot.url(), cloned.url());
}

#[test]
fn http_error_variant_clone_preserves_variant() {
    let reqwest_error = reqwest::Client::new()
        .get("http://[::1")
        .build()
        .expect_err("invalid URL should fail request build");
    let error = ClientError::HttpError(HttpErrorSnapshot::from_reqwest_error(&reqwest_error));

    assert!(matches!(error.clone(), ClientError::HttpError(_)));
}
