//! KVStore endpoint tests.
//!
//! This module tests the Splunk KVStore API:
//! - Getting KVStore status and replication information
//! - SplunkClient interface for KVStore operations
//!
//! # Invariants
//! - KVStore status includes current member info and replication status
//! - Replication status includes oplog size and timestamp
//!
//! # What this does NOT handle
//! - KVStore collection CRUD operations
//! - KVStore backup/restore

mod common;

use common::*;
use wiremock::matchers::{method, path};

#[tokio::test]
async fn test_get_kvstore_status() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("kvstore/status.json");

    Mock::given(method("GET"))
        .and(path("/services/kvstore/status"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result =
        endpoints::get_kvstore_status(&client, &mock_server.uri(), "test-token", 3, None).await;

    assert!(result.is_ok());
    let status = result.unwrap();
    assert_eq!(status.current_member.host, "splunk-idx-01");
    assert_eq!(status.replication_status.oplog_size, 1024);
}

#[tokio::test]
async fn test_splunk_client_get_kvstore_status() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("kvstore/status.json");

    Mock::given(method("GET"))
        .and(path("/services/kvstore/status"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    use secrecy::SecretString;
    use splunk_client::{AuthStrategy, SplunkClient};

    let strategy = AuthStrategy::ApiToken {
        token: SecretString::new("test-token".to_string().into()),
    };

    let client = SplunkClient::builder()
        .base_url(mock_server.uri())
        .auth_strategy(strategy)
        .build()
        .unwrap();

    let result = client.get_kvstore_status().await;

    assert!(result.is_ok());
    let status = result.unwrap();
    assert_eq!(status.current_member.host, "splunk-idx-01");
}
