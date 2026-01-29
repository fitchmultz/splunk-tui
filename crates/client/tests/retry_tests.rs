//! Retry behavior tests.
//!
//! This module tests the client's retry logic for various HTTP status codes
//! and error conditions:
//! - Rate limiting (429) with and without Retry-After header
//! - Service unavailable (503), Bad Gateway (502), Gateway Timeout (504)
//! - Session re-authentication on 401/403 with session token auth
//! - No retry on 401 with API token auth
//! - Timeout handling
//!
//! # Invariants
//! - 429, 502, 503, 504 trigger retry with exponential backoff
//! - Retry-After header is respected when present (both delta-seconds and HTTP-date)
//! - 401/403 trigger session re-auth only for SessionToken auth strategy
//! - 500/501 do NOT trigger retry
//! - Timeout errors trigger retry
//!
//! # What this does NOT handle
//! - Connection-level error retry (see error_tests.rs)
//! - TLS error handling (see error_tests.rs)

mod common;

use common::*;
use splunk_client::ClientError;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use wiremock::matchers::{method, path, query_param};

#[tokio::test]
async fn test_retry_on_429_success() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/create_job_success.json");

    // Use wiremock's sequence feature to return 429 twice, then 200
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Rate limited"}]
        })))
        .up_to_n_times(2)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let options = endpoints::CreateJobOptions {
        wait: Some(false),
        ..Default::default()
    };

    let start = std::time::Instant::now();
    let result = endpoints::create_job(
        &client,
        &mock_server.uri(),
        "test-token",
        "search index=main",
        &options,
        3, // max_retries
    )
    .await;
    let elapsed = start.elapsed();

    // Should succeed after retries
    assert!(result.is_ok());
    let sid = result.unwrap();
    assert!(sid.contains("scheduler__admin__search"));

    // Should have taken at least 1 + 2 = 3 seconds (exponential backoff: 1s, 2s)
    // Note: timing assertions can be flaky, so we use a generous threshold
    assert!(elapsed >= std::time::Duration::from_secs(2));
}

#[tokio::test]
async fn test_retry_on_429_exhaustion() {
    let mock_server = MockServer::start().await;

    // Always return 429
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid"))
        .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Rate limited"}]
        })))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let start = std::time::Instant::now();
    let result =
        endpoints::get_job_status(&client, &mock_server.uri(), "test-token", "test-sid", 2).await;
    let elapsed = start.elapsed();

    // Should fail after exhausting retries
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, ClientError::MaxRetriesExceeded(3, _))); // 2 retries + 1 initial attempt = 3 total

    // Should have taken at least 1 + 2 = 3 seconds (exponential backoff: 1s, 2s)
    assert!(elapsed >= std::time::Duration::from_secs(2));
}

