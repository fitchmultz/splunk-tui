//! Tests for formatters module.

use super::common::{escape_csv, escape_xml, flatten_json_object, format_json_value};
use super::{
    ClusterInfoOutput, ClusterPeerOutput, CsvFormatter, Formatter, JsonFormatter,
    LicenseInfoOutput, OutputFormat, TableFormatter, XmlFormatter,
};
use serde_json::json;
use splunk_client::models::LogEntry;
use splunk_client::{
    App, Index, KvStoreMember, KvStoreReplicationStatus, KvStoreStatus, LicensePool, LicenseStack,
    LicenseUsage, SavedSearch, SearchJobStatus, User,
};
use splunk_config::types::ProfileConfig;

#[test]
fn test_output_format_from_str() {
    assert_eq!(
        OutputFormat::from_str("json").unwrap(),
        super::OutputFormat::Json
    );
    assert_eq!(
        OutputFormat::from_str("JSON").unwrap(),
        super::OutputFormat::Json
    );
    assert_eq!(
        OutputFormat::from_str("csv").unwrap(),
        super::OutputFormat::Csv
    );
    assert_eq!(
        OutputFormat::from_str("xml").unwrap(),
        super::OutputFormat::Xml
    );
    assert_eq!(
        OutputFormat::from_str("table").unwrap(),
        super::OutputFormat::Table
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
    // New format uses nested elements instead of field attributes
    assert!(output.contains("<name>test</name>"));
    assert!(output.contains("<value>123</value>"));
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
            status: "Ready".to_string(),
        },
        replication_status: KvStoreReplicationStatus {
            oplog_size: 100,
            oplog_used: 1.5,
        },
    };
    let output = TableFormatter.format_kvstore_status(&status).unwrap();
    assert!(output.contains("KVStore Status:"));
    assert!(output.contains("localhost:8089"));
    assert!(output.contains("Status: Ready"));
    assert!(output.contains("Replica Set: rs0"));
    assert!(output.contains("Oplog Size: 100 MB"));
    assert!(output.contains("Oplog Used: 1.50%"));
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
    use super::common::get_all_flattened_keys;
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

#[test]
fn test_users_table_formatting() {
    let formatter = TableFormatter;
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
    assert!(output.contains("</roles>"));
    assert!(output.contains("<lastSuccessfulLogin>1704067200</lastSuccessfulLogin>"));
    assert!(output.contains("<name>user1</name>"));
    assert!(output.contains("</users>"));
}

// === RQ-0195: Empty result set tests for all formatters ===

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

// === RQ-0195: Unicode and special character tests ===

#[test]
fn test_table_unicode_in_users() {
    let formatter = TableFormatter;
    let users = vec![
        User {
            name: "user_日本語".to_string(),
            realname: Some("Japanese Name 日本語".to_string()),
            email: Some("test@example.com".to_string()),
            user_type: Some("Splunk".to_string()),
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

#[test]
fn test_json_unicode_escaping() {
    let formatter = JsonFormatter;
    let results = vec![json!({"name": "日本語", "emoji": "🎉"})];
    let output = formatter.format_search_results(&results).unwrap();
    // Unicode should be preserved (not escaped) in pretty-printed JSON
    assert!(output.contains("日本語"));
    assert!(output.contains("🎉"));
}

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
fn test_table_very_long_strings() {
    let formatter = TableFormatter;
    let long_string = "a".repeat(200);
    let results = vec![json!({"name": long_string, "value": "123"})];
    let output = formatter.format_search_results(&results).unwrap();
    // Long strings should be preserved
    assert!(output.contains(&"a".repeat(200)));
}

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

#[test]
fn test_json_null_fields() {
    let formatter = JsonFormatter;
    let results = vec![json!({"name": "test", "optional": null, "present": "value"})];
    let output = formatter.format_search_results(&results).unwrap();
    // Null should be serialized as null
    assert!(output.contains("\"optional\": null"));
    assert!(output.contains("\"present\": \"value\""));
}

#[test]
fn test_xml_null_fields() {
    let formatter = XmlFormatter;
    let results = vec![json!({"name": "test", "optional": null})];
    let output = formatter.format_search_results(&results).unwrap();
    // Null fields should produce empty elements
    assert!(output.contains("<optional></optional>"));
    assert!(output.contains("<name>test</name>"));
}

// === RQ-0195: format_json_value edge cases ===

#[test]
fn test_format_json_value_deeply_nested() {
    // Create a deeply nested structure (10 levels)
    let mut value = json!("deep");
    for _ in 0..10 {
        value = json!({"level": value});
    }
    let result = format_json_value(&value);
    // Should serialize without panicking
    assert!(result.contains("deep"));
    assert!(result.starts_with("{"));
}

#[test]
fn test_format_json_value_large_array() {
    // Create an array with many elements
    let arr: Vec<i32> = (0..100).collect();
    let value = json!(arr);
    let result = format_json_value(&value);
    // Should serialize without panicking
    assert!(result.contains("0"));
    assert!(result.contains("99"));
    assert!(result.starts_with("["));
}

#[test]
fn test_format_json_value_mixed_types() {
    let value = json!({
        "string": "text",
        "number": 42,
        "float": 1.23456_f64,
        "bool": true,
        "null": null,
        "array": [1, "two", 3.0, false, null],
        "nested": {"key": "value"}
    });
    let result = format_json_value(&value);
    // Should handle all types
    assert!(result.contains("text"));
    assert!(result.contains("42"));
    assert!(result.contains("1.23456"));
    assert!(result.contains("true"));
    assert!(result.starts_with("{"));
}

#[test]
fn test_format_json_value_empty_structures() {
    // Empty array
    let empty_arr = json!([]);
    assert_eq!(format_json_value(&empty_arr), "[]");

    // Empty object
    let empty_obj = json!({});
    assert_eq!(format_json_value(&empty_obj), "{}");

    // Empty string
    let empty_str = json!("");
    assert_eq!(format_json_value(&empty_str), "");
}
