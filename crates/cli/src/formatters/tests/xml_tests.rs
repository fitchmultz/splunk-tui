//! XML formatter tests.

use crate::formatters::{ClusterInfoOutput, ClusterPeerOutput, Formatter, XmlFormatter};
use serde_json::json;
use splunk_client::{Index, KvStoreMember, KvStoreReplicationStatus, KvStoreStatus, User};

#[test]
fn test_xml_formatter() {
    let formatter = XmlFormatter;
    let results = vec![json!({"name": "test", "value": "123"})];
    let output = formatter.format_search_results(&results).unwrap();
    assert!(output.contains("<?xml"));
    assert!(output.contains("<results>"));
    // New format uses nested elements instead of field attributes
    assert!(output.contains("<name>test</name>"));
    assert!(output.contains("<value>123</value>"));
    assert!(output.contains("</results>"));
}

#[test]
fn test_xml_formatter_with_non_string_values() {
    let formatter = XmlFormatter;
    let results =
        vec![json!({"name": "test", "count": 42, "active": true, "nested": {"key": "value"}})];
    let output = formatter.format_search_results(&results).unwrap();
    // Numbers should be rendered in nested elements
    assert!(output.contains("<count>42</count>"));
    // Booleans should be rendered
    assert!(output.contains("<active>true</active>"));
    // Nested objects should be properly nested, not JSON-escaped
    assert!(output.contains("<nested>"));
    assert!(output.contains("<key>value</key>"));
    assert!(output.contains("</nested>"));
    // Should NOT contain JSON serialization
    assert!(!output.contains("{&quot;"));
}

#[test]
fn test_xml_formatter_indexes_basic() {
    let formatter = XmlFormatter;
    let indexes = vec![Index {
        name: "main".to_string(),
        max_total_data_size_mb: Some(500),
        current_db_size_mb: 100,
        total_event_count: 1000,
        max_warm_db_count: Some(300),
        max_hot_buckets: Some("10".to_string()),
        frozen_time_period_in_secs: Some(2592000),
        cold_db_path: Some("/opt/splunk/var/lib/splunk/main/colddb".to_string()),
        home_path: Some("/opt/splunk/var/lib/splunk/main/db".to_string()),
        thawed_path: Some("/opt/splunk/var/lib/splunk/main/thaweddb".to_string()),
        cold_to_frozen_dir: None,
        primary_index: Some(true),
    }];
    let output = formatter.format_indexes(&indexes, false).unwrap();
    assert!(output.contains("<?xml"));
    assert!(output.contains("<indexes>"));
    assert!(output.contains("<name>main</name>"));
    assert!(output.contains("<sizeMB>100</sizeMB>"));
    assert!(output.contains("<maxSizeMB>500</maxSizeMB>"));
    // Detailed fields should NOT be present
    assert!(!output.contains("<homePath>"));
    assert!(!output.contains("<coldPath>"));
    assert!(!output.contains("<retentionSecs>"));
}

#[test]
fn test_xml_formatter_indexes_detailed() {
    let formatter = XmlFormatter;
    let indexes = vec![Index {
        name: "main".to_string(),
        max_total_data_size_mb: Some(500),
        current_db_size_mb: 100,
        total_event_count: 1000,
        max_warm_db_count: Some(300),
        max_hot_buckets: Some("10".to_string()),
        frozen_time_period_in_secs: Some(2592000),
        cold_db_path: Some("/opt/splunk/var/lib/splunk/main/colddb".to_string()),
        home_path: Some("/opt/splunk/var/lib/splunk/main/db".to_string()),
        thawed_path: Some("/opt/splunk/var/lib/splunk/main/thaweddb".to_string()),
        cold_to_frozen_dir: None,
        primary_index: Some(true),
    }];
    let output = formatter.format_indexes(&indexes, true).unwrap();
    assert!(output.contains("<?xml"));
    assert!(output.contains("<indexes>"));
    assert!(output.contains("<name>main</name>"));
    assert!(output.contains("<sizeMB>100</sizeMB>"));
    // Detailed fields SHOULD be present
    assert!(output.contains("<homePath>/opt/splunk/var/lib/splunk/main/db</homePath>"));
    assert!(output.contains("<coldPath>/opt/splunk/var/lib/splunk/main/colddb</coldPath>"));
    assert!(output.contains("<thawedPath>/opt/splunk/var/lib/splunk/main/thaweddb</thawedPath>"));
    assert!(output.contains("<retentionSecs>2592000</retentionSecs>"));
}

