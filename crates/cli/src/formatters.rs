//! Output formatters for CLI commands.
//!
//! Provides multiple output formats: JSON, Table, CSV, and XML.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use splunk_client::{Index, SearchJobStatus};

/// Supported output formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Table,
    Csv,
    Xml,
}

impl OutputFormat {
    /// Parse from string.
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "json" => Ok(OutputFormat::Json),
            "table" => Ok(OutputFormat::Table),
            "csv" => Ok(OutputFormat::Csv),
            "xml" => Ok(OutputFormat::Xml),
            _ => anyhow::bail!(
                "Invalid output format: {}. Valid options: json, table, csv, xml",
                s
            ),
        }
    }
}

/// Formatter trait for different output types.
pub trait Formatter {
    /// Format search results.
    fn format_search_results(&self, results: &[serde_json::Value]) -> Result<String>;

    /// Format indexes list.
    fn format_indexes(&self, indexes: &[Index]) -> Result<String>;

    /// Format jobs list.
    fn format_jobs(&self, jobs: &[SearchJobStatus]) -> Result<String>;

    /// Format cluster info.
    fn format_cluster_info(&self, cluster_info: &ClusterInfoOutput) -> Result<String>;
}

/// Cluster info output structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterInfoOutput {
    pub id: String,
    pub label: Option<String>,
    pub mode: String,
    pub manager_uri: Option<String>,
    pub replication_factor: Option<u32>,
    pub search_factor: Option<u32>,
    pub status: Option<String>,
}

/// JSON formatter.
pub struct JsonFormatter;

impl Formatter for JsonFormatter {
    fn format_search_results(&self, results: &[serde_json::Value]) -> Result<String> {
        Ok(serde_json::to_string_pretty(results)?)
    }

    fn format_indexes(&self, indexes: &[Index]) -> Result<String> {
        Ok(serde_json::to_string_pretty(indexes)?)
    }

    fn format_jobs(&self, jobs: &[SearchJobStatus]) -> Result<String> {
        Ok(serde_json::to_string_pretty(jobs)?)
    }

    fn format_cluster_info(&self, cluster_info: &ClusterInfoOutput) -> Result<String> {
        Ok(serde_json::to_string_pretty(cluster_info)?)
    }
}

/// Table formatter.
pub struct TableFormatter;

impl Formatter for TableFormatter {
    fn format_search_results(&self, results: &[serde_json::Value]) -> Result<String> {
        if results.is_empty() {
            return Ok("No results found.".to_string());
        }

        let mut output = String::new();

        // Get all unique keys from all results
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

        // Sort keys for consistent output
        all_keys.sort();

        // Print header
        output.push_str(&all_keys.join("\t"));
        output.push('\n');

        // Print rows
        for result in results {
            if let Some(obj) = result.as_object() {
                let row: Vec<String> = all_keys
                    .iter()
                    .map(|key| obj.get(key).map(format_json_value).unwrap_or_default())
                    .collect();
                output.push_str(&row.join("\t"));
                output.push('\n');
            }
        }

        Ok(output)
    }

    fn format_indexes(&self, indexes: &[Index]) -> Result<String> {
        let mut output = String::new();

        if indexes.is_empty() {
            return Ok("No indexes found.".to_string());
        }

        // Header
        output.push_str("Name\tSize (MB)\tEvents\tMax Size (MB)\n");

        for index in indexes {
            let max_size = index
                .max_total_data_size_mb
                .map(|v: u64| v.to_string())
                .unwrap_or_else(|| "N/A".to_string());
            output.push_str(&format!(
                "{}\t{}\t{}\t{}\n",
                index.name, index.current_db_size_mb, index.total_event_count, max_size
            ));
        }

        Ok(output)
    }

    fn format_jobs(&self, jobs: &[SearchJobStatus]) -> Result<String> {
        let mut output = String::new();

        if jobs.is_empty() {
            return Ok("No jobs found.".to_string());
        }

        // Header
        output.push_str("SID\tDone\tProgress\tResults\tDuration\n");

        for job in jobs {
            output.push_str(&format!(
                "{}\t{}\t{:.1}%\t{}\t{:.2}s\n",
                job.sid,
                if job.is_done { "Y" } else { "N" },
                job.done_progress * 100.0,
                job.result_count,
                job.run_duration
            ));
        }

        Ok(output)
    }

