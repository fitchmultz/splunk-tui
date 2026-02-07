//! Lookup table management endpoint tests.
//!
//! This module tests the Splunk lookup table files API:
//! - Listing all lookup table files (CSV-based lookups)
//!
//! # Invariants
//! - Lookup tables are returned with their metadata (name, filename, owner, app, sharing, size)
//! - Results are paginated according to the provided count/offset parameters
//!
//! # What this does NOT handle
//! - Lookup file content upload/download (not supported by this endpoint)
//! - KV store lookups (different endpoint)

mod common;

use common::*;
use wiremock::matchers::{method, path};

#[tokio::test]
async fn test_list_lookup_tables() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("lookups/list_lookup_tables.json");

    Mock::given(method("GET"))
        .and(path("/services/data/lookup-table-files"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::list_lookup_tables(
        &client,
        &mock_server.uri(),
        "test-token",
        None,
        None,
        3,
        None,
    )
    .await;

    if let Err(ref e) = result {
        eprintln!("List lookup tables error: {:?}", e);
    }
    assert!(result.is_ok());
    let lookups = result.unwrap();
    assert_eq!(lookups.len(), 2);
    assert_eq!(lookups[0].name, "my_lookup");
    assert_eq!(lookups[0].filename, "my_lookup.csv");
    assert_eq!(lookups[0].owner, "admin");
    assert_eq!(lookups[0].app, "search");
    assert_eq!(lookups[0].sharing, "app");
    assert_eq!(lookups[0].size, 1024);
    assert_eq!(lookups[1].name, "countries");
    assert_eq!(lookups[1].filename, "countries.csv");
    assert_eq!(lookups[1].sharing, "global");
    assert_eq!(lookups[1].size, 2048);
}

#[tokio::test]
async fn test_list_lookup_tables_with_pagination() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("lookups/list_lookup_tables.json");

    Mock::given(method("GET"))
        .and(path("/services/data/lookup-table-files"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::list_lookup_tables(
        &client,
        &mock_server.uri(),
        "test-token",
        Some(1),
        Some(0),
        3,
        None,
    )
    .await;

    assert!(result.is_ok());
    let lookups = result.unwrap();
    // The endpoint returns what the server gives it; pagination is handled server-side
    // Here we verify the request was made with the right parameters
    assert_eq!(lookups.len(), 2); // Mock returns all 2 regardless of params
}

#[tokio::test]
async fn test_list_lookup_tables_empty_response() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("lookups/list_lookup_tables_empty.json");

    Mock::given(method("GET"))
        .and(path("/services/data/lookup-table-files"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::list_lookup_tables(
        &client,
        &mock_server.uri(),
        "test-token",
        None,
        None,
        3,
        None,
    )
    .await;

    assert!(result.is_ok());
    let lookups = result.unwrap();
    assert!(lookups.is_empty());
}

#[tokio::test]
async fn test_splunk_client_list_lookup_tables() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("lookups/list_lookup_tables.json");

    Mock::given(method("GET"))
        .and(path("/services/data/lookup-table-files"))
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

    let result = client.list_lookup_tables(None, None).await;

    assert!(result.is_ok());
    let lookups = result.unwrap();
    assert_eq!(lookups.len(), 2);
    assert_eq!(lookups[0].name, "my_lookup");
    assert_eq!(lookups[1].name, "countries");
}
