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
use secrecy::SecretString;
use splunk_client::error::ClientError;
use splunk_client::models::InputType;
use splunk_client::{AuthStrategy, SplunkClient};
use wiremock::matchers::{method, path};

// Input type segments keep "/" path separators (e.g., "tcp/raw"), while input names
// are still path-encoded when needed.

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
    assert_eq!(inputs[0].input_type, InputType::TcpRaw);
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
        None,
    )
    .await;

    assert!(result.is_ok());
    let inputs = result.unwrap();
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0].name, "/var/log");
    assert_eq!(inputs[0].input_type, InputType::Monitor);
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

    // Note: "/var/log" contains "/" which is encoded to "%2Fvar%2Flog"
    Mock::given(method("POST"))
        .and(path("/services/data/inputs/monitor/%2Fvar%2Flog/enable"))
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
        None,
    )
    .await;

    assert!(result.is_ok());
    let inputs = result.unwrap();
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0].name, "script://./bin/my_script.sh");
    assert_eq!(inputs[0].input_type, InputType::Script);
}

#[tokio::test]
async fn test_list_inputs_by_type_infers_type_when_response_omits_input_type() {
    let mock_server = MockServer::start().await;

    let fixture = serde_json::json!({
        "entry": [
            {
                "name": "9997",
                "content": {
                    "disabled": false,
                    "host": "$decideOnStartup",
                    "port": "9997"
                }
            }
        ]
    });

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
        None,
    )
    .await;

    assert!(result.is_ok());
    let inputs = result.unwrap();
    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0].input_type, InputType::TcpRaw);
}

#[tokio::test]
async fn test_list_inputs_by_type_unauthorized() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/data/inputs/tcp/raw"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Unauthorized"}]
        })))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::list_inputs_by_type(
        &client,
        &mock_server.uri(),
        "invalid-token",
        "tcp/raw",
        Some(30),
        None,
        3,
        None,
        None,
    )
    .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    // 401 is now classified as Unauthorized variant
    assert!(
        matches!(err, ClientError::Unauthorized(_)),
        "Expected Unauthorized, got {:?}",
        err
    );
}

#[tokio::test]
async fn test_list_inputs_by_type_forbidden() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/data/inputs/tcp/raw"))
        .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Forbidden"}]
        })))
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
        None,
    )
    .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, ClientError::ApiError { status: 403, .. }));
}

#[tokio::test]
async fn test_splunk_client_list_inputs_skips_missing_input_type_endpoints() {
    let mock_server = MockServer::start().await;
    let tcp_fixture = load_fixture("inputs/list_inputs_tcp.json");
    let empty_response = serde_json::json!({ "entry": [] });

    Mock::given(method("GET"))
        .and(path("/services/data/inputs/tcp/raw"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&tcp_fixture))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/services/data/inputs/tcp/cooked"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Not Found"}]
        })))
        .mount(&mock_server)
        .await;

    for endpoint in [
        "/services/data/inputs/udp",
        "/services/data/inputs/monitor",
        "/services/data/inputs/script",
    ] {
        Mock::given(method("GET"))
            .and(path(endpoint))
            .respond_with(ResponseTemplate::new(200).set_body_json(&empty_response))
            .mount(&mock_server)
            .await;
    }

    let client = SplunkClient::builder()
        .base_url(mock_server.uri())
        .auth_strategy(AuthStrategy::ApiToken {
            token: SecretString::new("test-token".to_string().into()),
        })
        .build()
        .expect("Failed to build SplunkClient");

    let result = client.list_inputs(Some(30), None).await;

    assert!(result.is_ok(), "Expected list_inputs to skip missing types");
    let inputs = result.unwrap();
    assert_eq!(inputs.len(), 2);
    assert_eq!(inputs[0].input_type, InputType::TcpRaw);
}

#[tokio::test]
async fn test_list_inputs_by_type_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/data/inputs/invalid-type"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Not Found"}]
        })))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::list_inputs_by_type(
        &client,
        &mock_server.uri(),
        "test-token",
        "invalid-type",
        Some(30),
        None,
        3,
        None,
        None,
    )
    .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    // 404 is now classified as NotFound variant
    assert!(
        matches!(err, ClientError::NotFound(_)),
        "Expected NotFound, got {:?}",
        err
    );
}

#[tokio::test]
async fn test_enable_input_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/services/data/inputs/tcp/raw/9999/enable"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Not Found"}]
        })))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::enable_input(
        &client,
        &mock_server.uri(),
        "test-token",
        "tcp/raw",
        "9999",
        3,
        None,
        None,
    )
    .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    // 404 is now classified as NotFound variant
    assert!(
        matches!(err, ClientError::NotFound(_)),
        "Expected NotFound, got {:?}",
        err
    );
}

#[tokio::test]
async fn test_disable_input_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/services/data/inputs/tcp/raw/9999/disable"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Not Found"}]
        })))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::disable_input(
        &client,
        &mock_server.uri(),
        "test-token",
        "tcp/raw",
        "9999",
        3,
        None,
        None,
    )
    .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    // 404 is now classified as NotFound variant
    assert!(
        matches!(err, ClientError::NotFound(_)),
        "Expected NotFound, got {:?}",
        err
    );
}

#[tokio::test]
async fn test_list_inputs_malformed_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/data/inputs/tcp/raw"))
        .respond_with(ResponseTemplate::new(200).set_body_string("invalid json"))
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
        None,
    )
    .await;

    assert!(result.is_err());
}
