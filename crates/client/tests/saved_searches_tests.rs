//! Saved searches endpoint tests.
//!
//! This module tests the Splunk saved searches API:
//! - Listing all saved searches
//! - Creating new saved searches
//! - Updating saved searches
//! - Deleting saved searches
//! - SplunkClient interface for saved searches
//!
//! # Invariants
//! - Saved searches are returned with their names, queries, and metadata
//! - Creating a saved search requires a unique name and valid SPL query
//! - Updating a saved search only modifies provided fields
//! - Deleting a saved search returns success for existing searches
//!
//! # What this does NOT handle
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
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::list_saved_searches(
        &client,
        &mock_server.uri(),
        "test-token",
        None,
        None,
        3,
        None,
        None,
    )
    .await;

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
    let params = splunk_client::models::SavedSearchCreateParams {
        name: "my-search".to_string(),
        search: "| makeresults".to_string(),
        ..Default::default()
    };
    let result = endpoints::create_saved_search(
        &client,
        &mock_server.uri(),
        "test-token",
        &params.name,
        &params.search,
        3,
        None,
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

    let client = SplunkClient::builder()
        .base_url(mock_server.uri())
        .auth_strategy(strategy)
        .build()
        .unwrap();

    let result = client.list_saved_searches(None, None).await;

    assert!(result.is_ok());
    let searches = result.unwrap();
    assert_eq!(searches.len(), 2);
    assert_eq!(searches[0].name, "Errors in the last 24 hours");
}

#[tokio::test]
async fn test_list_saved_searches_with_pagination() {
    let mock_server = MockServer::start().await;
    let fixture = load_fixture("search/list_saved_searches.json");

    // Test with count=1
    Mock::given(method("GET"))
        .and(path("/services/saved/searches"))
        .and(query_param("output_mode", "json"))
        .and(query_param("count", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::list_saved_searches(
        &client,
        &mock_server.uri(),
        "test-token",
        Some(1),
        None,
        3,
        None,
        None,
    )
    .await;

    assert!(result.is_ok());

    // Test with offset
    Mock::given(method("GET"))
        .and(path("/services/saved/searches"))
        .and(query_param("output_mode", "json"))
        .and(query_param("count", "10"))
        .and(query_param("offset", "5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let result = endpoints::list_saved_searches(
        &client,
        &mock_server.uri(),
        "test-token",
        Some(10),
        Some(5),
        3,
        None,
        None,
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_get_saved_search_success() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/get_saved_search.json");

    Mock::given(method("GET"))
        .and(path(
            "/services/saved/searches/Errors%20in%20the%20last%2024%20hours",
        ))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::get_saved_search(
        &client,
        &mock_server.uri(),
        "test-token",
        "Errors in the last 24 hours",
        3,
        None,
        None,
    )
    .await;

    assert!(result.is_ok());
    let search = result.unwrap();
    assert_eq!(search.name, "Errors in the last 24 hours");
    assert_eq!(
        search.search,
        "index=_internal sourcetype=splunkd log_level=ERROR | head 100"
    );
    assert_eq!(
        search.description,
        Some("Finds error messages in internal logs".to_string())
    );
    assert!(!search.disabled);
}

#[tokio::test]
async fn test_get_saved_search_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/saved/searches/NonExistentSearch"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::get_saved_search(
        &client,
        &mock_server.uri(),
        "test-token",
        "NonExistentSearch",
        3,
        None,
        None,
    )
    .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("not found"),
        "Error should indicate resource not found: {}",
        err
    );
}

#[tokio::test]
async fn test_splunk_client_get_saved_search() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/get_saved_search.json");

    Mock::given(method("GET"))
        .and(path(
            "/services/saved/searches/Errors%20in%20the%20last%2024%20hours",
        ))
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

    let result = client.get_saved_search("Errors in the last 24 hours").await;

    assert!(result.is_ok());
    let search = result.unwrap();
    assert_eq!(search.name, "Errors in the last 24 hours");
    assert_eq!(
        search.search,
        "index=_internal sourcetype=splunkd log_level=ERROR | head 100"
    );
}

#[tokio::test]
async fn test_update_saved_search_success() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/services/saved/searches/my-search"))
        .and(query_param("output_mode", "json"))
        .and(body_string_contains("search=index%3Dmain"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
        .mount(&mock_server)
        .await;

    let params = endpoints::SavedSearchUpdateParams {
        search: Some("index=main"),
        description: None,
        disabled: None,
    };

    let client = Client::new();
    let result = endpoints::update_saved_search(
        &client,
        &mock_server.uri(),
        "test-token",
        "my-search",
        &params,
        3,
        None,
        None,
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_update_saved_search_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/services/saved/searches/NonExistentSearch"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
        .mount(&mock_server)
        .await;

    let params = endpoints::SavedSearchUpdateParams {
        search: Some("index=main"),
        description: None,
        disabled: None,
    };

    let client = Client::new();
    let result = endpoints::update_saved_search(
        &client,
        &mock_server.uri(),
        "test-token",
        "NonExistentSearch",
        &params,
        3,
        None,
        None,
    )
    .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("not found"),
        "Error should indicate resource not found: {}",
        err
    );
}

#[tokio::test]
async fn test_splunk_client_update_saved_search() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/services/saved/searches/my-search"))
        .and(query_param("output_mode", "json"))
        .and(body_string_contains("search=index%3Dmain"))
        .and(body_string_contains("description=Updated+description"))
        .and(body_string_contains("disabled=true"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
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

    let result = client
        .update_saved_search(
            "my-search",
            splunk_client::models::SavedSearchUpdateParams {
                search: Some("index=main".to_string()),
                description: Some("Updated description".to_string()),
                disabled: Some(true),
            },
        )
        .await;

    assert!(result.is_ok());
}
