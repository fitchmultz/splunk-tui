//! Data indexes endpoint tests.
//!
//! This module tests the Splunk data indexes API:
//! - Listing all available indexes
//!
//! # Invariants
//! - Indexes are returned with their names and metadata
//! - Results are paginated according to the provided limit/offset parameters
//!
//! # What this does NOT handle
//! - Index creation/deletion (not supported by this client)
//! - Index configuration updates

mod common;

use common::*;
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
