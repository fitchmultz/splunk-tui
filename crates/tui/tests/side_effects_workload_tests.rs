//! Workload management side effect handler tests.
//!
//! This module tests the LoadWorkloadPools and LoadWorkloadRules side effect
//! handlers which fetch workload management configuration from the Splunk REST API.
//!
//! These tests verify that:
//! - handle_side_effects returns promptly (doesn't block on network I/O)
//! - Loading(true) is sent before the API call
//! - WorkloadPoolsLoaded/WorkloadRulesLoaded actions are sent
//! - Both success and error cases are handled without blocking

mod common;

use common::*;
use wiremock::matchers::{method, path};

/// Test that LoadWorkloadPools returns promptly even with a delayed error response.
///
/// Uses a delayed 500 response to verify that handle_side_effects doesn't block
/// waiting for the network call to complete.
#[tokio::test]
async fn test_load_workload_pools_error_non_blocking() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Mock an error response with a delay (workload endpoints may not be available)
    Mock::given(method("GET"))
        .and(path("/services/workloads/pools"))
        .respond_with(
            ResponseTemplate::new(500)
                .set_body_string("Internal Server Error")
                .set_delay(std::time::Duration::from_millis(500)),
        )
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(
            Action::LoadWorkloadPools {
                count: 100,
                offset: 0,
            },
            5,
        )
        .await;

    // Should send Loading(true) before the API call
    assert!(
        actions.iter().any(|a| matches!(a, Action::Loading(true))),
        "Should send Loading(true)"
    );

    // Should eventually send WorkloadPoolsLoaded(Err)
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::WorkloadPoolsLoaded(Err(_)))),
        "Should send WorkloadPoolsLoaded(Err)"
    );
}

/// Test that LoadWorkloadRules returns promptly even with a delayed error response.
#[tokio::test]
async fn test_load_workload_rules_error_non_blocking() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Mock an error response with a delay
    Mock::given(method("GET"))
        .and(path("/services/workloads/rules"))
        .respond_with(
            ResponseTemplate::new(500)
                .set_body_string("Internal Server Error")
                .set_delay(std::time::Duration::from_millis(500)),
        )
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(
            Action::LoadWorkloadRules {
                count: 100,
                offset: 0,
            },
            5,
        )
        .await;

    // Should send Loading(true) before the API call
    assert!(
        actions.iter().any(|a| matches!(a, Action::Loading(true))),
        "Should send Loading(true)"
    );

    // Should eventually send WorkloadRulesLoaded(Err)
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::WorkloadRulesLoaded(Err(_)))),
        "Should send WorkloadRulesLoaded(Err)"
    );
}

/// Test that LoadWorkloadPools with offset > 0 sends MoreWorkloadPoolsLoaded.
#[tokio::test]
async fn test_load_more_workload_pools_error() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Mock an error response with a delay
    Mock::given(method("GET"))
        .and(path("/services/workloads/pools"))
        .respond_with(
            ResponseTemplate::new(404)
                .set_body_string("Not Found")
                .set_delay(std::time::Duration::from_millis(100)),
        )
        .mount(&harness.mock_server)
        .await;

    // Handle with offset > 0 (pagination)
    let actions = harness
        .handle_and_collect(
            Action::LoadWorkloadPools {
                count: 100,
                offset: 10,
            },
            5,
        )
        .await;

    // Should send MoreWorkloadPoolsLoaded for offset > 0
    assert!(
        actions.iter().any(|a| matches!(a, Action::Loading(true))),
        "Should send Loading(true)"
    );
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::MoreWorkloadPoolsLoaded(Err(_)))),
        "Should send MoreWorkloadPoolsLoaded(Err) for pagination"
    );
}

/// Test that LoadWorkloadRules with offset > 0 sends MoreWorkloadRulesLoaded.
#[tokio::test]
async fn test_load_more_workload_rules_error() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Mock an error response with a delay
    Mock::given(method("GET"))
        .and(path("/services/workloads/rules"))
        .respond_with(
            ResponseTemplate::new(404)
                .set_body_string("Not Found")
                .set_delay(std::time::Duration::from_millis(100)),
        )
        .mount(&harness.mock_server)
        .await;

    // Handle with offset > 0 (pagination)
    let actions = harness
        .handle_and_collect(
            Action::LoadWorkloadRules {
                count: 100,
                offset: 10,
            },
            5,
        )
        .await;

    // Should send MoreWorkloadRulesLoaded for offset > 0
    assert!(
        actions.iter().any(|a| matches!(a, Action::Loading(true))),
        "Should send Loading(true)"
    );
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::MoreWorkloadRulesLoaded(Err(_)))),
        "Should send MoreWorkloadRulesLoaded(Err) for pagination"
    );
}
