//! Streaming/tail mode formatter tests (RQ-0220).

use crate::formatters::{
    CsvFormatter, Formatter, JsonFormatter, TableFormatter, XmlFormatter,
    tests::make_test_log_entry,
};
use splunk_client::models::{LogEntry, LogLevel};

// === Table Streaming Tests ===

#[test]
fn test_table_streaming_first_batch() {
    let formatter = TableFormatter;
    let logs = vec![
        make_test_log_entry("2024-01-15T10:30:00Z", "INFO", "Search", "Query completed"),
        make_test_log_entry("2024-01-15T10:30:02Z", "WARN", "Index", "Disk space low"),
    ];

    // First batch should include header
    let output = formatter.format_logs_streaming(&logs, true).unwrap();
    assert!(output.contains("Time\tLevel\tComponent\tMessage"));
    assert!(output.contains("2024-01-15T10:30:00Z\tINFO\tSearch\tQuery completed"));
    assert!(output.contains("2024-01-15T10:30:02Z\tWARN\tIndex\tDisk space low"));
}

#[test]
fn test_table_streaming_subsequent_batch() {
    let formatter = TableFormatter;
    let logs = vec![make_test_log_entry(
        "2024-01-15T10:30:04Z",
        "ERROR",
        "Auth",
        "Login failed",
    )];

    // Subsequent batch should NOT include header
    let output = formatter.format_logs_streaming(&logs, false).unwrap();
    assert!(!output.contains("Time\tLevel\tComponent\tMessage"));
    assert!(output.contains("2024-01-15T10:30:04Z\tERROR\tAuth\tLogin failed"));
}

#[test]
fn test_table_streaming_empty() {
    let formatter = TableFormatter;
    let logs: Vec<LogEntry> = vec![];

    // Empty batch should return empty string (no header)
    let output = formatter.format_logs_streaming(&logs, true).unwrap();
    assert_eq!(output, "");
}

// === CSV Streaming Tests ===

#[test]
fn test_csv_streaming_first_batch() {
    let formatter = CsvFormatter;
    let logs = vec![
        make_test_log_entry("2024-01-15T10:30:00Z", "INFO", "Search", "Query completed"),
        make_test_log_entry("2024-01-15T10:30:02Z", "WARN", "Index", "Disk space low"),
    ];

    // First batch should include header
    let output = formatter.format_logs_streaming(&logs, true).unwrap();
    assert!(output.contains("Time,Level,Component,Message"));
    assert!(output.contains("2024-01-15T10:30:00Z,INFO,Search,Query completed"));
    assert!(output.contains("2024-01-15T10:30:02Z,WARN,Index,Disk space low"));
}

#[test]
fn test_csv_streaming_subsequent_batch() {
    let formatter = CsvFormatter;
    let logs = vec![make_test_log_entry(
        "2024-01-15T10:30:04Z",
        "ERROR",
        "Auth",
        "Login failed",
    )];

    // Subsequent batch should NOT include header
    let output = formatter.format_logs_streaming(&logs, false).unwrap();
    assert!(!output.contains("Time,Level,Component,Message"));
    assert!(output.contains("2024-01-15T10:30:04Z,ERROR,Auth,Login failed"));
}

#[test]
fn test_csv_streaming_empty() {
    let formatter = CsvFormatter;
    let logs: Vec<LogEntry> = vec![];

    // Empty batch should return empty string
    let output = formatter.format_logs_streaming(&logs, true).unwrap();
    assert_eq!(output, "");
}

// === JSON Streaming Tests ===

