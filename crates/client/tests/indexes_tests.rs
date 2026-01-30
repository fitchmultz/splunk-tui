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
        max_data_size_mb: Some(1000u64),
        max_hot_buckets: Some(10u64),
        max_warm_db_count: Some(300u64),
        frozen_time_period_in_secs: Some(15552000u64),
        home_path: None,
        cold_db_path: None,
        thawed_path: None,
        cold_to_frozen_dir: None,
    };

    let result =
        endpoints::create_index(&client, &mock_server.uri(), "test-token", &params, 3, None).await;

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
        max_data_size_mb: Some(2000u64),
        max_hot_buckets: Some(15u64),
        max_warm_db_count: Some(400u64),
        frozen_time_period_in_secs: Some(2592000u64),
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
    )
    .await;

    if let Err(ref e) = result {
        eprintln!("Delete index error: {:?}", e);
    }
    assert!(result.is_ok());
}