#[test]
fn test_cluster_peers_xml_formatting() {
    let formatter = XmlFormatter;
    let cluster_info = ClusterInfoOutput {
        id: "cluster-1".to_string(),
        label: Some("test-cluster".to_string()),
        mode: "master".to_string(),
        manager_uri: Some("https://master:8089".to_string()),
        replication_factor: Some(3),
        search_factor: Some(2),
        status: Some("Enabled".to_string()),
        peers: Some(vec![ClusterPeerOutput {
            host: "peer1".to_string(),
            port: 8089,
            id: "peer-1".to_string(),
            status: "Up".to_string(),
            peer_state: "Ready".to_string(),
            label: Some("Peer 1".to_string()),
            site: Some("site1".to_string()),
            is_captain: true,
        }]),
    };
    let output = formatter.format_cluster_info(&cluster_info, true).unwrap();
    // Verify XML structure
    assert!(output.contains("<cluster>"));
    assert!(output.contains("<id>cluster-1</id>"));
    assert!(output.contains("<peers>"));
    assert!(output.contains("<peer>"));
    assert!(output.contains("<host>peer1</host>"));
    assert!(output.contains("<port>8089</port>"));
    assert!(output.contains("<isCaptain>true</isCaptain>"));
    assert!(output.contains("</peers>"));
    assert!(output.contains("</cluster>"));
}

#[test]
fn test_kvstore_peers_xml_formatting() {
    let status = KvStoreStatus {
        current_member: KvStoreMember {
            guid: "guid".to_string(),
            host: "localhost".to_string(),
            port: 8089,
            replica_set: "rs0".to_string(),
            status: "Ready".to_string(),
        },
        replication_status: KvStoreReplicationStatus {
            oplog_size: 100,
            oplog_used: 1.5,
        },
    };
    let output = XmlFormatter.format_kvstore_status(&status).unwrap();
    assert!(output.contains("<kvstoreStatus>"));
    assert!(output.contains("<host>localhost</host>"));
    assert!(output.contains("<port>8089</port>"));
    assert!(output.contains("<oplogUsed>1.50</oplogUsed>"));
}

#[test]
fn test_users_xml_formatting() {
    let formatter = XmlFormatter;
    let users = vec![
        User {
            name: "admin".to_string(),
            realname: Some("Administrator".to_string()),
            email: Some("admin@example.com".to_string()),
            user_type: Some("Splunk".to_string()),
            default_app: Some("launcher".to_string()),
            roles: vec!["admin".to_string(), "power".to_string()],
            last_successful_login: Some(1704067200),
        },
        User {
            name: "user1".to_string(),
            realname: None,
            email: None,
            user_type: None,
            default_app: None,
            roles: vec![],
            last_successful_login: None,
        },
    ];
    let output = formatter.format_users(&users).unwrap();
    assert!(output.contains("<?xml"));
    assert!(output.contains("<users>"));
    assert!(output.contains("<user>"));
    assert!(output.contains("<name>admin</name>"));
    assert!(output.contains("<realname>Administrator</realname>"));
    assert!(output.contains("<type>Splunk</type>"));
    assert!(output.contains("<defaultApp>launcher</defaultApp>"));
    assert!(output.contains("<roles>"));
    assert!(output.contains("<role>admin</role>"));
    assert!(output.contains("<role>power</role>"));
    assert!(output.contains("<lastSuccessfulLogin>1704067200</lastSuccessfulLogin>"));
    assert!(output.contains("<name>user1</name>"));
    assert!(output.contains("</users>"));
}

// === RQ-0056: Tests for nested structure handling ===