#[test]
fn test_json_streaming_ndjson_format() {
    let formatter = JsonFormatter;
    let logs = vec![
        make_test_log_entry("2024-01-15T10:30:00Z", "INFO", "Search", "Query completed"),
        make_test_log_entry("2024-01-15T10:30:02Z", "WARN", "Index", "Disk space low"),
    ];

    // JSON streaming should output NDJSON (one object per line)
    let output = formatter.format_logs_streaming(&logs, true).unwrap();

    // Each line should be a valid JSON object, not wrapped in an array
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 2);

    // Verify each line is valid JSON and contains expected fields
    // Note: LogEntry uses serde rename, so serialized fields are _time, log_level, etc.
    for line in &lines {
        let parsed: serde_json::Value =
            serde_json::from_str(line).expect("Each line should be valid JSON");
        assert!(parsed.get("_time").is_some(), "Missing _time field");
        assert!(parsed.get("log_level").is_some(), "Missing log_level field");
        assert!(parsed.get("component").is_some(), "Missing component field");
        assert!(parsed.get("_raw").is_some(), "Missing _raw field");
    }

    // Should NOT be a JSON array
    assert!(!output.starts_with('['));
    assert!(!output.ends_with(']'));
}

#[test]
fn test_json_streaming_is_first_ignored() {
    let formatter = JsonFormatter;
    let logs = vec![make_test_log_entry(
        "2024-01-15T10:30:00Z",
        "INFO",
        "Search",
        "Test",
    )];

    // JSON streaming ignores is_first - both should produce same output format
    let output_first = formatter.format_logs_streaming(&logs, true).unwrap();
    let output_subsequent = formatter.format_logs_streaming(&logs, false).unwrap();

    assert_eq!(output_first, output_subsequent);
}

#[test]
fn test_json_streaming_empty() {
    let formatter = JsonFormatter;
    let logs: Vec<LogEntry> = vec![];

    // Empty batch should return empty string
    let output = formatter.format_logs_streaming(&logs, true).unwrap();
    assert_eq!(output, "");
}

// === XML Streaming Tests ===

#[test]
fn test_xml_streaming_first_batch() {
    let formatter = XmlFormatter;
    let logs = vec![make_test_log_entry(
        "2024-01-15T10:30:00Z",
        "INFO",
        "Search",
        "Query completed",
    )];

    // First batch should include XML declaration and root element
    let output = formatter.format_logs_streaming(&logs, true).unwrap();
    assert!(output.contains(r#"<?xml version="1.0" encoding="UTF-8"?>"#));
    assert!(output.contains("<logs>"));
    assert!(output.contains("<log>"));
    assert!(output.contains("<time>2024-01-15T10:30:00Z</time>"));
    assert!(output.contains("<level>INFO</level>"));
    assert!(output.contains("<component>Search</component>"));
    assert!(output.contains("<message>Query completed</message>"));
    // Should NOT include closing </logs> in streaming mode
    assert!(!output.contains("</logs>"));
}

#[test]
fn test_xml_streaming_subsequent_batch() {
    let formatter = XmlFormatter;
    let logs = vec![make_test_log_entry(
        "2024-01-15T10:30:02Z",
        "WARN",
        "Index",
        "Disk space low",
    )];

    // Subsequent batch should NOT include XML declaration or root element
    let output = formatter.format_logs_streaming(&logs, false).unwrap();
    assert!(!output.contains(r#"<?xml version="1.0" encoding="UTF-8"?>"#));
    assert!(!output.contains("<logs>"));
    assert!(output.contains("<log>"));
    assert!(output.contains("<time>2024-01-15T10:30:02Z</time>"));
    assert!(output.contains("<level>WARN</level>"));
}

#[test]
fn test_xml_streaming_empty() {
    let formatter = XmlFormatter;
    let logs: Vec<LogEntry> = vec![];

    // Empty batch should return empty string (no declaration)
    let output = formatter.format_logs_streaming(&logs, true).unwrap();
    assert_eq!(output, "");
}

#[test]
fn test_xml_streaming_special_chars_escaped() {
    let formatter = XmlFormatter;
    let logs = vec![LogEntry {
        time: "2024-01-15T10:30:00Z".to_string(),
        index_time: "2025-01-24T12:00:01.000Z".to_string(),
        serial: Some(1),
        level: LogLevel::Error,
        component: "Test".to_string(),
        message: "Error: <script>alert('xss')</script>".to_string(),
    }];

    let output = formatter.format_logs_streaming(&logs, true).unwrap();
    // Special XML characters should be escaped
    assert!(output.contains("&lt;script&gt;"));
    assert!(output.contains("&apos;"));
    assert!(!output.contains("<script>")); // Should not contain unescaped script tag
}
