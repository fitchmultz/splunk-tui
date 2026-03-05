//! Lookups side effect handler tests.
//!
//! This module tests the LoadLookups side effect handler which fetches
//! lookup table files from the Splunk REST API.
//!
//! These tests verify that:
//! - handle_side_effects returns promptly (doesn't block on network I/O)
//! - Loading(true) is sent before the API call
//! - LookupsLoaded(Ok) is sent on success
//! - LookupsLoaded(Err) is sent on error

mod common;

use common::*;
use wiremock::matchers::{method, path};

/// Test that LoadLookups returns promptly and loads lookup tables successfully.
///
/// Uses a delayed response to verify that handle_side_effects doesn't block
/// waiting for the network call to complete.
#[tokio::test]
async fn test_load_lookups_success_non_blocking() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Mock the lookups endpoint with a delay
    let fixture = load_fixture("lookups/list_lookup_tables.json");
    Mock::given(method("GET"))
        .and(path("/services/data/lookup-table-files"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(&fixture)
                .set_delay(std::time::Duration::from_millis(500)),
        )
        .mount(&harness.mock_server)
        .await;

    // Handle the action - should return promptly despite the delay
    let actions = harness
        .handle_and_collect(
            Action::LoadLookups {
                count: 100,
                offset: 0,
            },
            5,
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
            .any(|a| matches!(a, Action::LookupsLoaded(Ok(_)))),
        "Should send LookupsLoaded(Ok)"
    );

    // Verify the loaded data
    let lookups_loaded = actions
        .iter()
        .find_map(|a| match a {
            Action::LookupsLoaded(Ok(lookups)) => Some(lookups),
            _ => None,
        })
        .expect("Should have LookupsLoaded action");

    assert_eq!(lookups_loaded.len(), 2);
    assert_eq!(lookups_loaded[0].name, "my_lookup");
    assert_eq!(lookups_loaded[1].name, "countries");
}

/// Test that LoadLookups handles errors correctly without blocking.
#[tokio::test]
async fn test_load_lookups_error_non_blocking() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Mock an error response with a delay
    Mock::given(method("GET"))
        .and(path("/services/data/lookup-table-files"))
        .respond_with(
            ResponseTemplate::new(500)
                .set_body_string("Internal Server Error")
                .set_delay(std::time::Duration::from_millis(500)),
        )
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(
            Action::LoadLookups {
                count: 100,
                offset: 0,
            },
            5,
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
            .any(|a| matches!(a, Action::LookupsLoaded(Err(_)))),
        "Should send LookupsLoaded(Err)"
    );
}

/// Test that LoadLookups with offset > 0 sends MoreLookupsLoaded.
#[tokio::test]
async fn test_load_more_lookups_success() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Mock the lookups endpoint
    let fixture = load_fixture("lookups/list_lookup_tables.json");
    Mock::given(method("GET"))
        .and(path("/services/data/lookup-table-files"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(&fixture)
                .set_delay(std::time::Duration::from_millis(100)),
        )
        .mount(&harness.mock_server)
        .await;

    // Handle with offset > 0 (pagination)
    let actions = harness
        .handle_and_collect(
            Action::LoadLookups {
                count: 100,
                offset: 10,
            },
            5,
        )
        .await;

    // Should send MoreLookupsLoaded for offset > 0
    assert!(
        actions.iter().any(|a| matches!(a, Action::Loading(true))),
        "Should send Loading(true)"
    );
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::MoreLookupsLoaded(Ok(_)))),
        "Should send MoreLookupsLoaded(Ok) for pagination"
    );
}
