//! Integration tests using wiremock to test HTTP endpoints.

use reqwest::Client;
use splunk_client::endpoints;
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path, query_param},
};

// Re-export commonly used types for test convenience
use splunk_client::ClientError;

/// Helper to load fixture files.
fn load_fixture(fixture_path: &str) -> serde_json::Value {
    let fixture_dir = std::path::Path::new("fixtures");
    let full_path = fixture_dir.join(fixture_path);
    let content = std::fs::read_to_string(&full_path)
        .unwrap_or_else(|_| panic!("Failed to load fixture: {}", full_path.display()));
    serde_json::from_str(&content).expect("Invalid JSON in fixture")
}

#[tokio::test]
async fn test_login_success() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("auth/login_success.json");

    Mock::given(method("POST"))
        .and(path("/services/auth/login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::login(&client, &mock_server.uri(), "admin", "testpassword", 3).await;

    if let Err(ref e) = result {
        eprintln!("Login error: {:?}", e);
    }
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "test-session-key-12345678");
}

#[tokio::test]
async fn test_login_invalid_credentials() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("auth/login_invalid_creds.json");

    Mock::given(method("POST"))
        .and(path("/services/auth/login"))
        .respond_with(ResponseTemplate::new(401).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::login(&client, &mock_server.uri(), "admin", "wrongpassword", 3).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, ClientError::ApiError { status: 401, .. }));
}

#[tokio::test]
async fn test_create_search_job() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/create_job_success.json");

    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let options = endpoints::CreateJobOptions {
        wait: Some(false),
        exec_time: Some(60),
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

    assert!(result.is_ok());
    let sid = result.unwrap();
    assert!(sid.contains("scheduler__admin__search"));
}

#[tokio::test]
async fn test_get_search_results() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/get_results.json");

    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::get_results(
        &client,
        &mock_server.uri(),
        "test-token",
        "test-sid",
        Some(10),
        Some(0),
        endpoints::OutputMode::Json,
        3,
    )
    .await;

    if let Err(ref e) = result {
        eprintln!("Get results error: {:?}", e);
    }
    assert!(result.is_ok());
    let results = result.unwrap();
    assert_eq!(results.results.len(), 3);
    assert_eq!(results.results[0]["message"], "Test event 1");
}

#[tokio::test]
async fn test_get_job_status() {
    let mock_server = MockServer::start().await;

    let fixture = serde_json::json!({
        "entry": [{
            "content": {
                "sid": "test-sid-123",
                "isDone": true,
                "isFinalized": false,
                "doneProgress": 1.0,
                "runDuration": 5.5,
                "cursorTime": "2024-01-15T10:30:00.000-05:00",
                "scanCount": 1000,
                "eventCount": 500,
                "resultCount": 250,
                "diskUsage": 1024
            }
        }]
    });

    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid-123"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result =
        endpoints::get_job_status(&client, &mock_server.uri(), "test-token", "test-sid-123", 3)
            .await;

    assert!(result.is_ok());
    let status = result.unwrap();
    assert_eq!(status.sid, "test-sid-123");
    assert!(status.is_done);
    assert_eq!(status.result_count, 250);
}

#[tokio::test]
async fn test_list_indexes() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("indexes/list_indexes.json");

    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::list_indexes(
        &client,
        &mock_server.uri(),
        "test-token",
        Some(10),
        Some(0),
        3,
    )
    .await;

    if let Err(ref e) = result {
        eprintln!("List indexes error: {:?}", e);
    }
    assert!(result.is_ok());
    let indexes = result.unwrap();
    assert_eq!(indexes.len(), 3);
    assert_eq!(indexes[0].name, "main");
    assert_eq!(indexes[1].name, "_internal");
    assert_eq!(indexes[2].name, "_audit");
}

#[tokio::test]
async fn test_list_jobs() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("jobs/list_jobs.json");

    Mock::given(method("GET"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::list_jobs(
        &client,
        &mock_server.uri(),
        "test-token",
        Some(10),
        Some(0),
        3,
    )
    .await;

    assert!(result.is_ok());
    let jobs = result.unwrap();
    assert_eq!(jobs.len(), 2);
    assert!(!jobs[0].is_done);
    assert!(jobs[1].is_done);
}

#[tokio::test]
async fn test_cancel_job() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/services/search/jobs/test-sid/control"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result =
        endpoints::cancel_job(&client, &mock_server.uri(), "test-token", "test-sid", 3).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_delete_job() {
    let mock_server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/services/search/jobs/test-sid"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result =
        endpoints::delete_job(&client, &mock_server.uri(), "test-token", "test-sid", 3).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_get_cluster_info() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("cluster/get_cluster_info.json");

    Mock::given(method("GET"))
        .and(path("/services/cluster/master/config"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::get_cluster_info(&client, &mock_server.uri(), "test-token", 3).await;

    assert!(result.is_ok());
    let info = result.unwrap();
    assert_eq!(info.id, "cluster-01");
    assert_eq!(info.label.as_deref(), Some("Production Cluster"));
    assert_eq!(info.mode, "peer");
    assert_eq!(info.replication_factor, Some(3));
    assert_eq!(info.search_factor, Some(2));
}

// Error path tests

#[tokio::test]
async fn test_unauthorized_access() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Unauthorized"}]
        })))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::list_indexes(
        &client,
        &mock_server.uri(),
        "invalid-token",
        Some(10),
        Some(0),
        3,
    )
    .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, ClientError::ApiError { status: 401, .. }));
}

#[tokio::test]
async fn test_forbidden_access() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/cluster/master/config"))
        .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Forbidden"}]
        })))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::get_cluster_info(&client, &mock_server.uri(), "test-token", 3).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, ClientError::ApiError { status: 403, .. }));
}

#[tokio::test]
async fn test_internal_server_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Internal server error"}]
        })))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::create_job(
        &client,
        &mock_server.uri(),
        "test-token",
        "search index=main",
        &Default::default(),
        3,
    )
    .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, ClientError::ApiError { status: 500, .. }));
}

#[tokio::test]
async fn test_malformed_json_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(200).set_body_string("invalid json"))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::list_indexes(
        &client,
        &mock_server.uri(),
        "test-token",
        Some(10),
        Some(0),
        3,
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_timeout_handling() {
    let mock_server = MockServer::start().await;

    // Simulate a timeout by not responding immediately
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/timeout-sid/results"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!([]))
                .set_delay(std::time::Duration::from_secs(10)),
        )
        .mount(&mock_server)
        .await;

    let client = Client::builder()
        .timeout(std::time::Duration::from_millis(100))
        .build()
        .unwrap();

    let result = endpoints::get_results(
        &client,
        &mock_server.uri(),
        "test-token",
        "timeout-sid",
        Some(10),
        Some(0),
        endpoints::OutputMode::Json,
        3,
    )
    .await;

    // The request should timeout or return an error
    assert!(result.is_err());
}

// Retry behavior tests

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
    assert!(matches!(err, ClientError::MaxRetriesExceeded(3))); // 2 retries + 1 initial attempt = 3 total

    // Should have taken at least 1 + 2 = 3 seconds (exponential backoff: 1s, 2s)
    assert!(elapsed >= std::time::Duration::from_secs(2));
}

// 401/403 retry behavior tests

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

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
