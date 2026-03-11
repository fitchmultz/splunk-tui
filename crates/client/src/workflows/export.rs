//! Shared file-export workflow for frontend-neutral serialized data.
//!
//! Responsibilities:
//! - Export serializable payloads to JSON, CSV, NDJSON, YAML, or Markdown.
//! - Keep tabular CSV behavior consistent across frontends.
//!
//! Does NOT handle:
//! - Frontend UX such as toasts, dialogs, or progress bars.
//! - Streaming arbitrarily large payloads incrementally.
//!
//! Invariants:
//! - File output is derived from fully materialized payloads.

use anyhow::Context;
use serde::Serialize;
use serde_json::Value;
use std::{collections::BTreeSet, path::Path};

/// Supported file export formats shared across frontends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Json,
    Csv,
    Ndjson,
    Yaml,
    Markdown,
}

pub async fn export_data<T: Serialize + ?Sized>(
    data: &T,
    path: &Path,
    format: ExportFormat,
) -> anyhow::Result<()> {
    match format {
        ExportFormat::Json => export_json_serialize(data, path).await,
        ExportFormat::Csv => {
            let value = serde_json::to_value(data)
                .context("Failed to serialize data to JSON for CSV export")?;
            export_value(&value, path, ExportFormat::Csv).await
        }
        ExportFormat::Ndjson => {
            let value = serde_json::to_value(data)
                .context("Failed to serialize data to JSON for NDJSON export")?;
            export_value(&value, path, ExportFormat::Ndjson).await
        }
        ExportFormat::Yaml => export_yaml_serialize(data, path).await,
        ExportFormat::Markdown => export_markdown_serialize(data, path).await,
    }
}

pub async fn export_value(value: &Value, path: &Path, format: ExportFormat) -> anyhow::Result<()> {
    match format {
        ExportFormat::Json => export_json_value(value, path).await,
        ExportFormat::Csv => match value {
            Value::Array(rows) => export_csv_values(rows, path).await,
            _ => export_csv_values(std::slice::from_ref(value), path).await,
        },
        ExportFormat::Ndjson => match value {
            Value::Array(rows) => export_ndjson_values(rows, path).await,
            _ => export_ndjson_values(std::slice::from_ref(value), path).await,
        },
        ExportFormat::Yaml => export_yaml_value(value, path).await,
        ExportFormat::Markdown => export_markdown_value(value, path).await,
    }
}

pub async fn export_results(
    results: &[Value],
    path: &Path,
    format: ExportFormat,
) -> anyhow::Result<()> {
    export_data(results, path, format).await
}

async fn export_json_value(value: &Value, path: &Path) -> anyhow::Result<()> {
    let json_bytes =
        serde_json::to_vec_pretty(value).context("Failed to serialize JSON for export")?;
    write_export_bytes(path, &json_bytes, "JSON").await
}

async fn export_json_serialize<T: Serialize + ?Sized>(data: &T, path: &Path) -> anyhow::Result<()> {
    let json_bytes =
        serde_json::to_vec_pretty(data).context("Failed to serialize data to JSON for export")?;
    write_export_bytes(path, &json_bytes, "JSON").await
}

async fn export_csv_values(rows: &[Value], path: &Path) -> anyhow::Result<()> {
    let mut buffer = Vec::new();
    {
        let mut writer = csv::Writer::from_writer(&mut buffer);
        let headers = collect_csv_headers(rows);

        if !headers.is_empty() {
            writer
                .write_record(&headers)
                .context("Failed to write CSV headers")?;
            for row in rows {
                if let Some(map) = row.as_object() {
                    let record: Vec<String> = headers
                        .iter()
                        .map(|key| map.get(key).map(value_to_csv_cell).unwrap_or_default())
                        .collect();
                    writer
                        .write_record(&record)
                        .context("Failed to write CSV record")?;
                }
            }
        } else {
            writer
                .write_record(["value"])
                .context("Failed to write CSV fallback header")?;
            for row in rows {
                writer
                    .write_record([row.to_string()])
                    .context("Failed to write CSV fallback record")?;
            }
        }

        writer.flush().context("Failed to flush CSV writer")?;
    }

    write_export_bytes(path, &buffer, "CSV").await
}

async fn export_ndjson_values(rows: &[Value], path: &Path) -> anyhow::Result<()> {
    let mut buffer = String::new();
    for (index, row) in rows.iter().enumerate() {
        let line =
            serde_json::to_string(row).context("Failed to serialize row for NDJSON export")?;
        buffer.push_str(&line);
        if index < rows.len() - 1 {
            buffer.push('\n');
        }
    }
    write_export_bytes(path, buffer.as_bytes(), "NDJSON").await
}

fn collect_csv_headers(rows: &[Value]) -> Vec<String> {
    let mut keys = BTreeSet::new();
    for row in rows {
        if let Some(object) = row.as_object() {
            for key in object.keys() {
                keys.insert(key.clone());
            }
        }
    }
    keys.into_iter().collect()
}

fn value_to_csv_cell(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        _ => value.to_string(),
    }
}

async fn export_yaml_value(value: &Value, path: &Path) -> anyhow::Result<()> {
    let yaml = serde_yaml::to_string(value).context("Failed to serialize YAML for export")?;
    write_export_bytes(path, yaml.as_bytes(), "YAML").await
}

