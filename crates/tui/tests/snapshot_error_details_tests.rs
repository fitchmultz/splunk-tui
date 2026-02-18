//! Snapshot tests for ErrorDetails popup rendering.
//!
//! Provides regression coverage for error display UX across various
//! error scenarios including auth failures, API errors, and network issues.

mod helpers;

use helpers::TuiHarness;
use splunk_tui::Popup;
use splunk_tui::error_details::{AuthRecoveryDetails, AuthRecoveryKind, ErrorDetails};
use splunk_tui::ui::popup::PopupType;
use std::collections::HashMap;

fn create_error_details_popup() -> splunk_tui::ui::popup::Popup {
    Popup::builder(PopupType::ErrorDetails).build()
}

#[test]
fn snapshot_error_details_basic() {
    // Using very large terminal to avoid buffer overflow in popup rendering
    let mut harness = TuiHarness::new(120, 50);
    let details = ErrorDetails::from_error_string("Connection refused");
    harness.app.current_error = Some(details);
    harness.app.popup = Some(create_error_details_popup());
    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_error_details_with_context() {
    // Using very large terminal to avoid buffer overflow in popup rendering
    // Note: This test intentionally avoids context HashMap to prevent ordering flakiness
    // Context ordering is tested in integration tests, not snapshots
    let mut harness = TuiHarness::new(120, 50);
    let mut details = ErrorDetails::from_error_string("Authentication failed");
    details.status_code = Some(401);
    details.url = Some("https://localhost:8089/services/search/jobs".to_string());
    details.request_id = Some("req-abc123".to_string());
    // Empty context to avoid HashMap ordering issues
    details.context = HashMap::new();
    details.auth_recovery = Some(AuthRecoveryDetails {
        kind: AuthRecoveryKind::InvalidCredentials,
        diagnosis: "The provided credentials were not accepted by the Splunk server.".to_string(),
        next_steps: vec![
            "Check your username and password".to_string(),
            "Verify the API token if using token auth".to_string(),
        ],
    });
    harness.app.current_error = Some(details.clone());
    harness.app.popup = Some(create_error_details_popup());
    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_error_details_with_splunk_messages() {
    use splunk_client::models::{MessageType, SplunkMessage};

    // Using very large terminal to avoid buffer overflow in popup rendering
    let mut harness = TuiHarness::new(120, 50);
    let mut details = ErrorDetails::from_error_string("Search failed");
    details.status_code = Some(400);
    details.messages = vec![
        SplunkMessage {
            message_type: MessageType::Error,
            text: "Invalid SPL syntax near '|'".to_string(),
        },
        SplunkMessage {
            message_type: MessageType::Warn,
            text: "Search may be slow due to missing index".to_string(),
        },
    ];
    harness.app.current_error = Some(details.clone());
    harness.app.popup = Some(create_error_details_popup());
    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_error_details_with_raw_json() {
    // Using very large terminal to avoid buffer overflow in popup rendering
    let mut harness = TuiHarness::new(120, 50);
    let mut details = ErrorDetails::from_error_string("API error");
    details.status_code = Some(500);
    details.raw_body =
        Some(r#"{"messages":[{"type":"ERROR","text":"Internal server error"}]}"#.to_string());
    harness.app.current_error = Some(details.clone());
    harness.app.popup = Some(create_error_details_popup());
    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_error_details_scrollable() {
    // Using very large terminal to avoid buffer overflow in popup rendering
    let mut harness = TuiHarness::new(120, 50);
    // Note: ErrorDetails uses HashMap for context, so we minimize context entries
    // to avoid non-deterministic ordering in snapshots
    let mut details = ErrorDetails::from_error_string(
        "This is a very long error message that should cause the error details popup to scroll. \
         It contains multiple lines of text to demonstrate the scroll functionality. \
         When the error message is long enough, the popup should display a scrollbar.",
    );
    details.status_code = Some(500);
    details.url = Some("https://localhost:8089/api/endpoint".to_string());
    details.request_id = Some("req-long".to_string());
    details.raw_body = Some(
        serde_json::to_string_pretty(
            &serde_json::json!({"error":"details","nested":{"deeply":{"nested":"value"}}}),
        )
        .unwrap(),
    );
    // Use timestamp override to ensure deterministic snapshot
    details.timestamp = "2024-01-15T10:30:00Z".to_string();
    harness.app.current_error = Some(details.clone());
    harness.app.popup = Some(create_error_details_popup());
    insta::assert_snapshot!(harness.render());
}