#[test]
fn test_xml_formatter_nested_structure() {
    let formatter = XmlFormatter;
    let results = vec![json!({"user": {"name": "Alice", "age": 30}})];
    let output = formatter.format_search_results(&results).unwrap();

    // Should have proper nesting, not escaped JSON
    assert!(output.contains("<user>"));
    assert!(output.contains("<name>Alice</name>"));
    assert!(output.contains("<age>30</age>"));
    assert!(output.contains("</user>"));

    // Should NOT contain JSON serialization
    assert!(!output.contains("{&quot;"));
}

#[test]
fn test_xml_formatter_arrays() {
    let formatter = XmlFormatter;
    let results = vec![json!({"tags": ["foo", "bar"]})];
    let output = formatter.format_search_results(&results).unwrap();

    assert!(output.contains("<tags>"));
    assert!(output.contains("<item>foo</item>"));
    assert!(output.contains("<item>bar</item>"));
    assert!(output.contains("</tags>"));
}

#[test]
fn test_xml_formatter_complex_nesting() {
    let formatter = XmlFormatter;
    let results = vec![json!({
        "user": {
            "name": "Bob",
            "roles": ["admin", "user"]
        }
    })];
    let output = formatter.format_search_results(&results).unwrap();

    assert!(output.contains("<user>"));
    assert!(output.contains("<name>Bob</name>"));
    assert!(output.contains("<roles>"));
    assert!(output.contains("<item>admin</item>"));
    assert!(output.contains("<item>user</item>"));
    assert!(output.contains("</roles>"));
    assert!(output.contains("</user>"));
}

#[test]
fn test_xml_formatter_null_values() {
    let formatter = XmlFormatter;
    let results = vec![json!({"name": "test", "optional": null})];
    let output = formatter.format_search_results(&results).unwrap();

    // Null values should produce empty elements
    assert!(output.contains("<name>test</name>"));
    assert!(output.contains("<optional></optional>"));
}

#[test]
fn test_xml_formatter_deep_nesting() {
    let formatter = XmlFormatter;
    let results = vec![json!({
        "location": {
            "address": {
                "city": "NYC",
                "zip": "10001"
            }
        }
    })];
    let output = formatter.format_search_results(&results).unwrap();

    // Should have deeply nested structure
    assert!(output.contains("<location>"));
    assert!(output.contains("<address>"));
    assert!(output.contains("<city>NYC</city>"));
    assert!(output.contains("<zip>10001</zip>"));
    assert!(output.contains("</address>"));
    assert!(output.contains("</location>"));
}

// === RQ-0195: Null/missing fields tests ===

#[test]
fn test_xml_null_fields() {
    let formatter = XmlFormatter;
    let results = vec![json!({"name": "test", "optional": null})];
    let output = formatter.format_search_results(&results).unwrap();
    // Null fields should produce empty elements
    assert!(output.contains("<optional></optional>"));
    assert!(output.contains("<name>test</name>"));
}

// === RQ-0195: Unicode tests ===

#[test]
fn test_xml_unicode_escaping() {
    let formatter = XmlFormatter;
    let results = vec![json!({"name": "日本語", "emoji": "🎉"})];
    let output = formatter.format_search_results(&results).unwrap();
    // Unicode should be preserved in XML output
    assert!(output.contains("日本語"));
    assert!(output.contains("🎉"));
}

// === RQ-0195: Very wide data tests ===

#[test]
fn test_xml_very_long_strings() {
    let formatter = XmlFormatter;
    let long_string = "a".repeat(200);
    let results = vec![json!({"name": long_string, "value": "123"})];
    let output = formatter.format_search_results(&results).unwrap();
    // Long strings should be preserved
    assert!(output.contains(&"a".repeat(200)));
}

#[test]
fn test_xml_long_strings_with_special_chars() {
    let formatter = XmlFormatter;
    let long_string = "<tag>value</tag> & more ".repeat(20);
    let results = vec![json!({"name": long_string, "value": "123"})];
    let output = formatter.format_search_results(&results).unwrap();
    // Special XML characters should be escaped
    assert!(output.contains("&lt;tag&gt;"));
    assert!(output.contains("&amp;"));
}