    fn format_cluster_info(&self, cluster_info: &ClusterInfoOutput) -> Result<String> {
        Ok(format!(
            "Cluster Information:\n\
             ID: {}\n\
             Label: {}\n\
             Mode: {}\n\
             Manager URI: {}\n\
             Replication Factor: {}\n\
             Search Factor: {}\n\
             Status: {}\n",
            cluster_info.id,
            cluster_info.label.as_deref().unwrap_or("N/A"),
            cluster_info.mode,
            cluster_info.manager_uri.as_deref().unwrap_or("N/A"),
            cluster_info
                .replication_factor
                .map(|v| v.to_string())
                .unwrap_or_else(|| "N/A".to_string()),
            cluster_info
                .search_factor
                .map(|v| v.to_string())
                .unwrap_or_else(|| "N/A".to_string()),
            cluster_info.status.as_deref().unwrap_or("N/A")
        ))
    }
}

/// CSV formatter.
pub struct CsvFormatter;

impl Formatter for CsvFormatter {
    fn format_search_results(&self, results: &[serde_json::Value]) -> Result<String> {
        if results.is_empty() {
            return Ok(String::new());
        }

        let mut output = String::new();

        // Get all unique keys from all results
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

        // Sort keys for consistent output
        all_keys.sort();

        // Print header (escaped)
        let header: Vec<String> = all_keys.iter().map(|k| escape_csv(k)).collect();
        output.push_str(&header.join(","));
        output.push('\n');

        // Print rows
        for result in results {
            if let Some(obj) = result.as_object() {
                let row: Vec<String> = all_keys
                    .iter()
                    .map(|key| {
                        let value = obj.get(key).map(format_json_value).unwrap_or_default();
                        escape_csv(&value)
                    })
                    .collect();
                output.push_str(&row.join(","));
                output.push('\n');
            }
        }

        Ok(output)
    }

    fn format_indexes(&self, indexes: &[Index]) -> Result<String> {
        let mut output = String::new();

        if indexes.is_empty() {
            return Ok(String::new());
        }

        // Header (escaped)
        output.push_str(&escape_csv("Name"));
        output.push(',');
        output.push_str(&escape_csv("SizeMB"));
        output.push(',');
        output.push_str(&escape_csv("Events"));
        output.push(',');
        output.push_str(&escape_csv("MaxSizeMB"));
        output.push('\n');

        for index in indexes {
            let max_size = index
                .max_total_data_size_mb
                .map(|v: u64| v.to_string())
                .unwrap_or_else(|| "N/A".to_string());
            output.push_str(&escape_csv(&index.name));
            output.push(',');
            output.push_str(&escape_csv(&index.current_db_size_mb.to_string()));
            output.push(',');
            output.push_str(&escape_csv(&index.total_event_count.to_string()));
            output.push(',');
            output.push_str(&escape_csv(&max_size));
            output.push('\n');
        }

        Ok(output)
    }

    fn format_jobs(&self, jobs: &[SearchJobStatus]) -> Result<String> {
        let mut output = String::new();

        if jobs.is_empty() {
            return Ok(String::new());
        }

        // Header (escaped)
        output.push_str(&escape_csv("SID"));
        output.push(',');
        output.push_str(&escape_csv("Done"));
        output.push(',');
        output.push_str(&escape_csv("Progress"));
        output.push(',');
        output.push_str(&escape_csv("Results"));
        output.push(',');
        output.push_str(&escape_csv("Duration"));
        output.push('\n');

        for job in jobs {
            output.push_str(&escape_csv(&job.sid));
            output.push(',');
            output.push_str(&escape_csv(if job.is_done { "Y" } else { "N" }));
            output.push(',');
            output.push_str(&escape_csv(&format!("{:.1}", job.done_progress * 100.0)));
            output.push(',');
            output.push_str(&escape_csv(&job.result_count.to_string()));
            output.push(',');
            output.push_str(&escape_csv(&format!("{:.2}", job.run_duration)));
            output.push('\n');
        }

        Ok(output)
    }

