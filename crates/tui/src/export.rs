//! Export functionality for TUI screens.
//!
//! Responsibilities:
//! - Export *any* serializable payload to JSON or CSV.
//! - Keep CSV behavior consistent with search-result exporting (object rows become columns).
//!
//! Non-responsibilities:
//! - Path validation / directory creation.
//! - Enforcing filename extensions.
//! - Streaming extremely large exports efficiently (payload is provided in-memory).

use crate::action::ExportFormat;
use serde::Serialize;
use serde_json::Value;
use std::{collections::BTreeSet, fs::File, path::Path};

/// Export any serializable payload to a file in the requested format.
///
/// For CSV, the payload is first serialized to `serde_json::Value` to allow
/// uniform "tabularization" (arrays of objects become rows/columns).
pub fn export_data<T: Serialize + ?Sized>(
    data: &T,
    path: &Path,
    format: ExportFormat,
) -> Result<(), String> {
    match format {
        ExportFormat::Json => export_json_serialize(data, path),
        ExportFormat::Csv => {
            let v = serde_json::to_value(data).map_err(|e| e.to_string())?;
            export_value(&v, path, ExportFormat::Csv)
        }
    }
}

/// Export a pre-serialized JSON value.
///
/// JSON exports write the value directly. CSV exports attempt to treat:
/// - `Value::Array` as rows
/// - all other values as a single row
pub fn export_value(value: &Value, path: &Path, format: ExportFormat) -> Result<(), String> {
    match format {
        ExportFormat::Json => export_json_value(value, path),
        ExportFormat::Csv => match value {
            Value::Array(rows) => export_csv_values(rows, path),
            _ => {
                let rows = vec![value.clone()];
                export_csv_values(&rows, path)
            }
        },
    }
}

/// Back-compat: export search results (slice of `Value`) to file.
pub fn export_results(results: &[Value], path: &Path, format: ExportFormat) -> Result<(), String> {
    export_data(results, path, format)
}

fn export_json_value(value: &Value, path: &Path) -> Result<(), String> {
    let file = File::create(path).map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(file, value).map_err(|e| e.to_string())
}

fn export_json_serialize<T: Serialize + ?Sized>(data: &T, path: &Path) -> Result<(), String> {
    let file = File::create(path).map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(file, data).map_err(|e| e.to_string())
}

fn export_csv_values(rows: &[Value], path: &Path) -> Result<(), String> {
    let mut w = csv::Writer::from_path(path).map_err(|e| e.to_string())?;

    let headers = collect_csv_headers(rows);

    if !headers.is_empty() {
        w.write_record(&headers).map_err(|e| e.to_string())?;

        for row in rows {
            if let Some(map) = row.as_object() {
                let record: Vec<String> = headers
                    .iter()
                    .map(|k| map.get(k).map(value_to_csv_cell).unwrap_or_default())
                    .collect();
                w.write_record(&record).map_err(|e| e.to_string())?;
            }
        }
    } else {
        // Fallback for non-object rows (or empty input)
        w.write_record(["value"]).map_err(|e| e.to_string())?;
        for row in rows {
            w.write_record([row.to_string()])
                .map_err(|e| e.to_string())?;
        }
    }

    w.flush().map_err(|e| e.to_string())
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

    #[test]
    fn test_export_csv_mixed_keys() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.csv");
        let results = vec![json!({"a": 1, "b": 2}), json!({"b": 3, "c": 4})];

        export_results(&results, &path, ExportFormat::Csv).unwrap();

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

    #[test]
    fn test_export_json_array() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let results = vec![json!({"a": 1})];

        export_results(&results, &path, ExportFormat::Json).unwrap();

        let content = std::fs::read_to_string(path).unwrap();
        assert!(content.contains("\"a\": 1"));
    }

    #[test]
    fn test_export_value_single_object_csv() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("single.csv");
        let value = json!({"x": "y", "n": 2});

        export_value(&value, &path, ExportFormat::Csv).unwrap();

        let mut rdr = csv::Reader::from_path(path).unwrap();
        let headers = rdr.headers().unwrap();
        assert_eq!(headers, vec!["n", "x"]);

        let rec = rdr.records().next().unwrap().unwrap();
        assert_eq!(rec.get(0).unwrap(), "2");
        assert_eq!(rec.get(1).unwrap(), "y");
    }

    #[test]
    fn test_export_data_struct_json() {
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

        export_data(&demo, &path, ExportFormat::Json).unwrap();
        let content = std::fs::read_to_string(path).unwrap();
        assert!(content.contains("\"a\": 1"));
        assert!(content.contains("\"b\": \"ok\""));
    }
}
