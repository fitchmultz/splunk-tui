//! CSV formatter tests and JSON flattening tests.

use crate::formatters::{
    ClusterInfoOutput, ClusterPeerOutput, CsvFormatter, Formatter, LicenseInfoOutput,
    common::{flatten_json_object, get_all_flattened_keys},
};
use serde_json::json;
use splunk_client::{
    Index, KvStoreMember, KvStoreReplicationStatus, KvStoreStatus, LicenseUsage, User,
};

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
fn test_csv_formatter_indexes_basic() {
    let formatter = CsvFormatter;
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
    assert!(output.contains("Name,SizeMB,Events,MaxSizeMB"));
    assert!(!output.contains("HomePath"));
    assert!(!output.contains("ColdPath"));
}

#[test]
fn test_csv_formatter_indexes_detailed() {
    let formatter = CsvFormatter;
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
    assert!(
        output.contains("Name,SizeMB,Events,MaxSizeMB,RetentionSecs,HomePath,ColdPath,ThawedPath")
    );
    assert!(output.contains("2592000"));
    assert!(output.contains("/opt/splunk/var/lib/splunk/main/db"));
}

#[test]
fn test_cluster_peers_csv_formatting() {
    let formatter = CsvFormatter;
    let cluster_info = ClusterInfoOutput {
        id: "cluster-1".to_string(),
        label: Some("test-cluster".to_string()),
        mode: "master".to_string(),
        manager_uri: Some("https://master:8089".to_string()),
        replication_factor: Some(3),
        search_factor: Some(2),
        status: Some("Enabled".to_string()),
        maintenance_mode: None,
        peers: Some(vec![ClusterPeerOutput {
            host: "peer1".to_string(),
            port: 8089,
            id: "peer-1".to_string(),
            status: "Up".to_string(),
            peer_state: "Ready".to_string(),
            label: Some("Peer,1".to_string()),
            site: Some("site1".to_string()),
            is_captain: true,
        }]),
    };
    let output = formatter.format_cluster_info(&cluster_info, true).unwrap();
    // Verify CSV has cluster info row and peer row
    assert!(output.contains("ClusterInfo"));
    assert!(output.contains("cluster-1"));
    assert!(output.contains("Peer"));
    assert!(output.contains("peer1:8089"));
    // Verify CSV escaping for label with comma
    assert!(output.contains("\"Peer,1\""));
    assert!(output.contains("Yes"));
}

#[test]
fn test_kvstore_peers_csv_formatting() {
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
    let output = CsvFormatter.format_kvstore_status(&status).unwrap();
    assert!(output.contains("host,port,status,replica_set,oplog_size_mb,oplog_used_percent"));
    assert!(output.contains("localhost,8089,Ready,rs0,100,1.5"));
}

#[test]
fn test_format_license_csv() {
    let formatter = CsvFormatter;
    let license = LicenseInfoOutput {
        usage: vec![LicenseUsage {
            name: "daily_usage".to_string(),
            quota: 100 * 1024 * 1024,
            used_bytes: Some(50 * 1024 * 1024),
            slaves_usage_bytes: None,
            stack_id: Some("enterprise".to_string()),
        }],
        pools: vec![],
        stacks: vec![],
    };

    let output = formatter.format_license(&license).unwrap();
    assert!(output.contains("Type,Name,StackID,UsedMB,QuotaMB,PctUsed"));
    assert!(output.contains("Usage,daily_usage,enterprise,50,100,50.00"));
}

#[test]
fn test_users_csv_formatting() {
    let formatter = CsvFormatter;
    let users = vec![User {
        name: "admin".to_string(),
        realname: Some("Administrator".to_string()),
        email: Some("admin@example.com".to_string()),
        user_type: Some("Splunk".to_string()),
        default_app: Some("launcher".to_string()),
        roles: vec!["admin".to_string(), "power".to_string()],
        last_successful_login: Some(1704067200),
    }];
    let output = formatter.format_users(&users).unwrap();
    assert!(output.contains("name,realname,user_type,default_app,roles,last_successful_login"));
    assert!(output.contains("admin,Administrator,Splunk,launcher,admin;power,1704067200"));
}

#[test]
fn test_users_csv_special_characters() {
    let formatter = CsvFormatter;
    let users = vec![User {
        name: "user,name".to_string(),
        realname: Some("User, Name".to_string()),
        email: None,
        user_type: None,
        default_app: None,
        roles: vec![],
        last_successful_login: None,
    }];
    let output = formatter.format_users(&users).unwrap();
    assert!(output.contains("\"user,name\""));
    assert!(output.contains("\"User, Name\""));
}

// === RQ-0056: Tests for flattening nested JSON structures ===

#[test]
fn test_flatten_simple_object() {
    let value = json!({"name": "Alice", "age": 30});
    let mut flat = std::collections::BTreeMap::new();
    flatten_json_object(&value, "", &mut flat);
    assert_eq!(flat.get("name"), Some(&"Alice".to_string()));
    assert_eq!(flat.get("age"), Some(&"30".to_string()));
}

#[test]
fn test_flatten_nested_object() {
    let value = json!({"user": {"name": "Bob", "address": {"city": "NYC"}}});
    let mut flat = std::collections::BTreeMap::new();
    flatten_json_object(&value, "", &mut flat);
    assert_eq!(flat.get("user.name"), Some(&"Bob".to_string()));
    assert_eq!(flat.get("user.address.city"), Some(&"NYC".to_string()));
}

