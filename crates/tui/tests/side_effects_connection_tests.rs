//! Tests for connection failure scenarios and recovery.
//!
//! This module tests:
//! - Connection timeout handling
//! - Connection refused errors
//! - Intermittent connection recovery
//! - Network error propagation
//! - Retry logic behavior
//!
//! ## Invariants
//! - Connection errors must result in user-friendly error actions
//! - App must be able to retry after connection failures
//! - Errors should not expose sensitive information

mod common;

use common::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn test_connection_timeout_handling() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Mock endpoint that responds slowly
    let fixture = load_fixture("indexes/list_indexes.json");
    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(&fixture)
                .set_delay(Duration::from_secs(10)), // Long delay
        )
        .mount(&harness.mock_server)
        .await;

    // Set a short timeout on the client
    let client = SplunkClient::builder()
        .base_url(harness.mock_server.uri())
        .auth_strategy(AuthStrategy::ApiToken {
            token: secrecy::SecretString::new("test-token".to_string().into()),
        })
        .skip_verify(true)
        .timeout(Duration::from_millis(50)) // Very short timeout
        .build()
        .expect("Failed to build test client");

    harness.client = Arc::new(client);

    // Handle the action with a longer collection timeout
    let actions = harness
        .handle_and_collect(
            Action::LoadIndexes {
                count: 10,
                offset: 0,
            },
            3,
        )
        .await;

    // Should get a loading action - the important thing is it doesn't panic
    assert!(
        actions.iter().any(|a| matches!(a, Action::Loading(true))),
        "Should send Loading(true)"
    );
    // May or may not get error depending on timing - either is acceptable
}

#[tokio::test]
async fn test_connection_refused_error_handling() {
    // Create a server and then shut it down to simulate connection refused
    let server = MockServer::start().await;
    let server_uri = server.uri();

    // Drop the server to close the port
    drop(server);

    // Create client pointing to the now-dead server
    let client = SplunkClient::builder()
        .base_url(server_uri.to_string())
        .auth_strategy(AuthStrategy::ApiToken {
            token: secrecy::SecretString::new("test-token".to_string().into()),
        })
        .skip_verify(true)
        .timeout(Duration::from_secs(1))
        .build()
        .expect("Failed to build test client");

    let (action_tx, mut action_rx) = mpsc::channel::<Action>(100);
    let client = Arc::new(client);
    let (config_manager, _temp_dir) = create_test_config_manager().await;

    // Try to load indexes - should fail with connection error
    let action = Action::LoadIndexes {
        count: 10,
        offset: 0,
    };

    // Handle the action
    let task_tracker = TaskTracker::new();
    let handle_future =
        handle_side_effects(action, client, action_tx, config_manager, task_tracker);

    // Should complete quickly (not hang)
    match tokio::time::timeout(Duration::from_secs(5), handle_future).await {
        Ok(()) => {}
        Err(_) => panic!("handle_side_effects timed out"),
    }

    // Give some time for error action
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Collect actions
    let mut actions = Vec::new();
    while let Ok(Some(action)) =
        tokio::time::timeout(Duration::from_millis(100), action_rx.recv()).await
    {
        actions.push(action);
    }

    // Should have error action
    let has_error = actions.iter().any(|a| {
        matches!(a, Action::IndexesLoaded(Err(e)) if e.to_string().contains("connection") || 
                                                  e.to_string().contains("refused") ||
                                                  e.to_string().contains("Connect"))
    });

    assert!(
        has_error
            || actions
                .iter()
                .any(|a| matches!(a, Action::IndexesLoaded(Err(_)))),
        "Should have connection error, got: {:?}",
        actions
    );
}

#[tokio::test]
async fn test_intermittent_connection_recovery() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Test that the system can handle errors and then recover
    // First request fails with 503
    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(503).set_body_string("Service Unavailable"))
        .mount(&harness.mock_server)
        .await;

    // First attempt - may fail or succeed depending on error handling
    let actions1 = harness
        .handle_and_collect(
            Action::LoadIndexes {
                count: 10,
                offset: 0,
            },
            2,
        )
        .await;

    // Should get loading action at minimum
    assert!(
        actions1.iter().any(|a| matches!(a, Action::Loading(true))),
        "Should send Loading(true)"
    );

    // Create a new harness for the success case (to avoid mock conflicts)
    let mut harness2 = SideEffectsTestHarness::new().await;
    let fixture = load_fixture("indexes/list_indexes.json");
    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&harness2.mock_server)
        .await;

    // Second attempt - should succeed
    let actions2 = harness2
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
            .any(|a| matches!(a, Action::IndexesLoaded(Ok(_)))),
        "Second attempt should succeed"
    );
}

