//! License management endpoint tests.
//!
//! This module tests the Splunk license API:
//! - Getting license usage statistics
//! - Listing license pools
//! - Listing license stacks
//! - SplunkClient interface for license operations
//!
//! # Invariants
//! - License usage includes quota, used bytes, and breakdown by slave
//! - License pools and stacks are returned with their configuration
//!
//! # What this does NOT handle
//! - License installation or activation
//! - License violation handling

mod common;

use common::*;
use wiremock::matchers::{method, path};

#[tokio::test]
async fn test_get_license_usage() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("license/get_usage.json");

    Mock::given(method("GET"))
        .and(path("/services/licenser/usage"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result =
        endpoints::get_license_usage(&client, &mock_server.uri(), "test-token", 3, None, None)
            .await;

    assert!(result.is_ok());
    let usage = result.unwrap();
    assert_eq!(usage.len(), 1);
    assert_eq!(usage[0].quota, 53687091200);
    assert_eq!(usage[0].used_bytes, Some(1610612736));
    assert_eq!(usage[0].stack_id.as_deref(), Some("enterprise"));

    let slaves = usage[0].slaves_breakdown().unwrap();
    assert_eq!(
        slaves.get("00000000-0000-0000-0000-000000000000"),
        Some(&1073741824)
    );
}

#[tokio::test]
async fn test_list_license_pools() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("license/list_pools.json");

    Mock::given(method("GET"))
        .and(path("/services/licenser/pools"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result =
        endpoints::list_license_pools(&client, &mock_server.uri(), "test-token", 3, None, None)
            .await;

    assert!(result.is_ok());
    let pools = result.unwrap();
    assert_eq!(pools.len(), 1);
    assert_eq!(pools[0].name, "pool_enterprise");
    assert_eq!(pools[0].stack_id, "enterprise");
}

#[tokio::test]
async fn test_list_license_stacks() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("license/list_stacks.json");

    Mock::given(method("GET"))
        .and(path("/services/licenser/stacks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result =
        endpoints::list_license_stacks(&client, &mock_server.uri(), "test-token", 3, None, None)
            .await;

    assert!(result.is_ok());
    let stacks = result.unwrap();
    assert_eq!(stacks.len(), 1);
    assert_eq!(stacks[0].name, "enterprise");
    assert_eq!(stacks[0].label, "Enterprise");
    assert_eq!(stacks[0].type_name, "enterprise");
}

#[tokio::test]
async fn test_splunk_client_get_license_usage() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("license/get_usage.json");

    Mock::given(method("GET"))
        .and(path("/services/licenser/usage"))
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

    let result = client.get_license_usage().await;

    assert!(result.is_ok());
    let usage = result.unwrap();
    assert_eq!(usage.len(), 1);
    assert_eq!(usage[0].name, "daily_usage");
    assert_eq!(usage[0].quota, 53687091200);
}
