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
                    .map(|key| {
                        obj.get(key)
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string()
                    })
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

        // Print header
        output.push_str(&all_keys.join(","));
        output.push('\n');

        // Print rows
        for result in results {
            if let Some(obj) = result.as_object() {
                let row: Vec<String> = all_keys
                    .iter()
                    .map(|key| {
                        obj.get(key)
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string()
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

        // Header
        output.push_str("Name,SizeMB,Events,MaxSizeMB\n");

        for index in indexes {
            let max_size = index
                .max_total_data_size_mb
                .map(|v: u64| v.to_string())
                .unwrap_or_else(|| "N/A".to_string());
            output.push_str(&format!(
                "{},{},{},{}\n",
                index.name, index.current_db_size_mb, index.total_event_count, max_size
            ));
        }

        Ok(output)
    }

    fn format_jobs(&self, jobs: &[SearchJobStatus]) -> Result<String> {
        let mut output = String::new();

        if jobs.is_empty() {
            return Ok(String::new());
        }

        // Header
        output.push_str("SID,Done,Progress,Results,Duration\n");

        for job in jobs {
            output.push_str(&format!(
                "{},{},{:.1},{},{:.2}\n",
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
            "{},{},{},{},{},{},{}\n",
            "ClusterInfo",
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
                .unwrap_or_else(|| "N/A".to_string())
        ))
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
                    let value_str = value.as_str().unwrap_or("");
                    xml.push_str(&format!(
                        "    <field name=\"{}\">{}</field>\n",
                        escape_xml(key),
                        escape_xml(value_str)
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
    fn test_xml_formatter() {
        let formatter = XmlFormatter;
        let results = vec![json!({"name": "test", "value": "123"})];
        let output = formatter.format_search_results(&results).unwrap();
        assert!(output.contains("<?xml"));
        assert!(output.contains("<results>"));
        assert!(output.contains("<field name=\"name\">test</field>"));
        assert!(output.contains("</results>"));
    }
}
