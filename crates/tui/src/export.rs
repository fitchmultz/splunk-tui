//! TUI wrapper around the shared export workflow.
//!
//! Responsibilities:
//! - Translate TUI export format selections into the shared workflow format.
//! - Keep TUI callers decoupled from the underlying shared module layout.
//!
//! Does NOT handle:
//! - Frontend popup UX or toast behavior.
//! - Serialization logic (delegated to `splunk-client::workflows::export`).
//!
//! Invariants:
//! - TUI export operations use the shared file-export workflow.

use crate::action::ExportFormat;
use serde::Serialize;
use serde_json::Value;
use std::path::Path;

fn shared_format(format: ExportFormat) -> splunk_client::workflows::export::ExportFormat {
    match format {
        ExportFormat::Json => splunk_client::workflows::export::ExportFormat::Json,
        ExportFormat::Csv => splunk_client::workflows::export::ExportFormat::Csv,
        ExportFormat::Ndjson => splunk_client::workflows::export::ExportFormat::Ndjson,
        ExportFormat::Yaml => splunk_client::workflows::export::ExportFormat::Yaml,
        ExportFormat::Markdown => splunk_client::workflows::export::ExportFormat::Markdown,
    }
}

pub async fn export_data<T: Serialize + ?Sized>(
    data: &T,
    path: &Path,
    format: ExportFormat,
) -> anyhow::Result<()> {
    splunk_client::workflows::export::export_data(data, path, shared_format(format)).await
}

pub async fn export_value(value: &Value, path: &Path, format: ExportFormat) -> anyhow::Result<()> {
    splunk_client::workflows::export::export_value(value, path, shared_format(format)).await
}

pub async fn export_results(
    results: &[Value],
    path: &Path,
    format: ExportFormat,
) -> anyhow::Result<()> {
    splunk_client::workflows::export::export_results(results, path, shared_format(format)).await
}