    fn format_cluster_info(&self, cluster_info: &ClusterInfoOutput) -> Result<String> {
        let fields = [
            escape_csv("ClusterInfo"),
            escape_csv(&cluster_info.id),
            escape_csv(cluster_info.label.as_deref().unwrap_or("N/A")),
            escape_csv(&cluster_info.mode),
            escape_csv(cluster_info.manager_uri.as_deref().unwrap_or("N/A")),
            escape_csv(
                &cluster_info
                    .replication_factor
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "N/A".to_string()),
            ),
            escape_csv(
                &cluster_info
                    .search_factor
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "N/A".to_string()),
            ),
        ];
        Ok(format!("{}\n", fields.join(",")))
    }
}

/// XML formatter.
pub struct XmlFormatter;

impl Formatter for XmlFormatter {
    fn format_search_results(&self, results: &[serde_json::Value]) -> Result<String> {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<results>\n");

        for (i, result) in results.iter().enumerate() {
            xml.push_str(&format!("  <result index=\"{}\">\n", i));
            if let Some(obj) = result.as_object() {
                for (key, value) in obj {
                    let value_str = format_json_value(value);
                    xml.push_str(&format!(
                        "    <field name=\"{}\">{}</field>\n",
                        escape_xml(key),
                        escape_xml(&value_str)
                    ));
                }
            }
            xml.push_str("  </result>\n");
        }

        xml.push_str("</results>");
        Ok(xml)
    }

    fn format_indexes(&self, indexes: &[Index]) -> Result<String> {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<indexes>\n");

        for index in indexes {
            xml.push_str("  <index>\n");
            xml.push_str(&format!("    <name>{}</name>\n", escape_xml(&index.name)));
            xml.push_str(&format!(
                "    <sizeMB>{}</sizeMB>\n",
                index.current_db_size_mb
            ));
            xml.push_str(&format!(
                "    <events>{}</events>\n",
                index.total_event_count
            ));
            if let Some(max_size) = index.max_total_data_size_mb {
                xml.push_str(&format!("    <maxSizeMB>{}</maxSizeMB>\n", max_size));
            }
            if let Some(frozen_time) = index.frozen_time_period_in_secs {
                let days = frozen_time / 86400;
                xml.push_str(&format!("    <retentionDays>{}</retentionDays>\n", days));
            }
            if let Some(home_path) = &index.home_path {
                xml.push_str(&format!("    <path>{}</path>\n", escape_xml(home_path)));
            }
            xml.push_str("  </index>\n");
        }

        xml.push_str("</indexes>");
        Ok(xml)
    }

    fn format_jobs(&self, jobs: &[SearchJobStatus]) -> Result<String> {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<jobs>\n");

        for job in jobs {
            xml.push_str("  <job>\n");
            xml.push_str(&format!("    <sid>{}</sid>\n", escape_xml(&job.sid)));
            xml.push_str(&format!("    <done>{}</done>\n", job.is_done));
            xml.push_str(&format!(
                "    <progress>{:.1}</progress>\n",
                job.done_progress * 100.0
            ));
            xml.push_str(&format!("    <results>{}</results>\n", job.result_count));
            xml.push_str(&format!(
                "    <duration>{:.2}</duration>\n",
                job.run_duration
            ));
            xml.push_str("  </job>\n");
        }

        xml.push_str("</jobs>");
        Ok(xml)
    }

