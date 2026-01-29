//! Tests for clipboard and export action redaction.

use std::path::PathBuf;

use crate::action::format::ExportFormat;
use crate::action::tests::redacted_debug;
use crate::action::variants::Action;

#[test]
fn test_redact_copy_to_clipboard() {
    let action = Action::CopyToClipboard("{\"user\":\"alice\",\"token\":\"abc123\"}".to_string());
    let output = redacted_debug(&action);

    assert!(!output.contains("alice"), "Should not contain user name");
    assert!(!output.contains("abc123"), "Should not contain token");
    assert!(
        output.contains("CopyToClipboard"),
        "Should contain action name"
    );
    assert!(output.contains("33 chars"), "Should show size indicator");
}

#[test]
fn test_redact_export_data() {
    let data = serde_json::json!({"results": [{"id": 1, "password": "secret123"}]});
    let path = PathBuf::from("/tmp/export.json");
    let action = Action::ExportData(data.clone(), path, ExportFormat::Json);
    let output = redacted_debug(&action);

    assert!(
        !output.contains("secret123"),
        "Should not contain sensitive data"
    );
    assert!(output.contains("ExportData"), "Should contain action name");
    assert!(output.contains("bytes"), "Should show bytes indicator");
}

#[test]
fn test_unicode_in_payload() {
    let action = Action::CopyToClipboard("日本語テスト 🇯🇵".to_string());
    let output = redacted_debug(&action);

    assert!(
        !output.contains("日本語"),
        "Should not contain Unicode content"
    );
    assert!(output.contains("chars"), "Should show character count");
}
