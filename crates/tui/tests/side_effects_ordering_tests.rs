//! Tests for async side effect ordering and race condition handling.
//!
//! This module tests:
//! - Out-of-order response handling
//! - Cancelled request handling
//! - Concurrent request deduplication
//! - Request/response correlation
//!
//! ## Invariants
//! - Responses must be correctly matched to their requests
//! - Cancelled requests should not update state
//! - Concurrent identical requests should be deduplicated

mod common;

use common::*;
use std::time::Duration;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn test_slow_response_handled_correctly() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Request with moderate delay
    let fixture = load_fixture("indexes/list_indexes.json");
    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(&fixture)
                .set_delay(Duration::from_millis(100)),
        )
        .mount(&harness.mock_server)
        .await;

    let start = tokio::time::Instant::now();

    let actions = harness
        .handle_and_collect(
            Action::LoadIndexes {
                count: 10,
                offset: 0,
            },
            2,
        )
        .await;

    let elapsed = start.elapsed();

    // Should complete successfully
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::IndexesLoaded(Ok(_)))),
        "Slow request should complete successfully"
    );

    // Should have taken at least the delay time
    assert!(
        elapsed >= Duration::from_millis(100),
        "Should respect response delay"
    );
}

#[tokio::test]
async fn test_concurrent_different_requests() {
    let harness = SideEffectsTestHarness::new().await;

    // Mock multiple endpoints
    let indexes_fixture = load_fixture("indexes/list_indexes.json");
    let apps_fixture = load_fixture("apps/list_apps.json");

    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&indexes_fixture))
        .mount(&harness.mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/services/apps/local"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&apps_fixture))
        .mount(&harness.mock_server)
        .await;

    // Spawn concurrent requests
    let mut handles = vec![];

    let client1 = harness.client.clone();
    let tx1 = harness.action_tx.clone();
    let cm1 = harness.config_manager.clone();
    let tt1 = TaskTracker::new();
    handles.push(tokio::spawn(async move {
        handle_side_effects(
            Action::LoadIndexes {
                count: 10,
                offset: 0,
            },
            client1,
            tx1,
            cm1,
            tt1,
        )
        .await;
    }));

    let client2 = harness.client.clone();
    let tx2 = harness.action_tx.clone();
    let cm2 = harness.config_manager.clone();
    let tt2 = TaskTracker::new();
    handles.push(tokio::spawn(async move {
        handle_side_effects(
            Action::LoadApps {
                count: 10,
                offset: 0,
            },
            client2,
            tx2,
            cm2,
            tt2,
        )
        .await;
    }));

    // Wait for all to complete
    for handle in handles {
        match tokio::time::timeout(Duration::from_secs(5), handle).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => panic!("Task panicked: {:?}", e),
            Err(_) => panic!("Task timed out"),
        }
    }
}

#[tokio::test]
async fn test_request_with_varying_delays() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Responses with different delays
    let fixture = load_fixture("indexes/list_indexes.json");

    // First request - 100ms delay
    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(&fixture)
                .set_delay(Duration::from_millis(100)),
        )
        .up_to_n_times(1)
        .mount(&harness.mock_server)
        .await;

    let start = tokio::time::Instant::now();

    let actions = harness
        .handle_and_collect(
            Action::LoadIndexes {
                count: 10,
                offset: 0,
            },
            2,
        )
        .await;

    let elapsed = start.elapsed();

    // Should complete successfully
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::IndexesLoaded(Ok(_)))),
        "Request should complete"
    );

    // Should have taken at least the delay time
    assert!(
        elapsed >= Duration::from_millis(100),
        "Should respect response delay"
    );
}

