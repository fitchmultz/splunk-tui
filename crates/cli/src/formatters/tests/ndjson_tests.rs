//! NDJSON formatter tests.

use crate::formatters::{
    ClusterInfoOutput, ClusterManagementOutput, ClusterPeerOutput, Formatter, NdjsonFormatter,
    OutputFormat, Pagination,
};
use serde_json::json;
use splunk_client::Index;
use splunk_client::models::{PeerState, PeerStatus};

#[test]
fn test_ndjson_formatter_search_results() {
    let formatter = NdjsonFormatter;
    let results = vec![
        json!({"name": "test1", "value": "123"}),
        json!({"name": "test2", "value": "456"}),
    ];
    let output = formatter.format_search_results(&results).unwrap();

    // Should be two lines, each valid JSON
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 2);

    // Verify each line is valid JSON
    let parsed1: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(parsed1["name"], "test1");

    let parsed2: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(parsed2["name"], "test2");

    // Should NOT be wrapped in array
    assert!(!output.trim().starts_with('['));
    assert!(!output.trim().ends_with(']'));
}

#[test]
fn test_ndjson_formatter_empty_results() {
    let formatter = NdjsonFormatter;
    let results: Vec<serde_json::Value> = vec![];
    let output = formatter.format_search_results(&results).unwrap();
    assert_eq!(output, "");
}

#[test]
fn test_ndjson_formatter_single_item() {
    let formatter = NdjsonFormatter;
    let results = vec![json!({"name": "test", "value": "123"})];
    let output = formatter.format_search_results(&results).unwrap();

    // Single item - should have trailing newline for consistency
    assert!(output.ends_with('\n'));

    // Should be valid JSON (without the trailing newline)
    let trimmed = output.trim_end();
    let parsed: serde_json::Value = serde_json::from_str(trimmed).unwrap();
    assert_eq!(parsed["name"], "test");
}

#[test]
fn test_ndjson_formatter_indexes() {
    let formatter = NdjsonFormatter;
    let indexes = vec![Index {
        name: "main".to_string(),
        max_total_data_size_mb: Some(500),
        current_db_size_mb: 100,
        total_event_count: 1000,
        max_warm_db_count: Some(300),
        max_hot_buckets: Some("10".to_string()),
        frozen_time_period_in_secs: Some(2592000),
        cold_db_path: Some("/opt/splunk/cold".to_string()),
        home_path: Some("/opt/splunk/db".to_string()),
        thawed_path: Some("/opt/splunk/thawed".to_string()),
        cold_to_frozen_dir: None,
        primary_index: Some(true),
    }];
    let output = formatter.format_indexes(&indexes, false).unwrap();

    // One line per index
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 1);

    // Verify it's valid JSON with expected fields
    let parsed: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(parsed["name"], "main");
}

#[test]
fn test_ndjson_output_format_parsing() {
    // Test 'ndjson' parsing
    let format = OutputFormat::from_str("ndjson").unwrap();
    assert_eq!(format, OutputFormat::Ndjson);

    // Test 'jsonl' alias parsing
    let format = OutputFormat::from_str("jsonl").unwrap();
    assert_eq!(format, OutputFormat::Ndjson);

    // Test case insensitivity
    let format = OutputFormat::from_str("NDJSON").unwrap();
    assert_eq!(format, OutputFormat::Ndjson);

    let format = OutputFormat::from_str("JsonL").unwrap();
    assert_eq!(format, OutputFormat::Ndjson);
}

#[test]
fn test_ndjson_invalid_format_error_message() {
    let result = OutputFormat::from_str("yaml");
    assert!(result.is_err());
    let error = result.unwrap_err().to_string();
    // Error message should include ndjson as a valid option
    assert!(error.contains("ndjson"));
    assert!(error.contains("json"));
    assert!(error.contains("table"));
    assert!(error.contains("csv"));
    assert!(error.contains("xml"));
}

