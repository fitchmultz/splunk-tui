//! Export side effect handler tests.
//!
//! This module tests the ExportData side effect handler which exports
//! data to files in various formats.

mod common;

use common::*;

#[tokio::test]
async fn test_export_data_success() {
    let mut harness = SideEffectsTestHarness::new().await;

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let export_path = temp_dir.path().join("export.json");

    let data = serde_json::json!([{"name": "test", "value": 123}]);

    let actions = harness
        .handle_and_collect(
            Action::ExportData(
                data,
                export_path.clone(),
                splunk_tui::action::ExportFormat::Json,
            ),
            1,
        )
        .await;

    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::Notify(splunk_tui::ui::ToastLevel::Info, _))),
        "Should send info notification on success"
    );

    // Verify file was created
    assert!(export_path.exists(), "Export file should exist");
}

#[tokio::test]
async fn test_export_data_error() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Use an invalid path (directory that doesn't exist and can't be created)
    let export_path = std::path::PathBuf::from("/nonexistent/directory/export.json");

    let data = serde_json::json!([{"name": "test"}]);

    let actions = harness
        .handle_and_collect(
            Action::ExportData(
                data,
                export_path.clone(),
                splunk_tui::action::ExportFormat::Json,
            ),
            1,
        )
        .await;

    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::Notify(splunk_tui::ui::ToastLevel::Error, _))),
        "Should send error notification on failure"
    );
}