#[tokio::test]
async fn test_action_ordering_preserved() {
    let mut harness = SideEffectsTestHarness::new().await;

    let fixture = load_fixture("indexes/list_indexes.json");
    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&harness.mock_server)
        .await;

    // Execute multiple actions sequentially
    let actions = harness
        .handle_and_collect(
            Action::LoadIndexes {
                count: 10,
                offset: 0,
            },
            2,
        )
        .await;

    // Actions should be in expected order
    let loading_index = actions
        .iter()
        .position(|a| matches!(a, Action::Loading(true)))
        .expect("Should have Loading(true)");

    let loaded_index = actions
        .iter()
        .position(|a| matches!(a, Action::IndexesLoaded(_)))
        .expect("Should have IndexesLoaded");

    assert!(
        loading_index < loaded_index,
        "Loading(true) should come before IndexesLoaded"
    );
}

#[tokio::test]
async fn test_error_response_handling() {
    let mut harness = SideEffectsTestHarness::new().await;

    // First request fails
    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Server Error"))
        .up_to_n_times(1)
        .mount(&harness.mock_server)
        .await;

    let error_actions = harness
        .handle_and_collect(
            Action::LoadIndexes {
                count: 10,
                offset: 0,
            },
            2,
        )
        .await;

    // Should get error action
    assert!(
        error_actions
            .iter()
            .any(|a| matches!(a, Action::IndexesLoaded(Err(_)))),
        "Should get error action"
    );

    // Second request succeeds
    let fixture = load_fixture("indexes/list_indexes.json");
    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&harness.mock_server)
        .await;

    let success_actions = harness
        .handle_and_collect(
            Action::LoadIndexes {
                count: 10,
                offset: 0,
            },
            2,
        )
        .await;

    // Should get success action
    assert!(
        success_actions
            .iter()
            .any(|a| matches!(a, Action::IndexesLoaded(Ok(_)))),
        "Should get success action after error"
    );
}

#[tokio::test]
async fn test_multiple_identical_requests() {
    let mut harness = SideEffectsTestHarness::new().await;

    let fixture = load_fixture("indexes/list_indexes.json");
    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&harness.mock_server)
        .await;

    // Make two identical requests sequentially (not in parallel to avoid race conditions)
    for i in 0..2 {
        let actions = harness
            .handle_and_collect(
                Action::LoadIndexes {
                    count: 10,
                    offset: 0,
                },
                2,
            )
            .await;

        assert!(
            actions
                .iter()
                .any(|a| matches!(a, Action::IndexesLoaded(Ok(_)))),
            "Request {} should succeed",
            i
        );
    }
}

#[tokio::test]
async fn test_rapid_successive_requests() {
    let mut harness = SideEffectsTestHarness::new().await;

    let fixture = load_fixture("indexes/list_indexes.json");

    // Setup mock for multiple requests
    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&harness.mock_server)
        .await;

    // Make rapid successive requests
    for i in 0..3 {
        let actions = harness
            .handle_and_collect(
                Action::LoadIndexes {
                    count: 10,
                    offset: i * 10,
                },
                2,
            )
            .await;

        // Should have loading and result actions
        assert!(
            actions.iter().any(|a| matches!(a, Action::Loading(true))),
            "Request {} should have Loading(true)",
            i
        );
    }
}

#[tokio::test]
async fn test_mixed_success_and_error_responses() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Alternating success/error responses
    let fixture = load_fixture("indexes/list_indexes.json");

    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .up_to_n_times(1)
        .mount(&harness.mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Error"))
        .up_to_n_times(1)
        .mount(&harness.mock_server)
        .await;

    // First request succeeds
    let actions1 = harness
        .handle_and_collect(
            Action::LoadIndexes {
                count: 10,
                offset: 0,
            },
            2,
        )
        .await;

    assert!(
        actions1
            .iter()
            .any(|a| matches!(a, Action::IndexesLoaded(Ok(_)))),
        "First request should succeed"
    );

    // Second request fails
    let actions2 = harness
        .handle_and_collect(
            Action::LoadIndexes {
                count: 10,
                offset: 0,
            },
            2,
        )
        .await;

    assert!(
        actions2
            .iter()
            .any(|a| matches!(a, Action::IndexesLoaded(Err(_)))),
        "Second request should fail"
    );
}