#[test]
fn test_ndjson_unicode_handling() {
    let formatter = NdjsonFormatter;
    let results = vec![
        json!({"name": "日本語", "emoji": "🎉"}),
        json!({"name": "中文", "value": 123}),
    ];
    let output = formatter.format_search_results(&results).unwrap();

    // Unicode should be preserved (not escaped) in NDJSON
    assert!(output.contains("日本語"));
    assert!(output.contains("🎉"));
    assert!(output.contains("中文"));

    // Each line should be valid JSON
    for line in output.lines() {
        let _: serde_json::Value =
            serde_json::from_str(line).expect("Each line should be valid JSON");
    }
}

#[test]
fn test_ndjson_special_characters() {
    let formatter = NdjsonFormatter;
    let results = vec![
        json!({"message": "Line 1\nLine 2", "tab": "col1\tcol2"}),
        json!({"quote": "He said \"hello\"", "backslash": "C:\\path\\to\\file"}),
    ];
    let output = formatter.format_search_results(&results).unwrap();

    // Each line should still be valid JSON
    for line in output.lines() {
        let _: serde_json::Value =
            serde_json::from_str(line).expect("Each line should be valid JSON");
    }
}

#[test]
fn test_ndjson_cluster_peers() {
    let formatter = NdjsonFormatter;
    let peers = vec![
        ClusterPeerOutput {
            host: "peer1".to_string(),
            port: 8089,
            id: "peer-1".to_string(),
            status: PeerStatus::Up.to_string(),
            peer_state: PeerState::Searchable.to_string(),
            label: Some("Peer 1".to_string()),
            site: Some("site1".to_string()),
            is_captain: true,
        },
        ClusterPeerOutput {
            host: "peer2".to_string(),
            port: 8089,
            id: "peer-2".to_string(),
            status: PeerStatus::Up.to_string(),
            peer_state: PeerState::Searchable.to_string(),
            label: None,
            site: None,
            is_captain: false,
        },
    ];

    let pagination = Pagination {
        total: Some(2),
        offset: 0,
        page_size: 2,
    };

    let output = formatter.format_cluster_peers(&peers, &pagination).unwrap();
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 2);

    // Verify each peer is a valid JSON object
    let parsed1: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(parsed1["host"], "peer1");

    let parsed2: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(parsed2["host"], "peer2");
}

#[test]
fn test_ndjson_null_fields() {
    let formatter = NdjsonFormatter;
    let results = vec![json!({"name": "test", "optional": null, "present": "value"})];
    let output = formatter.format_search_results(&results).unwrap();

    // Null should be serialized as null
    assert!(output.contains("\"optional\":null"));
    assert!(output.contains("\"present\":\"value\""));
}

#[test]
fn test_ndjson_cluster_management_output() {
    let formatter = NdjsonFormatter;
    let output = ClusterManagementOutput {
        operation: "remove".to_string(),
        target: "peer1".to_string(),
        success: true,
        message: "Peer removed successfully".to_string(),
    };

    let result = formatter.format_cluster_management(&output).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(result.trim()).unwrap();
    assert_eq!(parsed["operation"], "remove");
    assert_eq!(parsed["success"], true);
}

#[test]
fn test_ndjson_cluster_info_output() {
    let formatter = NdjsonFormatter;
    let info = ClusterInfoOutput {
        id: "cluster-1".to_string(),
        label: Some("My Cluster".to_string()),
        mode: "master".to_string(),
        manager_uri: Some("https://manager:8089".to_string()),
        replication_factor: Some(3),
        search_factor: Some(2),
        status: Some("Healthy".to_string()),
        maintenance_mode: Some(false),
        peers: None,
    };

    let result = formatter.format_cluster_info(&info, false).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(result.trim()).unwrap();
    assert_eq!(parsed["id"], "cluster-1");
    assert_eq!(parsed["label"], "My Cluster");
    assert_eq!(parsed["mode"], "master");
}