    fn format_cluster_info(&self, cluster_info: &ClusterInfoOutput) -> Result<String> {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<cluster>\n");
        xml.push_str(&format!("  <id>{}</id>\n", escape_xml(&cluster_info.id)));
        if let Some(label) = &cluster_info.label {
            xml.push_str(&format!("  <label>{}</label>\n", escape_xml(label)));
        }
        xml.push_str(&format!(
            "  <mode>{}</mode>\n",
            escape_xml(&cluster_info.mode)
        ));
        if let Some(manager_uri) = &cluster_info.manager_uri {
            xml.push_str(&format!(
                "  <managerUri>{}</managerUri>\n",
                escape_xml(manager_uri)
            ));
        }
        if let Some(replication_factor) = cluster_info.replication_factor {
            xml.push_str(&format!(
                "  <replicationFactor>{}</replicationFactor>\n",
                replication_factor
            ));
        }
        if let Some(search_factor) = cluster_info.search_factor {
            xml.push_str(&format!(
                "  <searchFactor>{}</searchFactor>\n",
                search_factor
            ));
        }
        if let Some(status) = &cluster_info.status {
            xml.push_str(&format!("  <status>{}</status>\n", escape_xml(status)));
        }
        xml.push_str("</cluster>");
        Ok(xml)
    }
}

/// Format a JSON value as a string for display.
///
/// Converts any JSON value to its string representation:
/// - Strings are returned as-is
/// - Numbers and booleans are converted to their string representation
/// - Null values become empty strings
/// - Arrays and objects are serialized as compact JSON
fn format_json_value(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => String::new(),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            // Serialize arrays/objects as compact JSON
            serde_json::to_string(v).unwrap_or_default()
        }
    }
}

/// Escape a string value for CSV output according to RFC 4180.
///
/// Rules:
/// - Wrap in double quotes if the field contains comma, double quote, or newline
/// - Double any internal double quotes (e.g., `"hello"` -> `""hello""`)
fn escape_csv(s: &str) -> String {
    let needs_quoting = s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r');
    if !needs_quoting {
        return s.to_string();
    }
    // Double all quotes and wrap in quotes
    format!("\"{}\"", s.replace('"', "\"\""))
}

