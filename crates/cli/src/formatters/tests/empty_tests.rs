//! Empty result set tests for all formatters (RQ-0195).

use crate::formatters::{CsvFormatter, Formatter, JsonFormatter, TableFormatter, XmlFormatter};
use splunk_client::{App, Index, LogEntry, SavedSearch, SearchJobStatus, User};
use splunk_config::types::ProfileConfig;

// === CSV Empty Tests ===

#[test]
fn test_csv_empty_search_results() {
    let formatter = CsvFormatter;
    let results: Vec<serde_json::Value> = vec![];
    let output = formatter.format_search_results(&results).unwrap();
    assert_eq!(output, "");
}

#[test]
fn test_csv_empty_indexes() {
    let formatter = CsvFormatter;
    let indexes: Vec<Index> = vec![];
    let output = formatter.format_indexes(&indexes, false).unwrap();
    assert_eq!(output, "");
}

#[test]
fn test_csv_empty_indexes_detailed() {
    let formatter = CsvFormatter;
    let indexes: Vec<Index> = vec![];
    let output = formatter.format_indexes(&indexes, true).unwrap();
    assert_eq!(output, "");
}

#[test]
fn test_csv_empty_jobs() {
    let formatter = CsvFormatter;
    let jobs: Vec<SearchJobStatus> = vec![];
    let output = formatter.format_jobs(&jobs).unwrap();
    assert_eq!(output, "");
}

#[test]
fn test_csv_empty_logs() {
    let formatter = CsvFormatter;
    let logs: Vec<LogEntry> = vec![];
    let output = formatter.format_logs(&logs).unwrap();
    assert_eq!(output, "");
}

#[test]
fn test_csv_empty_profiles() {
    let formatter = CsvFormatter;
    let profiles: std::collections::BTreeMap<String, ProfileConfig> =
        std::collections::BTreeMap::new();
    let output = formatter.format_profiles(&profiles).unwrap();
    assert_eq!(output, "");
}

// === JSON Empty Tests ===

#[test]
fn test_json_empty_search_results() {
    let formatter = JsonFormatter;
    let results: Vec<serde_json::Value> = vec![];
    let output = formatter.format_search_results(&results).unwrap();
    assert_eq!(output, "[]");
}

#[test]
fn test_json_empty_indexes() {
    let formatter = JsonFormatter;
    let indexes: Vec<Index> = vec![];
    let output = formatter.format_indexes(&indexes, false).unwrap();
    assert_eq!(output, "[]");
}

#[test]
fn test_json_empty_jobs() {
    let formatter = JsonFormatter;
    let jobs: Vec<SearchJobStatus> = vec![];
    let output = formatter.format_jobs(&jobs).unwrap();
    assert_eq!(output, "[]");
}

#[test]
fn test_json_empty_users() {
    let formatter = JsonFormatter;
    let users: Vec<User> = vec![];
    let output = formatter.format_users(&users).unwrap();
    assert_eq!(output, "[]");
}

#[test]
fn test_json_empty_apps() {
    let formatter = JsonFormatter;
    let apps: Vec<App> = vec![];
    let output = formatter.format_apps(&apps).unwrap();
    assert_eq!(output, "[]");
}

// === XML Empty Tests ===

#[test]
fn test_xml_empty_search_results() {
    let formatter = XmlFormatter;
    let results: Vec<serde_json::Value> = vec![];
    let output = formatter.format_search_results(&results).unwrap();
    assert!(output.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    assert!(output.contains("<results>"));
    assert!(output.contains("</results>"));
    // Should not contain any result elements
    assert!(!output.contains("<result>"));
}

#[test]
fn test_xml_empty_indexes() {
    let formatter = XmlFormatter;
    let indexes: Vec<Index> = vec![];
    let output = formatter.format_indexes(&indexes, false).unwrap();
    assert!(output.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    assert!(output.contains("<indexes>"));
    assert!(output.contains("</indexes>"));
    // Should not contain any index elements
    assert!(!output.contains("<index>"));
}

#[test]
fn test_xml_empty_jobs() {
    let formatter = XmlFormatter;
    let jobs: Vec<SearchJobStatus> = vec![];
    let output = formatter.format_jobs(&jobs).unwrap();
    assert!(output.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    assert!(output.contains("<jobs>"));
    assert!(output.contains("</jobs>"));
    // Should not contain any job elements
    assert!(!output.contains("<job>"));
}

#[test]
fn test_xml_empty_users() {
    let formatter = XmlFormatter;
    let users: Vec<User> = vec![];
    let output = formatter.format_users(&users).unwrap();
    assert!(output.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    assert!(output.contains("<users>"));
    assert!(output.contains("</users>"));
    // Should not contain any user elements
    assert!(!output.contains("<user>"));
}

#[test]
fn test_xml_empty_apps() {
    let formatter = XmlFormatter;
    let apps: Vec<App> = vec![];
    let output = formatter.format_apps(&apps).unwrap();
    assert!(output.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    assert!(output.contains("<apps>"));
    assert!(output.contains("</apps>"));
    // Should not contain any app elements
    assert!(!output.contains("<app>"));
}

// === Table Empty Tests ===

#[test]
fn test_table_empty_search_results() {
    let formatter = TableFormatter;
    let results: Vec<serde_json::Value> = vec![];
    let output = formatter.format_search_results(&results).unwrap();
    assert_eq!(output, "No results found.");
}

#[test]
fn test_table_empty_indexes() {
    let formatter = TableFormatter;
    let indexes: Vec<Index> = vec![];
    let output = formatter.format_indexes(&indexes, false).unwrap();
    assert_eq!(output, "No indexes found.");
}

#[test]
fn test_table_empty_jobs() {
    let formatter = TableFormatter;
    let jobs: Vec<SearchJobStatus> = vec![];
    let output = formatter.format_jobs(&jobs).unwrap();
    assert_eq!(output, "No jobs found.");
}

#[test]
fn test_table_empty_logs() {
    let formatter = TableFormatter;
    let logs: Vec<LogEntry> = vec![];
    let output = formatter.format_logs(&logs).unwrap();
    assert_eq!(output, "No logs found.");
}

#[test]
fn test_table_empty_apps() {
    let formatter = TableFormatter;
    let apps: Vec<App> = vec![];
    let output = formatter.format_apps(&apps).unwrap();
    assert!(output.contains("No apps found"));
}

#[test]
fn test_table_empty_saved_searches() {
    let formatter = TableFormatter;
    let searches: Vec<SavedSearch> = vec![];
    let output = formatter.format_saved_searches(&searches).unwrap();
    assert!(output.contains("No saved searches found"));
}