async fn export_yaml_serialize<T: Serialize + ?Sized>(data: &T, path: &Path) -> anyhow::Result<()> {
    let yaml =
        serde_yaml::to_string(data).context("Failed to serialize data to YAML for export")?;
    write_export_bytes(path, yaml.as_bytes(), "YAML").await
}

async fn export_markdown_value(value: &Value, path: &Path) -> anyhow::Result<()> {
    let markdown = value_to_markdown(value);
    write_export_bytes(path, markdown.as_bytes(), "Markdown").await
}

async fn export_markdown_serialize<T: Serialize + ?Sized>(
    data: &T,
    path: &Path,
) -> anyhow::Result<()> {
    let value = serde_json::to_value(data)
        .context("Failed to serialize data to JSON for Markdown export")?;
    export_markdown_value(&value, path).await
}

fn value_to_markdown(value: &Value) -> String {
    match value {
        Value::Array(arr) => array_to_markdown_table(arr),
        Value::Object(obj) => object_to_markdown(obj),
        _ => format!("```\n{}\n```\n", value),
    }
}

fn array_to_markdown_table(arr: &[Value]) -> String {
    if arr.is_empty() {
        return "_No data available._\n".to_string();
    }

    let mut all_keys = BTreeSet::new();
    for item in arr {
        if let Some(obj) = item.as_object() {
            all_keys.extend(obj.keys().cloned());
        }
    }

    if all_keys.is_empty() {
        return "_No data available._\n".to_string();
    }

    let headers: Vec<&str> = all_keys.iter().map(|s| s.as_str()).collect();
    let mut output = String::new();

    output.push('|');
    for header in &headers {
        output.push_str(&format!(" {} |", escape_markdown(header)));
    }
    output.push('\n');

    output.push('|');
    for _ in &headers {
        output.push_str(" --- |");
    }
    output.push('\n');

    for item in arr {
        if let Some(obj) = item.as_object() {
            output.push('|');
            for header in &headers {
                let cell = obj
                    .get(*header)
                    .map(markdown_cell_value)
                    .unwrap_or_default();
                output.push_str(&format!(" {} |", escape_markdown(&cell)));
            }
            output.push('\n');
        }
    }

    output
}

fn object_to_markdown(obj: &serde_json::Map<String, Value>) -> String {
    let mut output = String::new();
    for (key, value) in obj {
        output.push_str(&format!(
            "- **{}**: {}\n",
            escape_markdown(key),
            markdown_cell_value(value)
        ));
    }
    output
}

fn markdown_cell_value(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        _ => value.to_string(),
    }
}

fn escape_markdown(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', "<br>")
}

async fn write_export_bytes(path: &Path, bytes: &[u8], format_name: &str) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await.with_context(|| {
            format!("Failed to create parent directory for {format_name} export")
        })?;
    }

    tokio::fs::write(path, bytes)
        .await
        .with_context(|| format!("Failed to write {format_name} export to {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    async fn read_back(path: &Path) -> String {
        tokio::fs::read_to_string(path).await.unwrap()
    }

    #[tokio::test]
    async fn exports_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.json");
        export_value(&json!({"name":"prod","count":2}), &path, ExportFormat::Json)
            .await
            .unwrap();
        let written = read_back(&path).await;
        assert!(written.contains("\"name\": \"prod\""));
    }

    #[tokio::test]
    async fn exports_csv_with_mixed_keys() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.csv");
        export_value(
            &json!([
                {"name":"prod","count":2},
                {"name":"dev","status":"ok"}
            ]),
            &path,
            ExportFormat::Csv,
        )
        .await
        .unwrap();
        let written = read_back(&path).await;
        assert!(written.lines().next().unwrap().contains("count"));
        assert!(written.contains("prod"));
        assert!(written.contains("dev"));
    }

    #[tokio::test]
    async fn exports_ndjson_for_array_and_single_object() {
        let dir = tempfile::tempdir().unwrap();
        let array_path = dir.path().join("array.ndjson");
        export_value(
            &json!([{"name":"prod"},{"name":"dev"}]),
            &array_path,
            ExportFormat::Ndjson,
        )
        .await
        .unwrap();
        let array_written = read_back(&array_path).await;
        assert_eq!(array_written.lines().count(), 2);

        let object_path = dir.path().join("object.ndjson");
        export_value(&json!({"name":"prod"}), &object_path, ExportFormat::Ndjson)
            .await
            .unwrap();
        let object_written = read_back(&object_path).await;
        assert_eq!(object_written.lines().count(), 1);
    }

    #[tokio::test]
    async fn exports_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.yaml");
        export_value(&json!({"name":"prod","count":2}), &path, ExportFormat::Yaml)
            .await
            .unwrap();
        let written = read_back(&path).await;
        assert!(written.contains("name: prod"));
    }

    #[tokio::test]
    async fn exports_markdown() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.md");
        export_value(
            &json!([
                {"name":"prod","status":"ok"},
                {"name":"dev","status":"warn"}
            ]),
            &path,
            ExportFormat::Markdown,
        )
        .await
        .unwrap();
        let written = read_back(&path).await;
        assert!(written.contains("| name |"));
        assert!(written.contains("prod"));
        assert!(written.contains("warn"));
    }
}
