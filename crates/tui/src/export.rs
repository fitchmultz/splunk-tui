//! Export functionality for search results.

use crate::action::ExportFormat;
use serde_json::Value;
use std::fs::File;
use std::path::Path;

/// Export search results to a file in the specified format.
pub fn export_results(results: &[Value], path: &Path, format: ExportFormat) -> Result<(), String> {
    match format {
        ExportFormat::Json => export_json(results, path),
        ExportFormat::Csv => export_csv(results, path),
    }
}

fn export_json(results: &[Value], path: &Path) -> Result<(), String> {
    let file = File::create(path).map_err(|e| e.to_string())?;
    serde_json::to_writer_pretty(file, results).map_err(|e| e.to_string())
}

fn export_csv(results: &[Value], path: &Path) -> Result<(), String> {
    let mut w = csv::Writer::from_path(path).map_err(|e| e.to_string())?;

    // Collect all unique keys from all results for CSV headers
    let mut all_keys: Vec<String> = Vec::new();
    for result in results {
        if let Some(obj) = result.as_object() {
            for key in obj.keys() {
                if !all_keys.contains(key) {
                    all_keys.push(key.clone());
                }
            }
        }
    }
    all_keys.sort();

    if !all_keys.is_empty() {
        // Write headers
        w.write_record(&all_keys).map_err(|e| e.to_string())?;

        for row in results {
            if let Some(map) = row.as_object() {
                let record: Vec<String> = all_keys
                    .iter()
                    .map(|k| {
                        map.get(k)
                            .map(|v| match v {
                                Value::String(s) => s.clone(),
                                Value::Null => String::new(),
                                _ => v.to_string(),
                            })
                            .unwrap_or_default()
                    })
                    .collect();
                w.write_record(&record).map_err(|e| e.to_string())?;
            }
        }
    } else {
        // Fallback for non-object results
        w.write_record(["value"]).map_err(|e| e.to_string())?;
        for row in results {
            w.write_record(&[row.to_string()])
                .map_err(|e| e.to_string())?;
        }
    }

    w.flush().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn test_export_json() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.json");
        let results = vec![json!({"a": 1})];

        export_results(&results, &path, ExportFormat::Json).unwrap();

        let content = std::fs::read_to_string(path).unwrap();
        assert!(content.contains("\"a\": 1"));
    }
}
