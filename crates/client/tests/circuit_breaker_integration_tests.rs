//! Integration tests for circuit breaker behavior.

mod common;

use common::*;
use splunk_client::client::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use std::time::Duration;
use wiremock::matchers::{method, path};

#[tokio::test]
async fn test_circuit_breaker_fast_fail() {
    let mock_server = MockServer::start().await;

    // Configure circuit breaker to open after 2 failures
    let config = CircuitBreakerConfig {
        failure_threshold: 2,
        failure_window: Duration::from_secs(60),
        reset_timeout: Duration::from_secs(30),
        half_open_requests: 1,
    };
    let cb = CircuitBreaker::new().with_default_config(config);

    // Mock server returns 503 Service Unavailable
    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let uri = mock_server.uri();

    // First failure
    let result = endpoints::get_server_info(&client, &uri, "token", 1, None, Some(&cb)).await;
    assert!(result.is_err());
    assert!(!matches!(
        result.unwrap_err(),
        splunk_client::ClientError::CircuitBreakerOpen(_)
    ));

    // Second failure - should open the circuit
    let result = endpoints::get_server_info(&client, &uri, "token", 1, None, Some(&cb)).await;
    assert!(result.is_err());

    // Third call - should fail fast with CircuitBreakerOpen
    let result = endpoints::get_server_info(&client, &uri, "token", 1, None, Some(&cb)).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        splunk_client::ClientError::CircuitBreakerOpen(_)
    ));
}

#[tokio::test]
async fn test_circuit_breaker_recovery() {
    let mock_server = MockServer::start().await;

    // Configure circuit breaker to open after 1 failure and reset quickly
    let config = CircuitBreakerConfig {
        failure_threshold: 1,
        failure_window: Duration::from_secs(60),
        reset_timeout: Duration::from_millis(100),
        half_open_requests: 1,
    };
    let cb = CircuitBreaker::new().with_default_config(config);

    let client = Client::new();
    let uri = mock_server.uri();

    // Mock failure
    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    // Trigger failure and open circuit
    // Use 0 retries to avoid race condition with reset_timeout during retry sleep
    let _ = endpoints::get_server_info(&client, &uri, "token", 0, None, Some(&cb)).await;
    assert!(matches!(
        cb.state("/services/server/info"),
        splunk_client::client::circuit_breaker::CircuitState::Open
    ));

    // Wait for reset timeout
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Mock success
    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entry": [{"content": {
                "version": "9.0.0",
                "serverName": "test",
                "build": "1",
                "cpu_arch": "x86_64",
                "osName": "Linux",
                "guid": "1"
            }}]
        })))
        .mount(&mock_server)
        .await;

    // Call should proceed in half-open state and then close the circuit on success
    let result = endpoints::get_server_info(&client, &uri, "token", 1, None, Some(&cb)).await;
    assert!(result.is_ok());
    assert!(matches!(
        cb.state("/services/server/info"),
        splunk_client::client::circuit_breaker::CircuitState::Closed
    ));
}

#[tokio::test]
async fn test_expected_shc_503_does_not_open_circuit_or_retry() {
    let mock_server = MockServer::start().await;

    // Open quickly if failures are incorrectly recorded.
    let config = CircuitBreakerConfig {
        failure_threshold: 1,
        failure_window: Duration::from_secs(60),
        reset_timeout: Duration::from_secs(30),
        half_open_requests: 1,
    };
    let cb = CircuitBreaker::new().with_default_config(config);

    // Standalone/non-SHC instances can return a 503 for SHC endpoints.
    // This should not be retried and should not count as circuit-breaker failure.
    Mock::given(method("GET"))
        .and(path("/services/shcluster/member/info"))
        .respond_with(ResponseTemplate::new(503).set_body_json(serde_json::json!({
            "messages": [{
                "type": "ERROR",
                "text": "Service temporarily unavailable"
            }]
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let uri = mock_server.uri();

    let result = endpoints::get_shc_status(&client, &uri, "token", 3, None, Some(&cb)).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        splunk_client::ClientError::ApiError { status: 503, .. }
    ));

    assert!(matches!(
        cb.state("/services/shcluster/member/info"),
        splunk_client::client::circuit_breaker::CircuitState::Closed
    ));
}
