//! Tests for cluster and health action redaction.

use std::collections::HashMap;

use splunk_client::models::{ClusterInfo, ClusterPeer, HealthCheckOutput, SplunkHealth};

use crate::action::tests::redacted_debug;
use crate::action::variants::Action;

#[test]
fn test_redact_cluster_peers_loaded() {
    let peers = vec![
        ClusterPeer {
            id: "peer1-id".to_string(),
            label: Some("peer1".to_string()),
            status: "Up".to_string(),
            peer_state: "Active".to_string(),
            site: None,
            guid: "guid1".to_string(),
            host: "internal-host1".to_string(),
            port: 8080,
            replication_count: None,
            replication_status: None,
            bundle_replication_count: None,
            is_captain: None,
        },
        ClusterPeer {
            id: "peer2-id".to_string(),
            label: Some("peer2".to_string()),
            status: "Up".to_string(),
            peer_state: "Active".to_string(),
            site: None,
            guid: "guid2".to_string(),
            host: "internal-host2".to_string(),
            port: 8080,
            replication_count: None,
            replication_status: None,
            bundle_replication_count: None,
            is_captain: None,
        },
    ];
    let action = Action::ClusterPeersLoaded(Ok(peers));
    let output = redacted_debug(&action);

    assert!(!output.contains("peer1"), "Should not contain peer name");
    assert!(
        !output.contains("internal-host1"),
        "Should not contain host"
    );
    assert!(
        output.contains("ClusterPeersLoaded"),
        "Should contain action name"
    );
    assert!(output.contains("2 items"), "Should show item count");
}

#[test]
fn test_redact_cluster_info_loaded() {
    let info = ClusterInfo {
        id: "cluster1-id".to_string(),
        label: Some("cluster1".to_string()),
        mode: "master".to_string(),
        manager_uri: None,
        replication_factor: None,
        search_factor: None,
        status: None,
        maintenance_mode: None,
    };
    let action = Action::ClusterInfoLoaded(Ok(info));
    let output = redacted_debug(&action);

    assert!(
        !output.contains("cluster1"),
        "Should not contain cluster name"
    );
    assert!(
        output.contains("ClusterInfoLoaded"),
        "Should contain action name"
    );
    assert!(output.contains("<data>"), "Should show data indicator");
}

#[test]
fn test_redact_health_loaded() {
    let health = HealthCheckOutput {
        server_info: None,
        splunkd_health: None,
        license_usage: None,
        kvstore_status: None,
        log_parsing_health: None,
    };
    let action = Action::HealthLoaded(Box::new(Ok(health)));
    let output = redacted_debug(&action);

    assert!(
        output.contains("HealthLoaded"),
        "Should contain action name"
    );
    assert!(output.contains("<data>"), "Should show data indicator");
}

#[test]
fn test_redact_health_status_loaded() {
    let health = SplunkHealth {
        health: "yellow".to_string(),
        features: HashMap::new(),
    };
    let action = Action::HealthStatusLoaded(Ok(health));
    let output = redacted_debug(&action);

    assert!(!output.contains("yellow"), "Should not contain status");
    assert!(
        output.contains("HealthStatusLoaded"),
        "Should contain action name"
    );
    assert!(output.contains("<data>"), "Should show data indicator");
}
