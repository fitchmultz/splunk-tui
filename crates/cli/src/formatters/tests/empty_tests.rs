//! Empty result set tests for all formatters (RQ-0195, RQ-0359).
//!
//! ## Empty-State Behavior Standards
//!
//! Different formatters handle empty result sets differently based on their use case:
//!
//! | Format | Empty State Behavior | Rationale |
//! |--------|---------------------|-----------|
//! | JSON | Valid empty structure (`[]`) | Machine parseable |
//! | XML | Valid empty container (`<items></items>`) | Valid XML structure |
//! | CSV | Headers only, no data rows | Valid CSV - pipelines can parse headers |
//! | Table | Human message (`No items found.`) | Interactive format needs human feedback |

use crate::formatters::{CsvFormatter, Formatter, JsonFormatter, TableFormatter, XmlFormatter};
use splunk_client::models::{ConfigFile, ConfigStanza, FiredAlert, KvStoreCollection, LookupTable};
use splunk_client::{App, Index, LogEntry, SavedSearch, SearchJobStatus, User};
use splunk_config::types::ProfileConfig;

// === CSV Empty Tests ===
// CSV returns headers-only for empty results with static schemas,
// providing valid parseable structure for programmatic pipelines.

#[test]
fn test_csv_empty_search_results() {
    let formatter = CsvFormatter;
    let results: Vec<serde_json::Value> = vec![];
    let output = formatter.format_search_results(&results).unwrap();
    // Dynamic schema (search results) - headers cannot be determined without data
    // Empty string is acceptable for truly dynamic content
    assert_eq!(output, "");
}

#[test]
fn test_csv_empty_indexes() {
    let formatter = CsvFormatter;
    let indexes: Vec<Index> = vec![];
    let output = formatter.format_indexes(&indexes, false).unwrap();
    // Static schema - should return headers even for empty results
    assert_eq!(output, "Name,SizeMB,Events,MaxSizeMB\n");
}

#[test]
fn test_csv_empty_indexes_detailed() {
    let formatter = CsvFormatter;
    let indexes: Vec<Index> = vec![];
    let output = formatter.format_indexes(&indexes, true).unwrap();
    // Static schema (detailed) - should return headers even for empty results
    assert_eq!(
        output,
        "Name,SizeMB,Events,MaxSizeMB,RetentionSecs,HomePath,ColdPath,ThawedPath\n"
    );
}

#[test]
fn test_csv_empty_jobs() {
    let formatter = CsvFormatter;
    let jobs: Vec<SearchJobStatus> = vec![];
    let output = formatter.format_jobs(&jobs).unwrap();
    // Static schema - should return headers even for empty results
    assert_eq!(output, "SID,Done,Progress,Results,Duration\n");
}

#[test]
fn test_csv_empty_logs() {
    let formatter = CsvFormatter;
    let logs: Vec<LogEntry> = vec![];
    let output = formatter.format_logs(&logs).unwrap();
    // Static schema - should return headers even for empty results
    assert_eq!(output, "Time,Level,Component,Message\n");
}

#[test]
fn test_csv_empty_profiles() {
    let formatter = CsvFormatter;
    let profiles: std::collections::BTreeMap<String, ProfileConfig> =
        std::collections::BTreeMap::new();
    let output = formatter.format_profiles(&profiles).unwrap();
    // Static schema - should return headers even for empty results
    assert_eq!(
        output,
        "profile,base_url,username,skip_verify,timeout_seconds,max_retries\n"
    );
}

#[test]
fn test_csv_empty_config_files() {
    let formatter = CsvFormatter;
    let files: Vec<ConfigFile> = vec![];
    let output = formatter.format_config_files(&files).unwrap();
    // Static schema - should return headers even for empty results
    assert_eq!(output, "Name,Title,Description\n");
}

#[test]
fn test_csv_empty_config_stanzas() {
    let formatter = CsvFormatter;
    let stanzas: Vec<ConfigStanza> = vec![];
    let output = formatter.format_config_stanzas(&stanzas).unwrap();
    // Static schema - should return headers even for empty results
    assert_eq!(output, "Config File,Stanza Name\n");
}

#[test]
fn test_csv_empty_lookups() {
    let formatter = CsvFormatter;
    let lookups: Vec<LookupTable> = vec![];
    let output = formatter.format_lookups(&lookups).unwrap();
    // Static schema - should return headers even for empty results
    assert_eq!(output, "Name,Filename,Owner,App,Sharing,Size\n");
}

#[test]
fn test_csv_empty_kvstore_collections() {
    let formatter = CsvFormatter;
    let collections: Vec<KvStoreCollection> = vec![];
    let output = formatter.format_kvstore_collections(&collections).unwrap();
    // Static schema - should return headers even for empty results
    assert_eq!(output, "name,app,owner,sharing,disabled\n");
}

#[test]
fn test_csv_empty_fired_alerts() {
    let formatter = CsvFormatter;
    let alerts: Vec<FiredAlert> = vec![];
    let output = formatter.format_fired_alerts(&alerts).unwrap();
    // Static schema - should return headers even for empty results
    assert_eq!(
        output,
        "Name,SavedSearch,Severity,TriggerTime,SID,Actions\n"
    );
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
