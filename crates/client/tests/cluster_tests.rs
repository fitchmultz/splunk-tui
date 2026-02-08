//! Cluster management endpoint tests.
//!
//! This module tests the Splunk cluster API:
//! - Getting cluster configuration and info
//!
//! # Invariants
//! - Cluster info includes mode (master, peer, search_head), replication factor, and search factor
//! - Cluster ID and label are returned for identification
//!
//! # What this does NOT handle
//! - Cluster peer management (add/remove peers)
//! - Cluster bundle operations
//! - Indexer cluster vs search head cluster distinctions

mod common;

use common::*;
use splunk_client::models::ClusterMode;
use wiremock::matchers::{method, path};

#[tokio::test]
async fn test_get_cluster_info() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("cluster/get_cluster_info.json");

    Mock::given(method("GET"))
        .and(path("/services/cluster/master/config"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result =
        endpoints::get_cluster_info(&client, &mock_server.uri(), "test-token", 3, None).await;

    assert!(result.is_ok());
    let info = result.unwrap();
    assert_eq!(info.id, "cluster-01");
    assert_eq!(info.label.as_deref(), Some("Production Cluster"));
    assert_eq!(info.mode, ClusterMode::Peer);
    assert_eq!(info.replication_factor, Some(3));
    assert_eq!(info.search_factor, Some(2));
}
