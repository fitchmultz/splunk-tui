//! Forwarders side effect handler tests.
//!
//! This module tests the LoadForwarders side effect handler which fetches
//! deployment clients (forwarders) from the Splunk REST API.
//!
//! These tests verify that:
//! - handle_side_effects returns promptly (doesn't block on network I/O)
//! - Loading(true) is sent before the API call
//! - ForwardersLoaded(Ok) is sent on success
//! - ForwardersLoaded(Err) is sent on error

mod common;

use common::*;
use wiremock::matchers::{method, path};

/// Test that LoadForwarders returns promptly and loads forwarders successfully.
///
/// Uses a delayed response to verify that handle_side_effects doesn't block
/// waiting for the network call to complete.
#[tokio::test]
async fn test_load_forwarders_success_non_blocking() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Mock the forwarders endpoint with a delay
    let fixture = load_fixture("forwarders/list_forwarders.json");
    Mock::given(method("GET"))
        .and(path("/services/deployment/server/clients"))
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
            Action::LoadForwarders {
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
            .any(|a| matches!(a, Action::ForwardersLoaded(Ok(_)))),
        "Should send ForwardersLoaded(Ok)"
    );

    // Verify the loaded data
    let forwarders_loaded = actions
        .iter()
        .find_map(|a| match a {
            Action::ForwardersLoaded(Ok(forwarders)) => Some(forwarders),
            _ => None,
        })
        .expect("Should have ForwardersLoaded action");

    assert_eq!(forwarders_loaded.len(), 3);
    assert_eq!(forwarders_loaded[0].name, "forwarder1.example.com");
    assert_eq!(forwarders_loaded[1].name, "forwarder2.example.com");
    assert_eq!(forwarders_loaded[2].name, "windows-forwarder.corp.local");
}

/// Test that LoadForwarders handles errors correctly without blocking.
#[tokio::test]
async fn test_load_forwarders_error_non_blocking() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Mock an error response with a delay
    Mock::given(method("GET"))
        .and(path("/services/deployment/server/clients"))
        .respond_with(
            ResponseTemplate::new(500)
                .set_body_string("Internal Server Error")
                .set_delay(std::time::Duration::from_millis(500)),
        )
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(
            Action::LoadForwarders {
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
            .any(|a| matches!(a, Action::ForwardersLoaded(Err(_)))),
        "Should send ForwardersLoaded(Err)"
    );
}

/// Test that LoadForwarders with offset > 0 sends MoreForwardersLoaded.
#[tokio::test]
async fn test_load_more_forwarders_success() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Mock the forwarders endpoint
    let fixture = load_fixture("forwarders/list_forwarders.json");
    Mock::given(method("GET"))
        .and(path("/services/deployment/server/clients"))
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
            Action::LoadForwarders {
                count: 100,
                offset: 10,
            },
            5,
        )
        .await;

    // Should send MoreForwardersLoaded for offset > 0
    assert!(
        actions.iter().any(|a| matches!(a, Action::Loading(true))),
        "Should send Loading(true)"
    );
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::MoreForwardersLoaded(Ok(_)))),
        "Should send MoreForwardersLoaded(Ok) for pagination"
    );
}