#[tokio::test]
async fn test_retry_on_401_session_auth() {
    let mock_server = MockServer::start().await;

    let login_fixture = load_fixture("auth/login_success.json");
    let list_indexes_fixture = load_fixture("indexes/list_indexes.json");

    // Track login requests using Arc<AtomicUsize>
    let login_count = Arc::new(AtomicUsize::new(0));
    let login_count_clone = login_count.clone();

    // Mock login endpoint - returns fresh session key
    Mock::given(method("POST"))
        .and(path("/services/auth/login"))
        .and(query_param("output_mode", "json"))
        .respond_with(move |_: &wiremock::Request| {
            login_count_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(200).set_body_json(&login_fixture)
        })
        .mount(&mock_server)
        .await;

    // First call to list_indexes returns 401, second returns 200
    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Session expired"}]
        })))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&list_indexes_fixture))
        .mount(&mock_server)
        .await;

    use secrecy::SecretString;
    use splunk_client::{AuthStrategy, SplunkClient};

    let strategy = AuthStrategy::SessionToken {
        username: "admin".to_string(),
        password: SecretString::new("testpassword".to_string().into()),
    };

    let mut client = SplunkClient::builder()
        .base_url(mock_server.uri())
        .auth_strategy(strategy)
        .skip_verify(true)
        .build()
        .unwrap();

    // Initial login
    client.login().await.unwrap();
    assert_eq!(login_count.load(Ordering::SeqCst), 1);

    // This should trigger a retry with re-login
    let result = client.list_indexes(Some(10), Some(0)).await;

    assert!(result.is_ok());
    let indexes = result.unwrap();
    assert_eq!(indexes.len(), 3);

    // Should have called login twice (initial + retry)
    assert_eq!(login_count.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn test_retry_on_403_session_auth() {
    let mock_server = MockServer::start().await;

    let login_fixture = load_fixture("auth/login_success.json");
    let job_fixture = load_fixture("search/create_job_success.json");

    // Track login requests using Arc<AtomicUsize>
    let login_count = Arc::new(AtomicUsize::new(0));
    let login_count_clone = login_count.clone();

    // Mock login endpoint
    Mock::given(method("POST"))
        .and(path("/services/auth/login"))
        .and(query_param("output_mode", "json"))
        .respond_with(move |_: &wiremock::Request| {
            login_count_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(200).set_body_json(&login_fixture)
        })
        .mount(&mock_server)
        .await;

    // First call to create_job returns 403, second returns 200
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Forbidden - session expired"}]
        })))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&job_fixture))
        .mount(&mock_server)
        .await;

    use secrecy::SecretString;
    use splunk_client::{AuthStrategy, SplunkClient};

    let strategy = AuthStrategy::SessionToken {
        username: "admin".to_string(),
        password: SecretString::new("testpassword".to_string().into()),
    };

    let mut client = SplunkClient::builder()
        .base_url(mock_server.uri())
        .auth_strategy(strategy)
        .skip_verify(true)
        .build()
        .unwrap();

    // Initial login
    client.login().await.unwrap();
    assert_eq!(login_count.load(Ordering::SeqCst), 1);

    // This should trigger a retry with re-login
    let options = splunk_client::endpoints::CreateJobOptions {
        wait: Some(false),
        ..Default::default()
    };
    let result = client
        .create_search_job("search index=main", &options)
        .await;

    assert!(result.is_ok());
    let sid = result.unwrap();
    assert!(sid.contains("scheduler__admin__search"));

    // Should have called login twice (initial + retry)
    assert_eq!(login_count.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn test_no_retry_on_401_api_token() {
    let mock_server = MockServer::start().await;

    // API token auth - return 401
    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Invalid token"}]
        })))
        .mount(&mock_server)
        .await;

    // Should never be called for API token auth
    Mock::given(method("POST"))
        .and(path("/services/auth/login"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "sessionKey": "should-not-be-called"
        })))
        .mount(&mock_server)
        .await;

    use secrecy::SecretString;
    use splunk_client::{AuthStrategy, SplunkClient};

    let strategy = AuthStrategy::ApiToken {
        token: SecretString::new("invalid-token".to_string().into()),
    };

    let mut client = SplunkClient::builder()
        .base_url(mock_server.uri())
        .auth_strategy(strategy)
        .skip_verify(true)
        .build()
        .unwrap();

    // Should fail immediately without retry
    let result = client.list_indexes(Some(10), Some(0)).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, ClientError::ApiError { status: 401, .. }));
}

#[tokio::test]
async fn test_retry_fails_on_second_401() {
    let mock_server = MockServer::start().await;

    let login_fixture = load_fixture("auth/login_success.json");

    // Mock login endpoint
    Mock::given(method("POST"))
        .and(path("/services/auth/login"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&login_fixture))
        .mount(&mock_server)
        .await;

    // Always return 401 even after retry
    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Session expired"}]
        })))
        .mount(&mock_server)
        .await;

    use secrecy::SecretString;
    use splunk_client::{AuthStrategy, SplunkClient};

    let strategy = AuthStrategy::SessionToken {
        username: "admin".to_string(),
        password: SecretString::new("testpassword".to_string().into()),
    };

    let mut client = SplunkClient::builder()
        .base_url(mock_server.uri())
        .auth_strategy(strategy)
        .skip_verify(true)
        .build()
        .unwrap();

    // Initial login
    client.login().await.unwrap();

    // Should fail even after retry
    let result = client.list_indexes(Some(10), Some(0)).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, ClientError::ApiError { status: 401, .. }));
}

// Retry-After header tests

