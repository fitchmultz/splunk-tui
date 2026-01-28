//! Server information and health endpoint tests.
//!
//! This module tests the Splunk server API:
//! - Getting server information (version, roles, mode)
//! - Getting health status for splunkd and features
//!
//! # Invariants
//! - Server info includes version, server name, mode (standalone, distributed), and roles
//! - Health status includes overall health and per-feature health/status
//!
//! # What this does NOT handle
//! - Server configuration changes
//! - Feature enablement/disablement

mod common;

use common::*;
use wiremock::matchers::{method, path};

#[tokio::test]
async fn test_get_server_info() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("server/get_server_info.json");

    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::get_server_info(&client, &mock_server.uri(), "test-token", 3).await;

    assert!(result.is_ok());
    let info = result.unwrap();
    assert_eq!(info.server_name, "splunk-local");
    assert_eq!(info.version, "9.1.2");
    assert_eq!(info.mode.as_deref(), Some("standalone"));
    assert!(info.server_roles.contains(&"search_head".to_string()));
    assert!(info.server_roles.contains(&"indexer".to_string()));
}

#[tokio::test]
async fn test_get_health() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("server/get_health.json");

    Mock::given(method("GET"))
        .and(path("/services/server/health/splunkd"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::get_health(&client, &mock_server.uri(), "test-token", 3).await;

    assert!(result.is_ok());
    let health = result.unwrap();
    assert_eq!(health.health, "green");
    assert!(health.features.contains_key("KVStore"));
    assert_eq!(health.features["KVStore"].health, "green");
    assert_eq!(health.features["KVStore"].status, "enabled");
    assert_eq!(health.features["SearchScheduler"].health, "green");
}