#[test]
fn test_flatten_array() {
    let value = json!({"tags": ["foo", "bar", "baz"]});
    let mut flat = std::collections::BTreeMap::new();
    flatten_json_object(&value, "", &mut flat);
    assert_eq!(flat.get("tags.0"), Some(&"foo".to_string()));
    assert_eq!(flat.get("tags.1"), Some(&"bar".to_string()));
    assert_eq!(flat.get("tags.2"), Some(&"baz".to_string()));
}

#[test]
fn test_flatten_array_of_objects() {
    let value = json!({"users": [{"name": "Alice"}, {"name": "Bob"}]});
    let mut flat = std::collections::BTreeMap::new();
    flatten_json_object(&value, "", &mut flat);
    assert_eq!(flat.get("users.0.name"), Some(&"Alice".to_string()));
    assert_eq!(flat.get("users.1.name"), Some(&"Bob".to_string()));
}

#[test]
fn test_flatten_null_values() {
    let value = json!({"name": "Test", "optional": null});
    let mut flat = std::collections::BTreeMap::new();
    flatten_json_object(&value, "", &mut flat);
    assert_eq!(flat.get("name"), Some(&"Test".to_string()));
    assert_eq!(flat.get("optional"), Some(&"".to_string())); // null becomes empty string
}

#[test]
fn test_get_all_flattened_keys() {
    let results = vec![
        json!({"user": {"name": "Alice"}}),
        json!({"user": {"age": 30}, "status": "active"}),
    ];
    let keys = get_all_flattened_keys(&results);
    // Should include all unique keys in sorted order
    assert!(keys.contains(&"status".to_string()));
    assert!(keys.contains(&"user.age".to_string()));
    assert!(keys.contains(&"user.name".to_string()));
}

#[test]
fn test_csv_formatter_nested_objects() {
    let formatter = CsvFormatter;
    let results = vec![
        json!({"user": {"name": "Alice", "age": 30}, "status": "active"}),
        json!({"user": {"name": "Bob"}, "status": "inactive"}),
    ];
    let output = formatter.format_search_results(&results).unwrap();

    // Headers should include dot-notation keys
    assert!(output.contains("status"));
    assert!(output.contains("user.age"));
    assert!(output.contains("user.name"));

    // First row - Alice has all fields
    assert!(output.contains("active"));
    assert!(output.contains("30"));
    assert!(output.contains("Alice"));

    // Second row - Bob is missing age field - should be empty
    assert!(output.contains("inactive"));
    assert!(output.contains("Bob"));
}

#[test]
fn test_csv_formatter_deeply_nested() {
    let formatter = CsvFormatter;
    let results = vec![json!({
        "location": {
            "address": {
                "city": "NYC",
                "zip": "10001"
            }
        }
    })];
    let output = formatter.format_search_results(&results).unwrap();
    assert!(output.contains("location.address.city"));
    assert!(output.contains("location.address.zip"));
    assert!(output.contains("NYC"));
    assert!(output.contains("10001"));
}

#[test]
fn test_csv_formatter_arrays() {
    let formatter = CsvFormatter;
    let results = vec![json!({"tags": ["foo", "bar"], "count": 2})];
    let output = formatter.format_search_results(&results).unwrap();
    assert!(output.contains("count"));
    assert!(output.contains("tags.0"));
    assert!(output.contains("tags.1"));
    assert!(output.contains("foo"));
    assert!(output.contains("bar"));
}

// === RQ-0195: Null/missing fields tests ===

#[test]
fn test_csv_null_fields_in_users() {
    let formatter = CsvFormatter;
    let users = vec![User {
        name: "user1".to_string(),
        realname: None,
        email: None,
        user_type: None,
        default_app: None,
        roles: vec![],
        last_successful_login: None,
    }];
    let output = formatter.format_users(&users).unwrap();
    // Null fields should be empty in CSV (consecutive commas)
    assert!(output.contains("user1,,,,,0"));
}

// === RQ-0195: Unicode tests ===

#[test]
fn test_csv_unicode_in_users() {
    let formatter = CsvFormatter;
    let users = vec![User {
        name: "user_日本語".to_string(),
        realname: Some("Japanese Name 日本語".to_string()),
        email: None,
        user_type: None,
        default_app: None,
        roles: vec![],
        last_successful_login: None,
    }];
    let output = formatter.format_users(&users).unwrap();
    // Unicode should be preserved in CSV output
    assert!(output.contains("user_日本語"));
    assert!(output.contains("Japanese Name 日本語"));
}

// === RQ-0195: Very wide data tests ===

#[test]
fn test_csv_very_long_strings() {
    let formatter = CsvFormatter;
    let long_string = "a".repeat(200);
    let results = vec![json!({"name": long_string, "value": "123"})];
    let output = formatter.format_search_results(&results).unwrap();
    // Long strings should be preserved
    assert!(output.contains(&"a".repeat(200)));
}

#[test]
fn test_csv_very_long_strings_with_commas() {
    let formatter = CsvFormatter;
    let long_string = "value, with, many, commas, ".repeat(20);
    let results = vec![json!({"name": long_string, "value": "123"})];
    let output = formatter.format_search_results(&results).unwrap();
    // Should be properly quoted
    assert!(output.contains("\"value, with, many, commas,"));
}