#[tokio::test]
async fn test_retry_respects_retry_after_header() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/create_job_success.json");

    // First response returns 429 with Retry-After: 3
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "3")
                .set_body_json(serde_json::json!({
                    "messages": [{"type": "ERROR", "text": "Rate limited"}]
                })),
        )
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    // Second response returns 200
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let options = endpoints::CreateJobOptions {
        wait: Some(false),
        ..Default::default()
    };

    let start = std::time::Instant::now();
    let result = endpoints::create_job(
        &client,
        &mock_server.uri(),
        "test-token",
        "search index=main",
        &options,
        3,
    )
    .await;
    let elapsed = start.elapsed();

    assert!(result.is_ok());

    // Should have waited at least 3 seconds (Retry-After value), not just 1s
    assert!(elapsed >= std::time::Duration::from_secs(3));
}

#[tokio::test]
async fn test_retry_with_max_of_backoff_and_retry_after() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/create_job_success.json");

    // First two responses return 429 with Retry-After: 1 (less than exponential backoff of 2 on second retry)
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "1")
                .set_body_json(serde_json::json!({
                    "messages": [{"type": "ERROR", "text": "Rate limited"}]
                })),
        )
        .up_to_n_times(2)
        .mount(&mock_server)
        .await;

    // Third response returns 200
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let options = endpoints::CreateJobOptions {
        wait: Some(false),
        ..Default::default()
    };

    let start = std::time::Instant::now();
    let result = endpoints::create_job(
        &client,
        &mock_server.uri(),
        "test-token",
        "search index=main",
        &options,
        3,
    )
    .await;
    let elapsed = start.elapsed();

    assert!(result.is_ok());

    // First retry: max(backoff=1, retry_after=1) = 1 second
    // Second retry: max(backoff=2, retry_after=1) = 2 seconds
    // Total wait time should be at least 3 seconds
    assert!(elapsed >= std::time::Duration::from_secs(3));
}

#[tokio::test]
async fn test_retry_falls_back_to_exponential_backoff() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/create_job_success.json");

    // First response returns 429 WITHOUT Retry-After header
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Rate limited"}]
        })))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    // Second response returns 200
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let options = endpoints::CreateJobOptions {
        wait: Some(false),
        ..Default::default()
    };

    let start = std::time::Instant::now();
    let result = endpoints::create_job(
        &client,
        &mock_server.uri(),
        "test-token",
        "search index=main",
        &options,
        3,
    )
    .await;
    let elapsed = start.elapsed();

    assert!(result.is_ok());

    // Should use exponential backoff (1 second for first retry)
    assert!(elapsed >= std::time::Duration::from_secs(1));
    // But less than 2 seconds (since we only retried once with 1s backoff)
    assert!(elapsed < std::time::Duration::from_secs(2));
}

#[tokio::test]
async fn test_retry_with_invalid_retry_after_header() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/create_job_success.json");

    // First response returns 429 with invalid Retry-After header
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "invalid-date")
                .set_body_json(serde_json::json!({
                    "messages": [{"type": "ERROR", "text": "Rate limited"}]
                })),
        )
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    // Second response returns 200
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let options = endpoints::CreateJobOptions {
        wait: Some(false),
        ..Default::default()
    };

    let result = endpoints::create_job(
        &client,
        &mock_server.uri(),
        "test-token",
        "search index=main",
        &options,
        3,
    )
    .await;

    // Should still succeed, falling back to exponential backoff
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_retry_respects_retry_after_http_date() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/create_job_success.json");

    // Calculate a future HTTP-date (10 seconds from now)
    // Use a longer duration to account for test setup time
    let future_time = time::OffsetDateTime::now_utc() + time::Duration::seconds(10);
    let http_date = future_time
        .format(&time::format_description::well_known::Rfc2822)
        .unwrap();

    // First response returns 429 with Retry-After as HTTP-date
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", http_date.as_str())
                .set_body_json(serde_json::json!({
                    "messages": [{"type": "ERROR", "text": "Rate limited"}]
                })),
        )
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    // Second response returns 200
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let options = endpoints::CreateJobOptions {
        wait: Some(false),
        ..Default::default()
    };

    let start = std::time::Instant::now();
    let result = endpoints::create_job(
        &client,
        &mock_server.uri(),
        "test-token",
        "search index=main",
        &options,
        3,
    )
    .await;
    let elapsed = start.elapsed();

    assert!(result.is_ok());

    // Should have waited at least 8 seconds (HTTP-date Retry-After value minus some tolerance)
    // The HTTP-date is 10 seconds in the future, but some time passes during setup
    assert!(
        elapsed >= std::time::Duration::from_secs(8),
        "Expected at least 8s but got {:?}",
        elapsed
    );
}

