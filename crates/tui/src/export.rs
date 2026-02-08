//! Export functionality for TUI screens.
//!
//! Responsibilities:
//! - Export *any* serializable payload to JSON or CSV.
//! - Keep CSV behavior consistent with search-result exporting (object rows become columns).
//!
//! Does NOT handle:
//! - Path validation / directory creation.
//! - Enforcing filename extensions.
//! - Streaming extremely large exports efficiently (payload is provided in-memory).

use crate::action::ExportFormat;
use anyhow::Context;
use serde::Serialize;
use serde_json::Value;
use std::{collections::BTreeSet, path::Path};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

/// Export any serializable payload to a file in the requested format.
///
/// For CSV, the payload is first serialized to `serde_json::Value` to allow
/// uniform "tabularization" (arrays of objects become rows/columns).
pub async fn export_data<T: Serialize + ?Sized>(
    data: &T,
    path: &Path,
    format: ExportFormat,
) -> anyhow::Result<()> {
    match format {
        ExportFormat::Json => export_json_serialize(data, path).await,
        ExportFormat::Csv => {
            let v = serde_json::to_value(data)
                .context("Failed to serialize data to JSON for CSV export")?;
            export_value(&v, path, ExportFormat::Csv).await
        }
    }
}

/// Export a pre-serialized JSON value.
///
/// JSON exports write the value directly. CSV exports attempt to treat:
/// - `Value::Array` as rows
/// - all other values as a single row
pub async fn export_value(value: &Value, path: &Path, format: ExportFormat) -> anyhow::Result<()> {
    match format {
        ExportFormat::Json => export_json_value(value, path).await,
        ExportFormat::Csv => match value {
            Value::Array(rows) => export_csv_values(rows, path).await,
            _ => {
                let rows = vec![value.clone()];
                export_csv_values(&rows, path).await
            }
        },
    }
}

/// Back-compat: export search results (slice of `Value`) to file.
pub async fn export_results(
    results: &[Value],
    path: &Path,
    format: ExportFormat,
) -> anyhow::Result<()> {
    export_data(results, path, format).await
}

async fn export_json_value(value: &Value, path: &Path) -> anyhow::Result<()> {
    let mut file = File::create(path)
        .await
        .with_context(|| format!("Failed to create JSON export file: {}", path.display()))?;

    let json_bytes =
        serde_json::to_vec_pretty(value).context("Failed to serialize JSON for export")?;

    file.write_all(&json_bytes)
        .await
        .with_context(|| format!("Failed to write JSON export to: {}", path.display()))?;

    file.flush()
        .await
        .with_context(|| format!("Failed to flush JSON export to: {}", path.display()))?;

    Ok(())
}

async fn export_json_serialize<T: Serialize + ?Sized>(data: &T, path: &Path) -> anyhow::Result<()> {
    let mut file = File::create(path)
        .await
        .with_context(|| format!("Failed to create JSON export file: {}", path.display()))?;

    let json_bytes =
        serde_json::to_vec_pretty(data).context("Failed to serialize data to JSON for export")?;

    file.write_all(&json_bytes)
        .await
        .with_context(|| format!("Failed to write JSON export to: {}", path.display()))?;

    file.flush()
        .await
        .with_context(|| format!("Failed to flush JSON export to: {}", path.display()))?;

    Ok(())
}

async fn export_csv_values(rows: &[Value], path: &Path) -> anyhow::Result<()> {
    // The csv crate has no async API, so we buffer in memory first
    let mut buffer = Vec::new();
    {
        let mut w = csv::Writer::from_writer(&mut buffer);

        let headers = collect_csv_headers(rows);

        if !headers.is_empty() {
            w.write_record(&headers)
                .context("Failed to write CSV headers")?;

            for row in rows {
                if let Some(map) = row.as_object() {
                    let record: Vec<String> = headers
                        .iter()
                        .map(|k| map.get(k).map(value_to_csv_cell).unwrap_or_default())
                        .collect();
                    w.write_record(&record)
                        .context("Failed to write CSV record")?;
                }
            }
        } else {
            // Fallback for non-object rows (or empty input)
            w.write_record(["value"])
                .context("Failed to write CSV fallback header")?;
            for row in rows {
                w.write_record([row.to_string()])
                    .context("Failed to write CSV fallback record")?;
            }
        }

        w.flush().context("Failed to flush CSV writer")?;
    } // csv::Writer is dropped here, releasing the mutable borrow on buffer

    // Now write the buffer to file asynchronously
    let mut file = File::create(path)
        .await
        .with_context(|| format!("Failed to create CSV export file: {}", path.display()))?;

    file.write_all(&buffer)
        .await
        .with_context(|| format!("Failed to write CSV export to: {}", path.display()))?;

    file.flush()
        .await
        .with_context(|| format!("Failed to flush CSV export to: {}", path.display()))?;

    Ok(())
}

fn collect_csv_headers(rows: &[Value]) -> Vec<String> {
    let mut keys: BTreeSet<String> = BTreeSet::new();
    for row in rows {
        if let Some(obj) = row.as_object() {
            for k in obj.keys() {
                keys.insert(k.clone());
            }
        }
    }
    keys.into_iter().collect()
}

fn value_to_csv_cell(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        _ => v.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;
    use serde_json::json;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_export_csv_mixed_keys() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.csv");
        let results = vec![json!({"a": 1, "b": 2}), json!({"b": 3, "c": 4})];

        export_results(&results, &path, ExportFormat::Csv)
            .await
            .unwrap();

        let mut rdr = csv::Reader::from_path(path).unwrap();
        let headers = rdr.headers().unwrap();
        assert_eq!(headers, vec!["a", "b", "c"]);

        let mut records = rdr.records();

        let record1 = records.next().unwrap().unwrap();
        assert_eq!(record1.get(0).unwrap(), "1");
        assert_eq!(record1.get(1).unwrap(), "2");
        assert_eq!(record1.get(2).unwrap(), "");

        let record2 = records.next().unwrap().unwrap();
        assert_eq!(record2.get(0).unwrap(), "");
        assert_eq!(record2.get(1).unwrap(), "3");
        assert_eq!(record2.get(2).unwrap(), "4");
    }

    #[tokio::test]
    async fn test_export_json_array() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let results = vec![json!({"a": 1})];

        export_results(&results, &path, ExportFormat::Json)
            .await
            .unwrap();

        let content = std::fs::read_to_string(path).unwrap();
        assert!(content.contains("\"a\": 1"));
    }

    #[tokio::test]
    async fn test_export_value_single_object_csv() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("single.csv");
        let value = json!({"x": "y", "n": 2});

        export_value(&value, &path, ExportFormat::Csv)
            .await
            .unwrap();

        let mut rdr = csv::Reader::from_path(path).unwrap();
        let headers = rdr.headers().unwrap();
        assert_eq!(headers, vec!["n", "x"]);

        let rec = rdr.records().next().unwrap().unwrap();
        assert_eq!(rec.get(0).unwrap(), "2");
        assert_eq!(rec.get(1).unwrap(), "y");
    }

    #[tokio::test]
    async fn test_export_data_struct_json() {
        #[derive(Debug, Serialize)]
        struct Demo {
            a: i32,
            b: String,
        }

        let dir = tempdir().unwrap();
        let path = dir.path().join("demo.json");
        let demo = Demo {
            a: 1,
            b: "ok".to_string(),
        };

        export_data(&demo, &path, ExportFormat::Json).await.unwrap();
        let content = std::fs::read_to_string(path).unwrap();
        assert!(content.contains("\"a\": 1"));
        assert!(content.contains("\"b\": \"ok\""));
    }
}
