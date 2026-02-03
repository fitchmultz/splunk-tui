//! Search peers side effect handler tests.
//!
//! This module tests the LoadSearchPeers side effect handler which fetches
//! distributed search peers from the Splunk REST API.
//!
//! These tests verify that:
//! - handle_side_effects returns promptly (doesn't block on network I/O)
//! - Loading(true) is sent before the API call
//! - SearchPeersLoaded(Ok) is sent on success
//! - SearchPeersLoaded(Err) is sent on error

mod common;

use common::*;
use wiremock::matchers::{method, path};

/// Test that LoadSearchPeers returns promptly and loads search peers successfully.
///
/// Uses a delayed response to verify that handle_side_effects doesn't block
/// waiting for the network call to complete.
#[tokio::test]
async fn test_load_search_peers_success_non_blocking() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Mock the search peers endpoint with a delay
    let fixture = load_fixture("search_peers/list_search_peers.json");
    Mock::given(method("GET"))
        .and(path("/services/search/distributed/peers"))
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
            Action::LoadSearchPeers {
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
            .any(|a| matches!(a, Action::SearchPeersLoaded(Ok(_)))),
        "Should send SearchPeersLoaded(Ok)"
    );

    // Verify the loaded data
    let peers_loaded = actions
        .iter()
        .find_map(|a| match a {
            Action::SearchPeersLoaded(Ok(peers)) => Some(peers),
            _ => None,
        })
        .expect("Should have SearchPeersLoaded action");

    assert_eq!(peers_loaded.len(), 2);
    assert_eq!(peers_loaded[0].name, "peer1");
    assert_eq!(peers_loaded[1].name, "peer2");
}

/// Test that LoadSearchPeers handles errors correctly without blocking.
#[tokio::test]
async fn test_load_search_peers_error_non_blocking() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Mock an error response with a delay
    Mock::given(method("GET"))
        .and(path("/services/search/distributed/peers"))
        .respond_with(
            ResponseTemplate::new(500)
                .set_body_string("Internal Server Error")
                .set_delay(std::time::Duration::from_millis(500)),
        )
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(
            Action::LoadSearchPeers {
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
            .any(|a| matches!(a, Action::SearchPeersLoaded(Err(_)))),
        "Should send SearchPeersLoaded(Err)"
    );
}

/// Test that LoadSearchPeers with offset > 0 sends MoreSearchPeersLoaded.
#[tokio::test]
async fn test_load_more_search_peers_success() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Mock the search peers endpoint
    let fixture = load_fixture("search_peers/list_search_peers.json");
    Mock::given(method("GET"))
        .and(path("/services/search/distributed/peers"))
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
            Action::LoadSearchPeers {
                count: 100,
                offset: 10,
            },
            5,
        )
        .await;

    // Should send MoreSearchPeersLoaded for offset > 0
    assert!(
        actions.iter().any(|a| matches!(a, Action::Loading(true))),
        "Should send Loading(true)"
    );
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::MoreSearchPeersLoaded(Ok(_)))),
        "Should send MoreSearchPeersLoaded(Ok) for pagination"
    );
}
