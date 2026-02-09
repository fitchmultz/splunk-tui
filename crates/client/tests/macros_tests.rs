//! Search macro endpoint tests.
//!
//! This module tests the Splunk macro API:
//! - Listing all macros
//! - Creating new macros
//! - Updating macros
//! - Deleting macros
//! - Request struct construction and validation

mod common;

use common::*;
use splunk_client::endpoints::{CreateMacroRequest, UpdateMacroRequest};
use wiremock::matchers::{body_string_contains, method, path, query_param};

#[tokio::test]
async fn test_list_macros() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("macros/list_macros.json");

    Mock::given(method("GET"))
        .and(path("/services/admin/macros"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result =
        endpoints::list_macros(&client, &mock_server.uri(), "test-token", 3, None, None).await;

    assert!(result.is_ok());
    let macros = result.unwrap();
    assert!(!macros.is_empty());
}

#[tokio::test]
async fn test_create_macro_request_struct() {
    // Test CreateMacroRequest construction with required fields
    let request = CreateMacroRequest::new("test_macro", "| makeresults");
    assert_eq!(request.name, "test_macro");
    assert_eq!(request.definition, "| makeresults");
    assert_eq!(request.args, None);
    assert_eq!(request.description, None);
    assert!(!request.disabled);
    assert!(!request.iseval);
    assert_eq!(request.validation, None);
    assert_eq!(request.errormsg, None);
}

#[tokio::test]
async fn test_create_macro_request_with_all_fields() {
    let request = CreateMacroRequest {
        name: "test_macro",
        definition: "| makeresults | eval x=1",
        args: Some("arg1,arg2"),
        description: Some("Test macro description"),
        disabled: true,
        iseval: false,
        validation: Some("x > 0"),
        errormsg: Some("x must be positive"),
    };

    assert_eq!(request.name, "test_macro");
    assert_eq!(request.definition, "| makeresults | eval x=1");
    assert_eq!(request.args, Some("arg1,arg2"));
    assert_eq!(request.description, Some("Test macro description"));
    assert!(request.disabled);
    assert!(!request.iseval);
    assert_eq!(request.validation, Some("x > 0"));
    assert_eq!(request.errormsg, Some("x must be positive"));
}

#[tokio::test]
async fn test_create_macro_endpoint() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/services/admin/macros"))
        .and(query_param("output_mode", "json"))
        .and(body_string_contains("name=test_macro"))
        .and(body_string_contains("definition=%7C+makeresults"))
        .respond_with(ResponseTemplate::new(201).set_body_string("{}"))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let request = CreateMacroRequest::new("test_macro", "| makeresults");

    let result = endpoints::create_macro(
        &client,
        &mock_server.uri(),
        "test-token",
        &request,
        3,
        None,
        None,
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_update_macro_request_struct() {
    // Test UpdateMacroRequest construction
    let request = UpdateMacroRequest::new("test_macro");
    assert_eq!(request.name, "test_macro");
    assert_eq!(request.definition, None);
    assert_eq!(request.args, None);
    assert_eq!(request.description, None);
    assert_eq!(request.disabled, None);
    assert_eq!(request.iseval, None);
    assert_eq!(request.validation, None);
    assert_eq!(request.errormsg, None);
}

#[tokio::test]
async fn test_update_macro_request_with_fields() {
    let request = UpdateMacroRequest {
        name: "test_macro",
        definition: Some("| makeresults | eval y=2"),
        args: Some("new_arg"),
        description: Some("Updated description"),
        disabled: Some(false),
        iseval: Some(true),
        validation: Some("y > 0"),
        errormsg: Some("y must be positive"),
    };

    assert_eq!(request.name, "test_macro");
    assert_eq!(request.definition, Some("| makeresults | eval y=2"));
    assert_eq!(request.args, Some("new_arg"));
    assert_eq!(request.description, Some("Updated description"));
    assert_eq!(request.disabled, Some(false));
    assert_eq!(request.iseval, Some(true));
    assert_eq!(request.validation, Some("y > 0"));
    assert_eq!(request.errormsg, Some("y must be positive"));
}

#[tokio::test]
async fn test_update_macro_endpoint() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/services/admin/macros/test_macro"))
        .and(query_param("output_mode", "json"))
        .and(body_string_contains("definition=updated"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let request = UpdateMacroRequest {
        name: "test_macro",
        definition: Some("updated"),
        ..Default::default()
    };

    let result = endpoints::update_macro(
        &client,
        &mock_server.uri(),
        "test-token",
        &request,
        3,
        None,
        None,
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_delete_macro() {
    let mock_server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/services/admin/macros/test_macro"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::delete_macro(
        &client,
        &mock_server.uri(),
        "test-token",
        "test_macro",
        3,
        None,
        None,
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_get_macro() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("macros/get_macro.json");

    Mock::given(method("GET"))
        .and(path("/services/admin/macros/test_macro"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::get_macro(
        &client,
        &mock_server.uri(),
        "test-token",
        "test_macro",
        3,
        None,
        None,
    )
    .await;

    assert!(result.is_ok());
    let macro_def = result.unwrap();
    assert_eq!(macro_def.name, "test_macro");
}

#[tokio::test]
async fn test_get_macro_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/admin/macros/nonexistent"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::get_macro(
        &client,
        &mock_server.uri(),
        "test-token",
        "nonexistent",
        3,
        None,
        None,
    )
    .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("not found"));
}