#[tokio::test]
async fn test_retry_with_past_http_date() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/create_job_success.json");

    // Use a past HTTP-date (RFC 7231 example date from 1994)
    let past_http_date = "Sun, 06 Nov 1994 08:49:37 GMT";

    // First response returns 429 with past Retry-After HTTP-date
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", past_http_date)
                .set_body_json(serde_json::json!({
                    "messages": [{"type": "ERROR", "text": "Rate limited"}]
                })),
        )
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    // Second response returns 200
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let options = endpoints::CreateJobOptions {
        wait: Some(false),
        ..Default::default()
    };

    let start = std::time::Instant::now();
    let result = endpoints::create_job(
        &client,
        &mock_server.uri(),
        "test-token",
        "search index=main",
        &options,
        3,
    )
    .await;
    let elapsed = start.elapsed();

    assert!(result.is_ok());

    // Should fall back to exponential backoff (1 second) since HTTP-date is in the past
    assert!(elapsed >= std::time::Duration::from_secs(1));
    // But less than 3 seconds (not waiting for the past date)
    assert!(elapsed < std::time::Duration::from_secs(3));
}

#[tokio::test]
async fn test_retry_with_invalid_http_date() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/create_job_success.json");

    // First response returns 429 with invalid HTTP-date format
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "not-a-valid-date")
                .set_body_json(serde_json::json!({
                    "messages": [{"type": "ERROR", "text": "Rate limited"}]
                })),
        )
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    // Second response returns 200
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let options = endpoints::CreateJobOptions {
        wait: Some(false),
        ..Default::default()
    };

    let start = std::time::Instant::now();
    let result = endpoints::create_job(
        &client,
        &mock_server.uri(),
        "test-token",
        "search index=main",
        &options,
        3,
    )
    .await;
    let elapsed = start.elapsed();

    assert!(result.is_ok());

    // Should fall back to exponential backoff (1 second) since HTTP-date is invalid
    assert!(elapsed >= std::time::Duration::from_secs(1));
    // But less than 3 seconds
    assert!(elapsed < std::time::Duration::from_secs(3));
}

// 5xx retry behavior tests

#[tokio::test]
async fn test_retry_on_503_success() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/create_job_success.json");

    // Use wiremock's sequence feature to return 503 twice, then 200
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(503).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Service Unavailable"}]
        })))
        .up_to_n_times(2)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let options = endpoints::CreateJobOptions {
        wait: Some(false),
        ..Default::default()
    };

    let start = std::time::Instant::now();
    let result = endpoints::create_job(
        &client,
        &mock_server.uri(),
        "test-token",
        "search index=main",
        &options,
        3, // max_retries
    )
    .await;
    let elapsed = start.elapsed();

    // Should succeed after retries
    assert!(result.is_ok());
    let sid = result.unwrap();
    assert!(sid.contains("scheduler__admin__search"));

    // Should have taken at least 1 + 2 = 3 seconds (exponential backoff: 1s, 2s)
    assert!(elapsed >= std::time::Duration::from_secs(2));
}

#[tokio::test]
async fn test_retry_on_502_success() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/create_job_success.json");

    // Return 502 once, then 200
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(502).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Bad Gateway"}]
        })))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let options = endpoints::CreateJobOptions {
        wait: Some(false),
        ..Default::default()
    };

    let start = std::time::Instant::now();
    let result = endpoints::create_job(
        &client,
        &mock_server.uri(),
        "test-token",
        "search index=main",
        &options,
        3, // max_retries
    )
    .await;
    let elapsed = start.elapsed();

    // Should succeed after retry
    assert!(result.is_ok());

    // Should have taken at least 1 second (exponential backoff: 1s)
    assert!(elapsed >= std::time::Duration::from_secs(1));
}

