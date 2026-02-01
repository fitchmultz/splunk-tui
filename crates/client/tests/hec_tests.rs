//! HEC (HTTP Event Collector) endpoint tests.
//!
//! This module tests the Splunk HEC API:
//! - Sending single events
//! - Sending batch events (JSON array and NDJSON formats)
//! - Health check endpoint
//! - Acknowledgment status checks
//!
//! # Invariants
//! - HEC uses "Splunk" auth prefix (not "Bearer")
//! - HEC endpoints are separate from REST API (typically port 8088)
//! - Events are returned with acknowledgment IDs when acks are enabled

mod common;

use common::*;
use serde_json::json;
use splunk_client::models::hec::{HecAckStatus, HecEvent, HecHealth, HecResponse};
use std::collections::HashMap;
use wiremock::matchers::{header, method, path};

#[tokio::test]
async fn test_send_single_event() {
    let mock_server = MockServer::start().await;

    let fixture = json!({
        "code": 0,
        "text": "Success",
        "ack_id": 123
    });

    Mock::given(method("POST"))
        .and(path("/services/collector/event"))
        .and(header("Authorization", "Splunk test-hec-token"))
        .and(header("Content-Type", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let event = HecEvent::new(json!({"message": "Test event"}))
        .with_index("main")
        .with_source("test");

    let result = endpoints::hec::send_event(
        &client,
        &mock_server.uri(),
        "test-hec-token",
        &event,
        3,
        None,
    )
    .await;

    if let Err(ref e) = result {
        eprintln!("Send event error: {:?}", e);
    }
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.code, 0);
    assert_eq!(response.text, "Success");
    assert_eq!(response.ack_id, Some(123));
}

#[tokio::test]
async fn test_send_batch_json_array() {
    let mock_server = MockServer::start().await;

    let fixture = json!({
        "code": 0,
        "text": "Success",
        "ack_ids": [1, 2, 3]
    });

    Mock::given(method("POST"))
        .and(path("/services/collector/event"))
        .and(header("Authorization", "Splunk test-hec-token"))
        .and(header("Content-Type", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let events = vec![
        HecEvent::new(json!({"message": "Event 1"})),
        HecEvent::new(json!({"message": "Event 2"})),
        HecEvent::new(json!({"message": "Event 3"})),
    ];

    let result = endpoints::hec::send_batch(
        &client,
        &mock_server.uri(),
        "test-hec-token",
        &events,
        false, // JSON array format
        3,
        None,
    )
    .await;

    if let Err(ref e) = result {
        eprintln!("Send batch error: {:?}", e);
    }
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.code, 0);
    assert_eq!(response.ack_ids, Some(vec![1, 2, 3]));
}

#[tokio::test]
async fn test_send_batch_ndjson() {
    let mock_server = MockServer::start().await;

    let fixture = json!({
        "code": 0,
        "text": "Success",
        "ack_ids": [1, 2]
    });

    Mock::given(method("POST"))
        .and(path("/services/collector/event"))
        .and(header("Authorization", "Splunk test-hec-token"))
        .and(header("Content-Type", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let events = vec![
        HecEvent::new(json!({"message": "Event 1"})),
        HecEvent::new(json!({"message": "Event 2"})),
    ];

    let result = endpoints::hec::send_batch(
        &client,
        &mock_server.uri(),
        "test-hec-token",
        &events,
        true, // NDJSON format
        3,
        None,
    )
    .await;

    if let Err(ref e) = result {
        eprintln!("Send batch NDJSON error: {:?}", e);
    }
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.code, 0);
}

#[tokio::test]
async fn test_hec_health_check() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/collector/health"))
        .and(header("Authorization", "Splunk test-hec-token"))
        .respond_with(ResponseTemplate::new(200).set_body_string("HEC is healthy"))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result =
        endpoints::hec::health_check(&client, &mock_server.uri(), "test-hec-token", 3, None).await;

    if let Err(ref e) = result {
        eprintln!("Health check error: {:?}", e);
    }
    assert!(result.is_ok());
    let health = result.unwrap();
    assert_eq!(health.code, 200);
    assert_eq!(health.text, "HEC is healthy");
    assert!(health.is_healthy());
}

#[tokio::test]
async fn test_hec_check_acks() {
    let mock_server = MockServer::start().await;

    let mut acks = HashMap::new();
    acks.insert(1, true);
    acks.insert(2, false);
    acks.insert(3, true);

    let fixture = json!({
        "acks": acks
    });

    Mock::given(method("POST"))
        .and(path("/services/collector/ack"))
        .and(header("Authorization", "Splunk test-hec-token"))
        .and(header("Content-Type", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let ack_ids = vec![1, 2, 3];

    let result = endpoints::hec::check_ack_status(
        &client,
        &mock_server.uri(),
        "test-hec-token",
        &ack_ids,
        3,
        None,
    )
    .await;

    if let Err(ref e) = result {
        eprintln!("Check acks error: {:?}", e);
    }
    assert!(result.is_ok());
    let status = result.unwrap();
    assert!(!status.all_indexed());
    assert_eq!(status.pending_ids(), vec![2]);
    let mut indexed = status.indexed_ids();
    indexed.sort();
    assert_eq!(indexed, vec![1, 3]);
}

#[tokio::test]
async fn test_hec_auth_header_format() {
    let mock_server = MockServer::start().await;

    // Verify that the "Splunk" prefix is used (not "Bearer")
    Mock::given(method("POST"))
        .and(path("/services/collector/event"))
        .and(header("Authorization", "Splunk test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "code": 0,
            "text": "Success"
        })))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let event = HecEvent::new(json!({"test": "data"}));

    let result =
        endpoints::hec::send_event(&client, &mock_server.uri(), "test-token", &event, 3, None)
            .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_hec_error_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/services/collector/event"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({
            "code": 2,
            "text": "Invalid token"
        })))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let event = HecEvent::new(json!({"test": "data"}));

    let result = endpoints::hec::send_event(
        &client,
        &mock_server.uri(),
        "invalid-token",
        &event,
        3,
        None,
    )
    .await;

    // Should return an error for 401 status
    assert!(result.is_err());
}

