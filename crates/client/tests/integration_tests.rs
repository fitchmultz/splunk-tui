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
    let result = endpoints::login(&client, &mock_server.uri(), "admin", "testpassword").await;

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
    let result = endpoints::login(&client, &mock_server.uri(), "admin", "wrongpassword").await;

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
        endpoints::get_job_status(&client, &mock_server.uri(), "test-token", "test-sid-123").await;

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
    let result =
        endpoints::list_indexes(&client, &mock_server.uri(), "test-token", Some(10), Some(0)).await;

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
    let result =
        endpoints::list_jobs(&client, &mock_server.uri(), "test-token", Some(10), Some(0)).await;

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
    let result = endpoints::cancel_job(&client, &mock_server.uri(), "test-token", "test-sid").await;

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
    let result = endpoints::delete_job(&client, &mock_server.uri(), "test-token", "test-sid").await;

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
    let result = endpoints::get_cluster_info(&client, &mock_server.uri(), "test-token").await;

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
    let result = endpoints::get_cluster_info(&client, &mock_server.uri(), "test-token").await;

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
    let result =
        endpoints::list_indexes(&client, &mock_server.uri(), "test-token", Some(10), Some(0)).await;

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
    )
    .await;

    // The request should timeout or return an error
    assert!(result.is_err());
}
