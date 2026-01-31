//! JSON formatter tests.

use crate::formatters::{ClusterInfoOutput, ClusterPeerOutput, Formatter, JsonFormatter};
use serde_json::json;
use splunk_client::{Index, KvStoreMember, KvStoreReplicationStatus, KvStoreStatus, User};

#[test]
fn test_json_formatter() {
    let formatter = JsonFormatter;
    let results = vec![json!({"name": "test", "value": "123"})];
    let output = formatter.format_search_results(&results).unwrap();
    assert!(output.contains("test"));
    assert!(output.contains("123"));
}

#[test]
fn test_json_formatter_indexes_always_detailed() {
    let formatter = JsonFormatter;
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
    // JSON always outputs all fields regardless of detailed flag
    let output_basic = formatter.format_indexes(&indexes, false).unwrap();
    let output_detailed = formatter.format_indexes(&indexes, true).unwrap();
    // Check that the JSON contains the expected fields (using serde rename names)
    assert!(output_basic.contains("\"name\""));
    assert!(output_basic.contains("\"homePath\""));
    assert!(output_basic.contains("\"coldDBPath\""));
    // Both should be identical since JSON ignores the detailed flag
    assert_eq!(output_basic, output_detailed);
}

#[test]
fn test_cluster_peers_json_formatting() {
    let formatter = JsonFormatter;
    let cluster_info = ClusterInfoOutput {
        id: "cluster-1".to_string(),
        label: Some("test-cluster".to_string()),
        mode: "master".to_string(),
        manager_uri: Some("https://master:8089".to_string()),
        replication_factor: Some(3),
        search_factor: Some(2),
        status: Some("Enabled".to_string()),
        peers: Some(vec![
            ClusterPeerOutput {
                host: "peer1".to_string(),
                port: 8089,
                id: "peer-1".to_string(),
                status: "Up".to_string(),
                peer_state: "Ready".to_string(),
                label: Some("Peer 1".to_string()),
                site: Some("site1".to_string()),
                is_captain: true,
            },
            ClusterPeerOutput {
                host: "peer2".to_string(),
                port: 8089,
                id: "peer-2".to_string(),
                status: "Up".to_string(),
                peer_state: "Ready".to_string(),
                label: None,
                site: None,
                is_captain: false,
            },
        ]),
    };
    let output = formatter.format_cluster_info(&cluster_info, true).unwrap();
    // Verify JSON structure includes peers array
    assert!(output.contains("\"peers\""));
    assert!(output.contains("\"host\""));
    assert!(output.contains("\"peer1\""));
    assert!(output.contains("\"peer2\""));
    assert!(output.contains("\"is_captain\""));
    assert!(output.contains("true"));
    assert!(output.contains("false"));
}

#[test]
fn test_kvstore_peers_json_formatting() {
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
    let output = JsonFormatter.format_kvstore_status(&status).unwrap();
    assert!(output.contains("\"currentMember\""));
    assert!(output.contains("\"replicationStatus\""));
    assert!(output.contains("\"localhost\""));
    assert!(output.contains("\"rs0\""));
}

#[test]
fn test_users_json_formatting() {
    let formatter = JsonFormatter;
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
    assert!(output.contains("\"name\""));
    assert!(output.contains("\"admin\""));
    assert!(output.contains("\"realname\""));
    assert!(output.contains("\"Administrator\""));
    assert!(output.contains("\"type\""));
    assert!(output.contains("\"Splunk\""));
    assert!(output.contains("\"defaultApp\""));
    assert!(output.contains("\"launcher\""));
    assert!(output.contains("\"roles\""));
    assert!(output.contains("\"admin\""));
    assert!(output.contains("\"power\""));
    assert!(output.contains("\"lastSuccessfulLogin\""));
    assert!(output.contains("1704067200"));
}

// === RQ-0195: Null/missing fields tests ===

#[test]
fn test_json_null_fields() {
    let formatter = JsonFormatter;
    let results = vec![json!({"name": "test", "optional": null, "present": "value"})];
    let output = formatter.format_search_results(&results).unwrap();
    // Null should be serialized as null
    assert!(output.contains("\"optional\": null"));
    assert!(output.contains("\"present\": \"value\""));
}

// === RQ-0195: Unicode tests ===

#[test]
fn test_json_unicode_escaping() {
    let formatter = JsonFormatter;
    let results = vec![json!({"name": "日本語", "emoji": "🎉"})];
    let output = formatter.format_search_results(&results).unwrap();
    // Unicode should be preserved (not escaped) in pretty-printed JSON
    assert!(output.contains("日本語"));
    assert!(output.contains("🎉"));
}
