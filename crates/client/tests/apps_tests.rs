//! App management endpoint tests.
//!
//! This module tests the Splunk apps API:
//! - Listing installed apps with pagination
//! - Getting specific app details
//! - Enabling/disabling apps
//!
//! # Invariants
//! - App list includes name, label, version, disabled status, and metadata
//! - get_app returns detailed info for a single app
//! - update_app (enable/disable) returns success on valid requests
//!
//! # What this does NOT handle
//! - App installation/removal
//! - App configuration beyond enable/disable

mod common;

use common::*;
use splunk_client::error::ClientError;
use wiremock::matchers::{method, path, query_param};

#[tokio::test]
async fn test_list_apps() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("apps/list_apps.json");

    Mock::given(method("GET"))
        .and(path("/services/apps/local"))
        .and(query_param("output_mode", "json"))
        .and(query_param("count", "30"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::list_apps(
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
    let apps = result.unwrap();
    assert_eq!(apps.len(), 3);

    // Verify first app
    assert_eq!(apps[0].name, "search");
    assert_eq!(apps[0].label, Some("Search & Reporting".to_string()));
    assert_eq!(apps[0].version, Some("9.1.2".to_string()));
    assert!(!apps[0].disabled);

    // Verify disabled app
    let disabled_app = apps.iter().find(|a| a.name == "disabled_app").unwrap();
    assert!(disabled_app.disabled);
    assert_eq!(disabled_app.label, Some("Disabled App".to_string()));
}

#[tokio::test]
async fn test_list_apps_with_pagination() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("apps/list_apps.json");

    Mock::given(method("GET"))
        .and(path("/services/apps/local"))
        .and(query_param("output_mode", "json"))
        .and(query_param("count", "10"))
        .and(query_param("offset", "5"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::list_apps(
        &client,
        &mock_server.uri(),
        "test-token",
        Some(10),
        Some(5),
        3,
        None,
    )
    .await;

    assert!(result.is_ok());
    let apps = result.unwrap();
    assert_eq!(apps.len(), 3);
}

#[tokio::test]
async fn test_get_app() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("apps/get_app.json");

    Mock::given(method("GET"))
        .and(path("/services/apps/local/search"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result =
        endpoints::get_app(&client, &mock_server.uri(), "test-token", "search", 3, None).await;

    assert!(result.is_ok());
    let app = result.unwrap();
    assert_eq!(app.name, "search");
    assert_eq!(app.label, Some("Search & Reporting".to_string()));
    assert_eq!(app.version, Some("9.1.2".to_string()));
    assert!(!app.disabled);
    assert_eq!(app.description, Some("Splunk Search app".to_string()));
    assert_eq!(app.author, Some("Splunk Inc.".to_string()));
}

#[tokio::test]
async fn test_get_app_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/apps/local/nonexistent"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(404).set_body_string("Not found"))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::get_app(
        &client,
        &mock_server.uri(),
        "test-token",
        "nonexistent",
        3,
        None,
    )
    .await;

    assert!(result.is_err());
    // Should be an API error with 404 status
    let err = result.unwrap_err();
    assert!(matches!(err, ClientError::ApiError { status: 404, .. }));
}

#[tokio::test]
async fn test_enable_app() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("apps/update_app.json");

    Mock::given(method("POST"))
        .and(path("/services/apps/local/test_app"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::update_app(
        &client,
        &mock_server.uri(),
        "test-token",
        "test_app",
        false, // disabled = false means enable
        3,
        None,
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_disable_app() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("apps/update_app.json");

    Mock::given(method("POST"))
        .and(path("/services/apps/local/test_app"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::update_app(
        &client,
        &mock_server.uri(),
        "test-token",
        "test_app",
        true, // disabled = true means disable
        3,
        None,
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_list_apps_empty() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("apps/list_apps_empty.json");

    Mock::given(method("GET"))
        .and(path("/services/apps/local"))
        .and(query_param("output_mode", "json"))
        .and(query_param("count", "30"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::list_apps(
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
    let apps = result.unwrap();
    assert!(apps.is_empty());
}
