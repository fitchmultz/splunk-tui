//! Tests for error handling action redaction.

use crate::action::tests::redacted_debug;
use crate::action::variants::Action;

#[test]
fn test_redact_notify() {
    let action = Action::Notify(
        crate::ui::ToastLevel::Error,
        "Failed to authenticate: invalid token xyz789".to_string(),
    );
    let output = redacted_debug(&action);

    assert!(!output.contains("xyz789"), "Should not contain token");
    assert!(output.contains("Notify"), "Should contain action name");
    assert!(output.contains("Error"), "Should contain toast level");
    assert!(output.contains("chars"), "Should show size indicator");
}

#[test]
fn test_redact_show_error_details() {
    let details = crate::error_details::ErrorDetails::from_error_string(
        "Authentication failed for user admin with password secret123",
    );
    let action = Action::ShowErrorDetails(details);
    let output = redacted_debug(&action);

    assert!(
        !output.contains("Authentication failed"),
        "Should not contain error message"
    );
    assert!(!output.contains("admin"), "Should not contain user name");
    assert!(!output.contains("secret123"), "Should not contain password");
    assert!(
        output.contains("ShowErrorDetails"),
        "Should contain action name"
    );
    assert!(
        output.contains("<redacted>"),
        "Should show redacted indicator"
    );
}

#[test]
fn test_show_error_details_from_current() {
    let action = Action::ShowErrorDetailsFromCurrent;
    let output = redacted_debug(&action);

    assert!(
        output.contains("ShowErrorDetailsFromCurrent"),
        "Should contain action name"
    );
}
