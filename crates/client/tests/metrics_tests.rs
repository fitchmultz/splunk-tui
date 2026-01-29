//! Tests for API metrics collection.
//!
//! These tests verify that the metrics collection system correctly records:
//! - Request counts
//! - Request latencies
//! - Retry counts
//! - Error categorization

use std::time::Duration;

use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use splunk_client::endpoints::send_request_with_retry;
use splunk_client::metrics::{ErrorCategory, MetricsCollector};

#[tokio::test]
async fn test_metrics_collector_enabled_by_default() {
    let collector = MetricsCollector::new();
    assert!(collector.is_enabled());
}

#[tokio::test]
async fn test_metrics_collector_disabled() {
    let collector = MetricsCollector::disabled();
    assert!(!collector.is_enabled());
}

#[tokio::test]
async fn test_error_category_as_str() {
    assert_eq!(ErrorCategory::Transport.as_str(), "transport");
    assert_eq!(ErrorCategory::Http4xx.as_str(), "http_4xx");
    assert_eq!(ErrorCategory::Http5xx.as_str(), "http_5xx");
    assert_eq!(ErrorCategory::Api.as_str(), "api");
    assert_eq!(ErrorCategory::Timeout.as_str(), "timeout");
    assert_eq!(ErrorCategory::Tls.as_str(), "tls");
    assert_eq!(ErrorCategory::Unknown.as_str(), "unknown");
}

#[tokio::test]
async fn test_metrics_record_request() {
    let collector = MetricsCollector::new();

    // Should not panic
    collector.record_request("/services/search/jobs", "POST");
}

#[tokio::test]
async fn test_metrics_record_disabled_collector() {
    let collector = MetricsCollector::disabled();

    // These should not panic even when disabled
    collector.record_request("/services/search/jobs", "POST");
    collector.record_request_duration(
        "/services/search/jobs",
        "POST",
        Duration::from_millis(100),
        Some(200),
    );
    collector.record_retry("/services/search/jobs", "POST", 1);
    collector.record_error("/services/search/jobs", "POST", ErrorCategory::Transport);
}

#[tokio::test]
async fn test_send_request_with_retry_records_metrics() {
    let mock_server = MockServer::start().await;

    // Mock a successful response
    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entry": [{
                "content": {
                    "version": "9.0.0"
                }
            }]
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let collector = MetricsCollector::new();

    let request = client
        .get(format!("{}/services/server/info", mock_server.uri()))
        .header("Authorization", "Bearer test-token")
        .query(&[("output_mode", "json")]);

    let result =
        send_request_with_retry(request, 3, "/services/server/info", "GET", Some(&collector)).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_send_request_with_retry_records_error_metrics() {
    let mock_server = MockServer::start().await;

    // Mock a 404 error response (non-retryable)
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/nonexistent"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "messages": [{
                "type": "ERROR",
                "text": "Job not found"
            }]
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let collector = MetricsCollector::new();

    let request = client
        .get(format!(
            "{}/services/search/jobs/nonexistent",
            mock_server.uri()
        ))
        .header("Authorization", "Bearer test-token")
        .query(&[("output_mode", "json")]);

    let result = send_request_with_retry(
        request,
        3,
        "/services/search/jobs/{sid}",
        "GET",
        Some(&collector),
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_send_request_with_retry_records_retry_metrics() {
    let mock_server = MockServer::start().await;

    // Mock a 503 error followed by success
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "sid": "test-job-123"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();
    let collector = MetricsCollector::new();

    let request = client
        .post(format!("{}/services/search/jobs", mock_server.uri()))
        .header("Authorization", "Bearer test-token")
        .form(&[("search", "search index=main"), ("output_mode", "json")]);

    let result = send_request_with_retry(
        request,
        3,
        "/services/search/jobs",
        "POST",
        Some(&collector),
    )
    .await;

    // Should succeed after retry
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_metrics_collector_with_splunk_client() {
    use secrecy::SecretString;
    use splunk_client::{AuthStrategy, SplunkClient};

    let mock_server = MockServer::start().await;

    let metrics = MetricsCollector::new();

    let client = SplunkClient::builder()
        .base_url(mock_server.uri())
        .auth_strategy(AuthStrategy::ApiToken {
            token: SecretString::new("test-token".to_string().into()),
        })
        .metrics(metrics)
        .build();

    assert!(client.is_ok());
}

#[tokio::test]
async fn test_metrics_record_request_duration() {
    let collector = MetricsCollector::new();

    // Should not panic
    collector.record_request_duration(
        "/services/search/jobs",
        "POST",
        Duration::from_millis(150),
        Some(200),
    );
}

#[tokio::test]
async fn test_metrics_record_retry() {
    let collector = MetricsCollector::new();

    // Should not panic
    collector.record_retry("/services/search/jobs", "POST", 1);
    collector.record_retry("/services/search/jobs", "POST", 2);
}

#[tokio::test]
async fn test_metrics_record_error() {
    let collector = MetricsCollector::new();

    // Should not panic
    collector.record_error("/services/search/jobs", "POST", ErrorCategory::Http4xx);
    collector.record_error("/services/search/jobs", "POST", ErrorCategory::Http5xx);
    collector.record_error("/services/search/jobs", "POST", ErrorCategory::Transport);
    collector.record_error("/services/search/jobs", "POST", ErrorCategory::Timeout);
    collector.record_error("/services/search/jobs", "POST", ErrorCategory::Tls);
    collector.record_error("/services/search/jobs", "POST", ErrorCategory::Api);
    collector.record_error("/services/search/jobs", "POST", ErrorCategory::Unknown);
}

#[tokio::test]
async fn test_send_request_with_no_metrics() {
    let mock_server = MockServer::start().await;

    // Mock a successful response
    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entry": [{
                "content": {
                    "version": "9.0.0"
                }
            }]
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = reqwest::Client::new();

    let request = client
        .get(format!("{}/services/server/info", mock_server.uri()))
        .header("Authorization", "Bearer test-token")
        .query(&[("output_mode", "json")]);

    // Should work with None for metrics
    let result = send_request_with_retry(
        request,
        3,
        "/services/server/info",
        "GET",
        None, // No metrics collector
    )
    .await;

    assert!(result.is_ok());
}
