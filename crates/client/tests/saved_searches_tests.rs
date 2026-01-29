//! Saved searches endpoint tests.
//!
//! This module tests the Splunk saved searches API:
//! - Listing all saved searches
//! - Creating new saved searches
//! - Deleting saved searches
//! - SplunkClient interface for saved searches
//!
//! # Invariants
//! - Saved searches are returned with their names, queries, and metadata
//! - Creating a saved search requires a unique name and valid SPL query
//! - Deleting a saved search returns success for existing searches
//!
//! # What this does NOT handle
//! - Updating existing saved searches
//! - Scheduling configuration for saved searches

mod common;

use common::*;
use wiremock::matchers::{body_string_contains, method, path, query_param};

#[tokio::test]
async fn test_list_saved_searches() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/list_saved_searches.json");

    Mock::given(method("GET"))
        .and(path("/services/saved/searches"))
        .and(query_param("output_mode", "json"))
        .and(query_param("count", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result =
        endpoints::list_saved_searches(&client, &mock_server.uri(), "test-token", 3, None).await;

    assert!(result.is_ok());
    let searches = result.unwrap();
    assert_eq!(searches.len(), 2);
    assert_eq!(searches[0].name, "Errors in the last 24 hours");
    assert_eq!(
        searches[0].search,
        "index=_internal sourcetype=splunkd log_level=ERROR | head 100"
    );
    assert_eq!(searches[1].name, "Disabled Search");
    assert!(searches[1].disabled);
}

#[tokio::test]
async fn test_create_saved_search() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/services/saved/searches"))
        .and(query_param("output_mode", "json"))
        .and(body_string_contains("name=my-search"))
        .and(body_string_contains("search=%7C+makeresults"))
        .respond_with(ResponseTemplate::new(201).set_body_string("{}"))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::create_saved_search(
        &client,
        &mock_server.uri(),
        "test-token",
        "my-search",
        "| makeresults",
        3,
        None,
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_delete_saved_search() {
    let mock_server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/services/saved/searches/my-search"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::delete_saved_search(
        &client,
        &mock_server.uri(),
        "test-token",
        "my-search",
        3,
        None,
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_splunk_client_list_saved_searches() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/list_saved_searches.json");

    Mock::given(method("GET"))
        .and(path("/services/saved/searches"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    use secrecy::SecretString;
    use splunk_client::{AuthStrategy, SplunkClient};

    let strategy = AuthStrategy::ApiToken {
        token: SecretString::new("test-token".to_string().into()),
    };

    let mut client = SplunkClient::builder()
        .base_url(mock_server.uri())
        .auth_strategy(strategy)
        .build()
        .unwrap();

    let result = client.list_saved_searches().await;

    assert!(result.is_ok());
    let searches = result.unwrap();
    assert_eq!(searches.len(), 2);
    assert_eq!(searches[0].name, "Errors in the last 24 hours");
}