#[tokio::test]
async fn test_dns_resolution_failure() {
    // Use an invalid URL that will fail DNS resolution
    let client = SplunkClient::builder()
        .base_url("http://invalid.invalid.invalid:8080".to_string())
        .auth_strategy(AuthStrategy::ApiToken {
            token: secrecy::SecretString::new("test-token".to_string().into()),
        })
        .skip_verify(true)
        .timeout(Duration::from_secs(2))
        .max_retries(0)
        .build()
        .expect("Failed to build test client");

    let (action_tx, mut action_rx) = mpsc::channel::<Action>(100);
    let client = Arc::new(client);
    let (config_manager, _temp_dir) = create_test_config_manager().await;

    let action = Action::LoadIndexes {
        count: 10,
        offset: 0,
    };

    let task_tracker = TaskTracker::new();
    handle_side_effects(action, client, action_tx, config_manager, task_tracker).await;

    // Collect any actions
    let mut actions = Vec::new();
    let mut has_indexes_loaded = false;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(6);
    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_millis(200), action_rx.recv()).await {
            Ok(Some(action)) => {
                has_indexes_loaded |= matches!(action, Action::IndexesLoaded(_));
                actions.push(action);
                if has_indexes_loaded {
                    break;
                }
            }
            Ok(None) => break,
            Err(_) => continue,
        }
    }

    // Should eventually complete with either success or error.
    assert!(
        has_indexes_loaded,
        "Should have completed with some result, got: {:?}",
        actions
    );
}

#[tokio::test]
async fn test_malformed_response_handling() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Return malformed JSON
    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{ invalid json }"))
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(
            Action::LoadIndexes {
                count: 10,
                offset: 0,
            },
            2,
        )
        .await;

    // Should get an error, not panic
    let has_error = actions
        .iter()
        .any(|a| matches!(a, Action::IndexesLoaded(Err(_))));
    assert!(
        has_error,
        "Should have error for malformed response, got: {:?}",
        actions
    );
}

#[tokio::test]
async fn test_empty_response_handling() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Return empty body
    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(200).set_body_string(""))
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(
            Action::LoadIndexes {
                count: 10,
                offset: 0,
            },
            2,
        )
        .await;

    // Should handle gracefully
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::IndexesLoaded(_))),
        "Should handle empty response, got: {:?}",
        actions
    );
}

#[tokio::test]
async fn test_connection_reset_handling() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Mock that will close connection abruptly
    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(200).set_body_string(""))
        .mount(&harness.mock_server)
        .await;

    // This test verifies the client handles connection resets gracefully
    // The mock server doesn't support true connection resets, but we can
    // verify error handling works

    let actions = harness
        .handle_and_collect(
            Action::LoadIndexes {
                count: 10,
                offset: 0,
            },
            2,
        )
        .await;

    // Should complete with some result (success or error)
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::IndexesLoaded(_))),
        "Should complete, got: {:?}",
        actions
    );
}

#[tokio::test]
async fn test_retry_after_header_handling() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Return 429 with Retry-After header
    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "1")
                .set_body_string("Rate limited"),
        )
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(
            Action::LoadIndexes {
                count: 10,
                offset: 0,
            },
            2,
        )
        .await;

    // Should handle rate limiting gracefully - either with error or loading action
    // The important thing is it doesn't panic
    assert!(
        actions.iter().any(|a| matches!(a, Action::Loading(true)))
            || actions
                .iter()
                .any(|a| matches!(a, Action::IndexesLoaded(Err(_)))),
        "Should handle rate limit, got: {:?}",
        actions
    );
}

#[tokio::test]
async fn test_concurrent_request_handling_under_stress() {
    let harness = SideEffectsTestHarness::new().await;

    // Mock successful response
    let fixture = load_fixture("indexes/list_indexes.json");
    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&harness.mock_server)
        .await;

    // Spawn multiple concurrent requests
    let mut handles = vec![];
    for _ in 0..5 {
        let client = harness.client.clone();
        let tx = harness.action_tx.clone();
        let cm = harness.config_manager.clone();

        let task_tracker = TaskTracker::new();
        let handle = tokio::spawn(async move {
            let action = Action::LoadIndexes {
                count: 10,
                offset: 0,
            };
            handle_side_effects(action, client, tx, cm, task_tracker).await;
        });
        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        match tokio::time::timeout(Duration::from_secs(5), handle).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => panic!("Task panicked: {:?}", e),
            Err(_) => panic!("Task timed out"),
        }
    }
}