/// Escape special XML characters.
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Get a formatter for the specified output format.
pub fn get_formatter(format: OutputFormat) -> Box<dyn Formatter> {
    match format {
        OutputFormat::Json => Box::new(JsonFormatter),
        OutputFormat::Table => Box::new(TableFormatter),
        OutputFormat::Csv => Box::new(CsvFormatter),
        OutputFormat::Xml => Box::new(XmlFormatter),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_output_format_from_str() {
        assert_eq!(OutputFormat::from_str("json").unwrap(), OutputFormat::Json);
        assert_eq!(OutputFormat::from_str("JSON").unwrap(), OutputFormat::Json);
        assert_eq!(OutputFormat::from_str("csv").unwrap(), OutputFormat::Csv);
        assert_eq!(OutputFormat::from_str("xml").unwrap(), OutputFormat::Xml);
        assert_eq!(
            OutputFormat::from_str("table").unwrap(),
            OutputFormat::Table
        );
        assert!(OutputFormat::from_str("invalid").is_err());
    }

    #[test]
    fn test_xml_escaping() {
        assert_eq!(escape_xml("test&<>'\""), "test&amp;&lt;&gt;&apos;&quot;");
    }

    #[test]
    fn test_csv_escaping() {
        // No escaping needed for simple strings
        assert_eq!(escape_csv("simple"), "simple");
        // Comma requires quoting
        assert_eq!(escape_csv("hello,world"), "\"hello,world\"");
        // Quote requires doubling and wrapping
        assert_eq!(escape_csv("say \"hi\""), "\"say \"\"hi\"\"\"");
        // Newline requires quoting
        assert_eq!(escape_csv("line1\nline2"), "\"line1\nline2\"");
        // Mixed special chars
        assert_eq!(
            escape_csv("value, with \"quotes\"\nand newline"),
            "\"value, with \"\"quotes\"\"\nand newline\""
        );
    }

    #[test]
    fn test_format_json_value() {
        // String values
        assert_eq!(format_json_value(&json!("hello")), "hello");
        // Number values
        assert_eq!(format_json_value(&json!(42)), "42");
        assert_eq!(
            format_json_value(&json!(std::f64::consts::PI)),
            format!("{}", std::f64::consts::PI)
        );
        // Boolean values
        assert_eq!(format_json_value(&json!(true)), "true");
        assert_eq!(format_json_value(&json!(false)), "false");
        // Null values
        assert_eq!(format_json_value(&json!(null)), "");
        // Array values (compact JSON)
        assert_eq!(format_json_value(&json!([1, 2, 3])), "[1,2,3]");
        // Object values (compact JSON)
        assert_eq!(
            format_json_value(&json!({"key": "value"})),
            "{\"key\":\"value\"}"
        );
    }

    #[test]
    fn test_json_formatter() {
        let formatter = JsonFormatter;
        let results = vec![json!({"name": "test", "value": "123"})];
        let output = formatter.format_search_results(&results).unwrap();
        assert!(output.contains("test"));
        assert!(output.contains("123"));
    }

    #[test]
    fn test_csv_formatter() {
        let formatter = CsvFormatter;
        let results = vec![json!({"name": "test", "value": "123"})];
        let output = formatter.format_search_results(&results).unwrap();
        assert!(output.contains("name,value"));
        assert!(output.contains("test,123"));
    }

    #[test]
    fn test_csv_formatter_with_special_chars() {
        let formatter = CsvFormatter;
        let results = vec![json!({"name": "test,with,commas", "value": "say \"hello\""})];
        let output = formatter.format_search_results(&results).unwrap();
        // Headers should be properly escaped
        assert!(output.contains("name,value"));
        // Values with commas should be quoted
        assert!(output.contains("\"test,with,commas\""));
        // Values with quotes should have doubled quotes
        assert!(output.contains("\"say \"\"hello\"\"\""));
    }

    #[test]
    fn test_xml_formatter() {
        let formatter = XmlFormatter;
        let results = vec![json!({"name": "test", "value": "123"})];
        let output = formatter.format_search_results(&results).unwrap();
        assert!(output.contains("<?xml"));
        assert!(output.contains("<results>"));
        assert!(output.contains("<field name=\"name\">test</field>"));
        assert!(output.contains("</results>"));
    }

    #[test]
    fn test_table_formatter_with_non_string_values() {
        let formatter = TableFormatter;
        let results = vec![json!({"name": "test", "count": 42, "active": true, "data": null})];
        let output = formatter.format_search_results(&results).unwrap();
        // Numbers should be rendered
        assert!(output.contains("42"));
        // Booleans should be rendered
        assert!(output.contains("true"));
        // Null should be empty string (not "null")
        assert!(!output.contains("null"));
    }

    #[test]
    fn test_csv_formatter_with_non_string_values() {
        let formatter = CsvFormatter;
        let results = vec![json!({"name": "test", "count": 42, "active": true})];
        let output = formatter.format_search_results(&results).unwrap();
        // Numbers should be rendered
        assert!(output.contains("42"));
        // Booleans should be rendered
        assert!(output.contains("true"));
    }

    #[test]
    fn test_xml_formatter_with_non_string_values() {
        let formatter = XmlFormatter;
        let results =
            vec![json!({"name": "test", "count": 42, "active": true, "nested": {"key": "value"}})];
        let output = formatter.format_search_results(&results).unwrap();
        // Numbers should be rendered
        assert!(output.contains("<field name=\"count\">42</field>"));
        // Booleans should be rendered
        assert!(output.contains("<field name=\"active\">true</field>"));
        // Objects should be rendered as compact JSON (with XML-escaped quotes)
        assert!(
            output.contains("<field name=\"nested\">{&quot;key&quot;:&quot;value&quot;}</field>")
        );
    }

    #[test]
    fn test_value_rendering() {
        // Test that numeric and boolean values appear in all formatters
        let results = vec![json!({"name": "test", "count": 123, "enabled": false})];

        // Table formatter
        let table_output = TableFormatter.format_search_results(&results).unwrap();
        assert!(table_output.contains("123"));
        assert!(table_output.contains("false"));

        // CSV formatter
        let csv_output = CsvFormatter.format_search_results(&results).unwrap();
        assert!(csv_output.contains("123"));
        assert!(csv_output.contains("false"));

        // XML formatter
        let xml_output = XmlFormatter.format_search_results(&results).unwrap();
        assert!(xml_output.contains("123"));
        assert!(xml_output.contains("false"));
    }
}
