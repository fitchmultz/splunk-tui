//! Indexes side effect handler tests.
//!
//! This module tests the LoadIndexes side effect handler which fetches
//! index information from the Splunk REST API.

mod common;

use common::*;
use wiremock::matchers::{method, path};

#[tokio::test]
async fn test_load_indexes_success() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Mock the indexes endpoint
    let fixture = load_fixture("indexes/list_indexes.json");
    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&harness.mock_server)
        .await;

    // Handle the action
    let actions = harness
        .handle_and_collect(
            Action::LoadIndexes {
                count: 100,
                offset: 0,
            },
            2,
        )
        .await;

    // Verify actions sent
    assert!(
        actions.iter().any(|a| matches!(a, Action::Loading(true))),
        "Should send Loading(true)"
    );
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::IndexesLoaded(Ok(_)))),
        "Should send IndexesLoaded(Ok)"
    );

    // Verify the loaded data
    let indexes_loaded = actions
        .iter()
        .find_map(|a| match a {
            Action::IndexesLoaded(Ok(indexes)) => Some(indexes),
            _ => None,
        })
        .expect("Should have IndexesLoaded action");

    assert_eq!(indexes_loaded.len(), 3);
    assert_eq!(indexes_loaded[0].name, "main");
    assert_eq!(indexes_loaded[1].name, "_internal");
    assert_eq!(indexes_loaded[2].name, "_audit");
}

#[tokio::test]
async fn test_load_indexes_error() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Mock an error response
    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(
            Action::LoadIndexes {
                count: 100,
                offset: 0,
            },
            2,
        )
        .await;

    // Should still send Loading actions and error result
    assert!(
        actions.iter().any(|a| matches!(a, Action::Loading(true))),
        "Should send Loading(true)"
    );
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::IndexesLoaded(Err(_)))),
        "Should send IndexesLoaded(Err)"
    );
}