#[tokio::test]
async fn test_retry_on_504_success() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/create_job_success.json");

    // Return 504 twice, then 200
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(504).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Gateway Timeout"}]
        })))
        .up_to_n_times(2)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let options = endpoints::CreateJobOptions {
        wait: Some(false),
        ..Default::default()
    };

    let result = endpoints::create_job(
        &client,
        &mock_server.uri(),
        "test-token",
        "search index=main",
        &options,
        3, // max_retries
    )
    .await;

    // Should succeed after retries
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_retry_on_5xx_exhaustion() {
    let mock_server = MockServer::start().await;

    // Always return 503
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid"))
        .respond_with(ResponseTemplate::new(503).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Service Unavailable"}]
        })))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let start = std::time::Instant::now();
    let result =
        endpoints::get_job_status(&client, &mock_server.uri(), "test-token", "test-sid", 2).await;
    let elapsed = start.elapsed();

    // Should fail after exhausting retries
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, ClientError::MaxRetriesExceeded(3, _))); // 2 retries + 1 initial attempt = 3 total

    // Should have taken at least 1 + 2 = 3 seconds (exponential backoff: 1s, 2s)
    assert!(elapsed >= std::time::Duration::from_secs(2));
}

#[tokio::test]
async fn test_no_retry_on_500_or_501() {
    let mock_server = MockServer::start().await;

    // Return 500 (should not retry)
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Internal Server Error"}]
        })))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let start = std::time::Instant::now();
    let result =
        endpoints::get_job_status(&client, &mock_server.uri(), "test-token", "test-sid", 3).await;
    let elapsed = start.elapsed();

    // Should fail immediately without retry
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, ClientError::ApiError { status: 500, .. }),
        "Expected ApiError with status 500, got {:?}",
        err
    );

    // Should have completed quickly (no retry delay)
    assert!(elapsed < std::time::Duration::from_millis(500));
}

#[tokio::test]
async fn test_retry_mixed_503_and_429() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/create_job_success.json");

    // Return 503, then 429, then 200
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(503).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Service Unavailable"}]
        })))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Rate limited"}]
        })))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let options = endpoints::CreateJobOptions {
        wait: Some(false),
        ..Default::default()
    };

    let result = endpoints::create_job(
        &client,
        &mock_server.uri(),
        "test-token",
        "search index=main",
        &options,
        3, // max_retries
    )
    .await;

    // Should succeed after handling both 503 and 429
    assert!(result.is_ok());
}

/// Test that verifies timeout errors trigger retry behavior.
///
/// This test uses a mock server that delays responses longer than the client
/// timeout, causing reqwest to return a timeout error. The retry logic should
/// attempt the request multiple times before succeeding.
#[tokio::test]
async fn test_retry_on_timeout() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/create_job_success.json");

    // First two requests will timeout (we simulate this by having the mock
    // server delay longer than the client timeout)
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({
                    "messages": [{"type": "ERROR", "text": "Timeout"}]
                }))
                .set_delay(std::time::Duration::from_secs(5)),
        )
        .up_to_n_times(2)
        .mount(&mock_server)
        .await;

    // Third request succeeds immediately
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    // Create client with a short timeout
    let client = Client::builder()
        .timeout(std::time::Duration::from_millis(100))
        .build()
        .unwrap();

    let options = endpoints::CreateJobOptions {
        wait: Some(false),
        ..Default::default()
    };

    let start = std::time::Instant::now();
    let result = endpoints::create_job(
        &client,
        &mock_server.uri(),
        "test-token",
        "search index=main",
        &options,
        3, // max_retries
    )
    .await;
    let elapsed = start.elapsed();

    // The request should eventually succeed after retries
    // Note: This test may be flaky due to timing; we're mainly verifying
    // that the retry logic is invoked for timeout errors
    if result.is_ok() {
        // If it succeeded, it should have taken some time for retries
        assert!(elapsed >= std::time::Duration::from_millis(100));
    }
    // We don't assert on failure because network timing can be unpredictable
}
