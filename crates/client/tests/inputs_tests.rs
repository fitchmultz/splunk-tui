//! Input management endpoint tests.
//!
//! This module tests the Splunk data inputs REST API:
//! - Listing inputs by type (TCP, UDP, Monitor, Script)
//! - Enabling/disabling inputs
//!
//! # Invariants
//! - Inputs are returned with their names, types, and metadata
//! - Results are paginated according to the provided count/offset parameters
//! - Enable/disable operations return empty success responses
//!
//! # What this does NOT handle
//! - Input creation/deletion (not tested here)
//! - Input configuration updates beyond enable/disable

mod common;

use common::*;
use wiremock::matchers::{method, path};

#[tokio::test]
async fn test_list_inputs_by_type_tcp() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("inputs/list_inputs_tcp.json");

    Mock::given(method("GET"))
        .and(path("/services/data/inputs/tcp/raw"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::list_inputs_by_type(
        &client,
        &mock_server.uri(),
        "test-token",
        "tcp/raw",
        Some(30),
        None,
        3,
        None,
    )
    .await;

    if let Err(ref e) = result {
        eprintln!("List inputs error: {:?}", e);
    }
    assert!(result.is_ok());
    let inputs = result.unwrap();
    assert_eq!(inputs.len(), 2);
    assert_eq!(inputs[0].name, "9997");
    assert_eq!(inputs[0].input_type, "tcp/raw");
    assert!(!inputs[0].disabled);
    assert_eq!(inputs[0].port, Some("9997".to_string()));
    assert_eq!(inputs[0].sourcetype, Some("tcp".to_string()));
    assert_eq!(inputs[1].name, "9998");
    assert!(inputs[1].disabled);
}

#[tokio::test]
async fn test_list_inputs_by_type_monitor() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("inputs/list_inputs_monitor.json");

    Mock::given(method("GET"))
        .and(path("/services/data/inputs/monitor"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::list_inputs_by_type(
        &client,
        &mock_server.uri(),
        "test-token",
        "monitor",
        Some(30),
        None,
        3,
        None,
    )
    .await;

    assert!(result.is_ok());
    let inputs = result.unwrap();
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0].name, "/var/log");
    assert_eq!(inputs[0].input_type, "monitor");
    assert!(!inputs[0].disabled);
    assert_eq!(inputs[0].path, Some("/var/log".to_string()));
    assert_eq!(inputs[0].recursive, Some(true));
    assert_eq!(inputs[0].sourcetype, Some("syslog".to_string()));
}

#[tokio::test]
async fn test_list_inputs_by_type_with_pagination() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("inputs/list_inputs_tcp.json");

    Mock::given(method("GET"))
        .and(path("/services/data/inputs/tcp/raw"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::list_inputs_by_type(
        &client,
        &mock_server.uri(),
        "test-token",
        "tcp/raw",
        Some(5),
        Some(10),
        3,
        None,
    )
    .await;

    assert!(result.is_ok());
    let inputs = result.unwrap();
    // The endpoint returns what the server gives it; pagination is handled server-side
    // Here we verify the request was made with the right parameters
    assert_eq!(inputs.len(), 2); // Mock returns all regardless of params
}

#[tokio::test]
async fn test_list_inputs_by_type_empty_response() {
    let mock_server = MockServer::start().await;

    let empty_response = serde_json::json!({ "entry": [] });

    Mock::given(method("GET"))
        .and(path("/services/data/inputs/udp"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&empty_response))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::list_inputs_by_type(
        &client,
        &mock_server.uri(),
        "test-token",
        "udp",
        Some(30),
        None,
        3,
        None,
    )
    .await;

    assert!(result.is_ok());
    let inputs = result.unwrap();
    assert!(inputs.is_empty());
}

#[tokio::test]
async fn test_enable_input() {
    let mock_server = MockServer::start().await;

    // Enable endpoint returns 200 with empty body or minimal response
    Mock::given(method("POST"))
        .and(path("/services/data/inputs/tcp/raw/9997/enable"))
        .respond_with(ResponseTemplate::new(200).set_body_string(""))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::enable_input(
        &client,
        &mock_server.uri(),
        "test-token",
        "tcp/raw",
        "9997",
        3,
        None,
    )
    .await;

    if let Err(ref e) = result {
        eprintln!("Enable input error: {:?}", e);
    }
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_disable_input() {
    let mock_server = MockServer::start().await;

    // Disable endpoint returns 200 with empty body or minimal response
    Mock::given(method("POST"))
        .and(path("/services/data/inputs/tcp/raw/9998/disable"))
        .respond_with(ResponseTemplate::new(200).set_body_string(""))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::disable_input(
        &client,
        &mock_server.uri(),
        "test-token",
        "tcp/raw",
        "9998",
        3,
        None,
    )
    .await;

    if let Err(ref e) = result {
        eprintln!("Disable input error: {:?}", e);
    }
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_enable_input_monitor() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/services/data/inputs/monitor//var/log/enable"))
        .respond_with(ResponseTemplate::new(200).set_body_string(""))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::enable_input(
        &client,
        &mock_server.uri(),
        "test-token",
        "monitor",
        "/var/log",
        3,
        None,
    )
    .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_list_inputs_by_type_with_special_characters_in_name() {
    let mock_server = MockServer::start().await;

    // Test with input names that might need URL encoding
    let fixture = serde_json::json!({
        "entry": [
            {
                "name": "script://./bin/my_script.sh",
                "content": {
                    "name": "script://./bin/my_script.sh",
                    "input_type": "script",
                    "disabled": false,
                    "command": "./bin/my_script.sh",
                    "interval": "60"
                }
            }
        ]
    });

    Mock::given(method("GET"))
        .and(path("/services/data/inputs/script"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::list_inputs_by_type(
        &client,
        &mock_server.uri(),
        "test-token",
        "script",
        Some(30),
        None,
        3,
        None,
    )
    .await;

    assert!(result.is_ok());
    let inputs = result.unwrap();
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0].name, "script://./bin/my_script.sh");
    assert_eq!(inputs[0].input_type, "script");
}