#[tokio::test]
async fn test_hec_event_builder() {
    let event = HecEvent::new(json!({"message": "test"}))
        .with_index("main")
        .with_source("myapp")
        .with_sourcetype("json")
        .with_host("server01")
        .with_time(1234567890.123);

    assert_eq!(event.event, json!({"message": "test"}));
    assert_eq!(event.index, Some("main".to_string()));
    assert_eq!(event.source, Some("myapp".to_string()));
    assert_eq!(event.sourcetype, Some("json".to_string()));
    assert_eq!(event.host, Some("server01".to_string()));
    assert_eq!(event.time, Some(1234567890.123));
}

#[tokio::test]
async fn test_hec_response_methods() {
    let success = HecResponse {
        code: 0,
        text: "Success".to_string(),
        ack_id: Some(123),
    };
    assert!(success.is_success());
    assert_eq!(success.description(), "Success");

    let error = HecResponse {
        code: 2,
        text: "Invalid token".to_string(),
        ack_id: None,
    };
    assert!(!error.is_success());
    assert_eq!(error.description(), "Invalid token");

    let unknown = HecResponse {
        code: 999,
        text: "Unknown".to_string(),
        ack_id: None,
    };
    assert_eq!(unknown.description(), "Unknown error code: 999");
}

#[tokio::test]
async fn test_hec_health_methods() {
    let healthy = HecHealth {
        text: "HEC is healthy".to_string(),
        code: 200,
    };
    assert!(healthy.is_healthy());

    let unhealthy = HecHealth {
        text: "HEC is not healthy".to_string(),
        code: 503,
    };
    assert!(!unhealthy.is_healthy());

    let not_healthy_text = HecHealth {
        text: "Something else".to_string(),
        code: 200,
    };
    assert!(!not_healthy_text.is_healthy());
}

#[tokio::test]
async fn test_hec_ack_status_methods() {
    let mut acks = HashMap::new();
    acks.insert(1, true);
    acks.insert(2, false);
    acks.insert(3, true);

    let status = HecAckStatus { acks };

    assert!(!status.all_indexed());
    assert_eq!(status.pending_ids(), vec![2]);
    let mut indexed = status.indexed_ids();
    indexed.sort();
    assert_eq!(indexed, vec![1, 3]);

    let mut all_acks = HashMap::new();
    all_acks.insert(1, true);
    all_acks.insert(2, true);

    let all_indexed = HecAckStatus { acks: all_acks };
    assert!(all_indexed.all_indexed());
    assert!(all_indexed.pending_ids().is_empty());
}
