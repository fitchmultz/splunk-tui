//! Table formatter tests.

use crate::formatters::{
    ClusterInfoOutput, ClusterPeerOutput, Formatter, LicenseInfoOutput, TableFormatter,
};
use serde_json::json;
use splunk_client::models::{KvStoreMemberStatus, PeerState, PeerStatus, UserType};
use splunk_client::{
    Index, KvStoreMember, KvStoreReplicationStatus, KvStoreStatus, LicensePool, LicenseStack,
    LicenseUsage, User,
};

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
fn test_table_formatter_indexes_basic() {
    let formatter = TableFormatter;
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
    assert!(output.contains("Name"));
    assert!(output.contains("Size (MB)"));
    assert!(output.contains("main"));
    assert!(!output.contains("Home Path"));
    assert!(!output.contains("Cold Path"));
    assert!(!output.contains("Retention"));
}

#[test]
fn test_table_formatter_indexes_detailed() {
    let formatter = TableFormatter;
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
    assert!(output.contains("Name"));
    assert!(output.contains("Size (MB)"));
    assert!(output.contains("main"));
    assert!(output.contains("Home Path"));
    assert!(output.contains("Cold Path"));
    assert!(output.contains("Thawed Path"));
    assert!(output.contains("Retention (s)"));
    assert!(output.contains("2592000"));
    assert!(output.contains("/opt/splunk/var/lib/splunk/main/db"));
}

#[test]
fn test_cluster_peers_table_formatting() {
    let formatter = TableFormatter;
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
            status: PeerStatus::Up.to_string(),
            peer_state: PeerState::Searchable.to_string(),
            label: Some("Peer 1".to_string()),
            site: Some("site1".to_string()),
            is_captain: true,
        }]),
    };
    let output = formatter.format_cluster_info(&cluster_info, true).unwrap();
    // Verify table structure includes peers
    assert!(output.contains("Cluster Information:"));
    assert!(output.contains("ID: cluster-1"));
    assert!(output.contains("Cluster Peers (1)"));
    assert!(output.contains("Host: peer1:8089"));
    assert!(output.contains("Captain: Yes"));
}

#[test]
fn test_kvstore_peers_table_formatting() {
    let status = KvStoreStatus {
        current_member: KvStoreMember {
            guid: "guid".to_string(),
            host: "localhost".to_string(),
            port: 8089,
            replica_set: "rs0".to_string(),
            status: KvStoreMemberStatus::Ready,
        },
        replication_status: KvStoreReplicationStatus {
            oplog_size: 100,
            oplog_used: 1.5,
        },
    };
    let output = TableFormatter.format_kvstore_status(&status).unwrap();
    assert!(output.contains("KVStore Status:"));
    assert!(output.contains("localhost:8089"));
    assert!(output.contains("Status: ready"));
    assert!(output.contains("Replica Set: rs0"));
    assert!(output.contains("Oplog Size: 100 MB"));
    assert!(output.contains("Oplog Used: 1.50%"));
}

#[test]
fn test_format_license_table() {
    let formatter = TableFormatter;
    let license = LicenseInfoOutput {
        usage: vec![LicenseUsage {
            name: "daily_usage".to_string(),
            quota: 100 * 1024 * 1024,
            used_bytes: Some(50 * 1024 * 1024),
            slaves_usage_bytes: None,
            stack_id: Some("enterprise".to_string()),
        }],
        pools: vec![LicensePool {
            name: "pool1".to_string(),
            quota: (50 * 1024 * 1024).to_string(),
            used_bytes: 25 * 1024 * 1024,
            stack_id: "enterprise".to_string(),
            description: Some("Test pool".to_string()),
        }],
        stacks: vec![LicenseStack {
            name: "enterprise".to_string(),
            quota: 100 * 1024 * 1024,
            type_name: "enterprise".to_string(),
            label: "Enterprise".to_string(),
        }],
    };

    let output = formatter.format_license(&license).unwrap();
    assert!(output.contains("daily_usage"));
    assert!(output.contains("50.0%"));
    assert!(output.contains("pool1"));
    assert!(output.contains("Test pool"));
    assert!(output.contains("Enterprise"));
}

