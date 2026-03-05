//! Data indexes endpoint tests.
//!
//! This module tests the Splunk data indexes API:
//! - Listing all available indexes
//! - Creating new indexes
//! - Modifying existing indexes
//! - Deleting indexes
//!
//! # Invariants
//! - Indexes are returned with their names and metadata
//! - Results are paginated according to the provided limit/offset parameters
//! - Index creation/modification returns the updated index data
//! - Index deletion returns successfully

mod common;

use common::*;
use splunk_client::error::ClientError;
use splunk_client::{CreateIndexParams, ModifyIndexParams};
use wiremock::matchers::{method, path};

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
        None,
        None,
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
async fn test_create_index() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("indexes/create_index.json");

    Mock::given(method("POST"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(201).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let params = CreateIndexParams {
        name: "test_index".to_string(),
        max_data_size_mb: Some(1000usize),
        max_hot_buckets: Some(10usize),
        max_warm_db_count: Some(300usize),
        frozen_time_period_in_secs: Some(15552000usize),
        home_path: None,
        cold_db_path: None,
        thawed_path: None,
        cold_to_frozen_dir: None,
    };

    let result = endpoints::create_index(
        &client,
        &mock_server.uri(),
        "test-token",
        &params,
        3,
        None,
        None,
    )
    .await;

    if let Err(ref e) = result {
        eprintln!("Create index error: {:?}", e);
    }
    assert!(result.is_ok());
    let index = result.unwrap();
    assert_eq!(index.name, "test_index");
    assert_eq!(index.max_total_data_size_mb, Some(1000));
}

#[tokio::test]
async fn test_modify_index() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("indexes/modify_index.json");

    Mock::given(method("POST"))
        .and(path("/services/data/indexes/main"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let params = ModifyIndexParams {
        max_data_size_mb: Some(2000usize),
        max_hot_buckets: Some(15usize),
        max_warm_db_count: Some(400usize),
        frozen_time_period_in_secs: Some(2592000usize),
        home_path: None,
        cold_db_path: None,
        thawed_path: None,
        cold_to_frozen_dir: None,
    };

    let result = endpoints::modify_index(
        &client,
        &mock_server.uri(),
        "test-token",
        "main",
        &params,
        3,
        None,
        None,
    )
    .await;

    if let Err(ref e) = result {
        eprintln!("Modify index error: {:?}", e);
    }
    assert!(result.is_ok());
    let index = result.unwrap();
    assert_eq!(index.name, "main");
    assert_eq!(index.max_total_data_size_mb, Some(2000));
}

#[tokio::test]
async fn test_delete_index() {
    let mock_server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/services/data/indexes/test_index"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::delete_index(
        &client,
        &mock_server.uri(),
        "test-token",
        "test_index",
        3,
        None,
        None,
    )
    .await;

    if let Err(ref e) = result {
        eprintln!("Delete index error: {:?}", e);
    }
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_list_indexes_unauthorized() {
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
async fn test_list_indexes_forbidden() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Forbidden"}]
        })))
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
        None,
        None,
    )
    .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, ClientError::ApiError { status: 403, .. }));
}

#[tokio::test]
async fn test_create_index_unauthorized() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Unauthorized"}]
        })))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let params = CreateIndexParams {
        name: "test_index".to_string(),
        max_data_size_mb: None,
        max_hot_buckets: None,
        max_warm_db_count: None,
        frozen_time_period_in_secs: None,
        home_path: None,
        cold_db_path: None,
        thawed_path: None,
        cold_to_frozen_dir: None,
    };

    let result = endpoints::create_index(
        &client,
        &mock_server.uri(),
        "invalid-token",
        &params,
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
async fn test_modify_index_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/services/data/indexes/nonexistent"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Not Found"}]
        })))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let params = ModifyIndexParams {
        max_data_size_mb: Some(2000usize),
        max_hot_buckets: None,
        max_warm_db_count: None,
        frozen_time_period_in_secs: None,
        home_path: None,
        cold_db_path: None,
        thawed_path: None,
        cold_to_frozen_dir: None,
    };

    let result = endpoints::modify_index(
        &client,
        &mock_server.uri(),
        "test-token",
        "nonexistent",
        &params,
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
async fn test_delete_index_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("DELETE"))
        .and(path("/services/data/indexes/nonexistent"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Not Found"}]
        })))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::delete_index(
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
    // 404 is now classified as NotFound variant
    assert!(
        matches!(err, ClientError::NotFound(_)),
        "Expected NotFound, got {:?}",
        err
    );
}

#[tokio::test]
async fn test_list_indexes_malformed_response() {
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
        None,
        None,
    )
    .await;

    assert!(result.is_err());
}
