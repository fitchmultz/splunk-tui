//! Cluster side effect handler tests.
//!
//! This module tests the cluster-related side effect handlers including
//! LoadClusterInfo and LoadClusterPeers.

mod common;

use common::*;
use wiremock::matchers::{method, path};

#[tokio::test]
async fn test_load_cluster_info_success() {
    let mut harness = SideEffectsTestHarness::new().await;

    let fixture = load_fixture("cluster/get_cluster_info.json");
    Mock::given(method("GET"))
        .and(path("/services/cluster/master/config"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&harness.mock_server)
        .await;

    let actions = harness.handle_and_collect(Action::LoadClusterInfo, 2).await;

    assert!(
        actions.iter().any(|a| matches!(a, Action::Loading(true))),
        "Should send Loading(true)"
    );
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::ClusterInfoLoaded(Ok(_)))),
        "Should send ClusterInfoLoaded(Ok)"
    );
}

#[tokio::test]
async fn test_load_cluster_info_error() {
    let mut harness = SideEffectsTestHarness::new().await;

    Mock::given(method("GET"))
        .and(path("/services/cluster/master/config"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&harness.mock_server)
        .await;

    let actions = harness.handle_and_collect(Action::LoadClusterInfo, 2).await;

    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::ClusterInfoLoaded(Err(_)))),
        "Should send ClusterInfoLoaded(Err)"
    );
}

#[tokio::test]
async fn test_load_cluster_peers_success() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Cluster peers uses the same fixture structure
    let fixture = serde_json::json!({
        "entry": [
            {
                "name": "peer1",
                "content": {
                    "id": "peer-01",
                    "label": "Peer 1",
                    "status": "Up",
                    "peer_state": "searchable",
                    "guid": "guid-01",
                    "host": "peer1.example.com",
                    "port": 8089
                }
            }
        ]
    });

    Mock::given(method("GET"))
        .and(path("/services/cluster/master/peers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(Action::LoadClusterPeers, 2)
        .await;

    assert!(
        actions.iter().any(|a| matches!(a, Action::Loading(true))),
        "Should send Loading(true)"
    );
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::ClusterPeersLoaded(Ok(_)))),
        "Should send ClusterPeersLoaded(Ok)"
    );
}