#[test]
fn test_users_table_formatting() {
    let formatter = TableFormatter;
    let users = vec![
        User {
            name: "admin".to_string(),
            realname: Some("Administrator".to_string()),
            email: Some("admin@example.com".to_string()),
            user_type: Some(UserType::Splunk),
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
    assert!(output.contains("NAME"));
    assert!(output.contains("REAL NAME"));
    assert!(output.contains("TYPE"));
    assert!(output.contains("ROLES"));
    assert!(output.contains("admin"));
    assert!(output.contains("Administrator"));
    assert!(output.contains("Splunk"));
    assert!(output.contains("admin, power"));
    assert!(output.contains("user1"));
}

#[test]
fn test_users_table_empty() {
    let formatter = TableFormatter;
    let users: Vec<User> = vec![];
    let output = formatter.format_users(&users).unwrap();
    assert!(output.contains("No users found"));
}

// === RQ-0195: Null/missing fields tests ===

#[test]
fn test_table_null_fields_in_users() {
    let formatter = TableFormatter;
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
    // Null fields should show as "-"
    assert!(output.contains("user1"));
    // The row should exist and contain the name
    assert!(output.contains("NAME"));
}

// === RQ-0195: Unicode tests ===

#[test]
fn test_table_unicode_in_users() {
    let formatter = TableFormatter;
    let users = vec![
        User {
            name: "user_日本語".to_string(),
            realname: Some("Japanese Name 日本語".to_string()),
            email: Some("test@example.com".to_string()),
            user_type: Some(UserType::Splunk),
            default_app: Some("launcher".to_string()),
            roles: vec!["admin".to_string()],
            last_successful_login: Some(1704067200),
        },
        User {
            name: "user_emoji".to_string(),
            realname: Some("User with Emoji 🎉🚀".to_string()),
            email: None,
            user_type: None,
            default_app: None,
            roles: vec![],
            last_successful_login: None,
        },
    ];
    let output = formatter.format_users(&users).unwrap();
    // Unicode characters should be preserved
    assert!(output.contains("日本語"));
    assert!(output.contains("🎉"));
    assert!(output.contains("🚀"));
}

#[test]
fn test_table_wide_characters() {
    let formatter = TableFormatter;
    let users = vec![User {
        name: "user_cn".to_string(),
        realname: Some("中文用户".to_string()),
        email: None,
        user_type: None,
        default_app: None,
        roles: vec![],
        last_successful_login: None,
    }];
    let output = formatter.format_users(&users).unwrap();
    // CJK characters should be preserved
    assert!(output.contains("中文用户"));
}

// === RQ-0195: Special characters tests ===

#[test]
fn test_table_special_chars_in_search_results() {
    let formatter = TableFormatter;
    let results = vec![
        json!({"name": "test\twith\ttabs", "value": "123"}),
        json!({"name": "test\nwith\nnewlines", "value": "456"}),
        json!({"name": "test\r\nwith\r\ncrlf", "value": "789"}),
    ];
    let output = formatter.format_search_results(&results).unwrap();
    // Special characters should be preserved in output
    assert!(output.contains("test\twith\ttabs"));
    assert!(output.contains("test\nwith\nnewlines"));
    assert!(output.contains("test\r\nwith\r\ncrlf"));
}

// === RQ-0195: Very wide data tests ===

#[test]
fn test_table_very_long_strings() {
    let formatter = TableFormatter;
    let long_string = "a".repeat(200);
    let results = vec![json!({"name": long_string, "value": "123"})];
    let output = formatter.format_search_results(&results).unwrap();
    // Long strings should be preserved
    assert!(output.contains(&"a".repeat(200)));
}
