//! Progress callback edge case tests.
//!
//! This module tests the progress callback functionality in wait_for_job_with_progress:
//! - None callback handling
//! - Progress value accuracy
//! - Rapid update handling
//! - Panic handling
//! - Blocking callback behavior
//!
//! # Invariants
//! - Callback receives done_progress as f64 in range [0.0, 1.0]
//! - None callback is handled gracefully (no panic)
//! - Rapid updates don't cause race conditions
//!
//! # What this does NOT handle
//! - Live server testing (see live_tests.rs)
//! - Job lifecycle (see jobs_tests.rs)

mod common;

use common::*;
use std::panic::AssertUnwindSafe;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use wiremock::matchers::{method, path, query_param};

/// Helper to create a job status response with specific progress values.
fn make_status_response(sid: &str, done_progress: f64, is_done: bool) -> serde_json::Value {
    serde_json::json!({
        "entry": [{
            "content": {
                "sid": sid,
                "isDone": is_done,
                "isFinalized": false,
                "doneProgress": done_progress,
                "runDuration": 5.5,
                "cursorTime": "2024-01-15T10:30:00.000-05:00",
                "scanCount": 1000,
                "eventCount": 500,
                "resultCount": 250,
                "diskUsage": 1024
            }
        }]
    })
}

#[tokio::test]
async fn test_none_callback_handling() {
    // Verify that wait_for_job_with_progress works correctly when progress_cb is None.
    // This is the path taken by wait_for_job.
    let mock_server = MockServer::start().await;

    // First call: job not done
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid"))
        .and(query_param("output_mode", "json"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(make_status_response("test-sid", 0.5, false)),
        )
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    // Second call: job done
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid"))
        .and(query_param("output_mode", "json"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(make_status_response("test-sid", 1.0, true)),
        )
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::wait_for_job_with_progress(
        &client,
        &mock_server.uri(),
        "test-token",
        "test-sid",
        10,   // poll_interval_ms (short for test speed)
        30,   // max_wait_secs
        3,    // max_retries
        None, // progress_cb: None
        None, // metrics: None
        None, // circuit_breaker: None
    )
    .await;

    assert!(
        result.is_ok(),
        "wait_for_job_with_progress with None callback should succeed"
    );
    let status = result.unwrap();
    assert!(status.is_done);
    assert_eq!(status.done_progress, 1.0);
}

#[tokio::test]
async fn test_progress_accuracy() {
    // Verify that done_progress values are passed correctly to the callback.
    let mock_server = MockServer::start().await;

    // Sequence of progress values: 0.0, 0.25, 0.5, 0.75, 1.0
    let progress_sequence = vec![0.0_f64, 0.25, 0.5, 0.75, 1.0];

    for progress in &progress_sequence {
        let is_done = *progress >= 1.0;
        Mock::given(method("GET"))
            .and(path("/services/search/jobs/test-sid"))
            .and(query_param("output_mode", "json"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(make_status_response("test-sid", *progress, is_done)),
            )
            .up_to_n_times(1)
            .mount(&mock_server)
            .await;
    }

    let progress_values = Arc::new(Mutex::new(Vec::new()));
    let progress_clone = Arc::clone(&progress_values);

    let mut callback = move |progress: f64| {
        progress_clone.lock().unwrap().push(progress);
    };

    let client = Client::new();
    let result = endpoints::wait_for_job_with_progress(
        &client,
        &mock_server.uri(),
        "test-token",
        "test-sid",
        10, // poll_interval_ms
        30, // max_wait_secs
        3,  // max_retries
        Some(&mut callback),
        None, // metrics: None
        None, // circuit_breaker: None
    )
    .await;

    assert!(result.is_ok(), "wait_for_job_with_progress should succeed");
    let status = result.unwrap();
    assert!(status.is_done);

    // Verify all progress values were received
    let received = progress_values.lock().unwrap();
    assert_eq!(
        received.len(),
        progress_sequence.len(),
        "Should receive all progress updates"
    );

    for (i, (expected, actual)) in progress_sequence.iter().zip(received.iter()).enumerate() {
        assert!(
            (expected - actual).abs() < f64::EPSILON,
            "Progress value at index {} should be {}, got {}",
            i,
            expected,
            actual
        );
    }
}

#[tokio::test]
async fn test_progress_values_in_range() {
    // Verify that progress values are always within [0.0, 1.0].
    let mock_server = MockServer::start().await;

    // Test with edge case progress values
    let test_cases = vec![
        (0.0_f64, false),
        (0.0001, false),
        (0.5, false),
        (0.9999, false),
        (1.0, true),
    ];

    for (progress, is_done) in &test_cases {
        Mock::given(method("GET"))
            .and(path("/services/search/jobs/test-sid"))
            .and(query_param("output_mode", "json"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(make_status_response("test-sid", *progress, *is_done)),
            )
            .up_to_n_times(1)
            .mount(&mock_server)
            .await;
    }

    let progress_values = Arc::new(Mutex::new(Vec::new()));
    let progress_clone = Arc::clone(&progress_values);

    let mut callback = move |progress: f64| {
        // Verify progress is in valid range
        assert!(
            (0.0..=1.0).contains(&progress),
            "Progress {} should be in range [0.0, 1.0]",
            progress
        );
        progress_clone.lock().unwrap().push(progress);
    };

    let client = Client::new();
    let result = endpoints::wait_for_job_with_progress(
        &client,
        &mock_server.uri(),
        "test-token",
        "test-sid",
        10,
        30,
        3,
        Some(&mut callback),
        None,
        None,
    )
    .await;

    assert!(result.is_ok());
    let received = progress_values.lock().unwrap();
    assert_eq!(received.len(), test_cases.len());
}

#[tokio::test]
async fn test_rapid_progress_updates() {
    // Verify callback handles rapid polling without issues.
    // This tests that many rapid updates don't cause race conditions.
    let mock_server = MockServer::start().await;

    let update_count = 50;

    // Create many sequential responses with small progress increments
    for i in 0..update_count {
        let progress = (i + 1) as f64 / update_count as f64;
        let is_done = i == update_count - 1;

        Mock::given(method("GET"))
            .and(path("/services/search/jobs/test-sid"))
            .and(query_param("output_mode", "json"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(make_status_response("test-sid", progress, is_done)),
            )
            .up_to_n_times(1)
            .mount(&mock_server)
            .await;
    }

    let callback_count = Arc::new(AtomicUsize::new(0));
    let count_clone = Arc::clone(&callback_count);

    let mut callback = move |_progress: f64| {
        count_clone.fetch_add(1, Ordering::SeqCst);
    };

    let client = Client::new();
    let result = endpoints::wait_for_job_with_progress(
        &client,
        &mock_server.uri(),
        "test-token",
        "test-sid",
        1, // Very short poll interval (1ms)
        60,
        3,
        Some(&mut callback),
        None, // metrics: None
        None, // circuit_breaker: None
    )
    .await;

    assert!(result.is_ok(), "Rapid updates should not cause failures");

    // Verify all callbacks were invoked
    let final_count = callback_count.load(Ordering::SeqCst);
    assert_eq!(
        final_count, update_count,
        "All {} callbacks should be invoked, got {}",
        update_count, final_count
    );
}

#[tokio::test]
async fn test_callback_with_shared_state_thread_safety() {
    // Verify that Send bound is sufficient for typical use cases with shared state.
    let mock_server = MockServer::start().await;

    // Create 3 responses
    for i in 0..3 {
        let progress = (i + 1) as f64 / 3.0;
        let is_done = i == 2;

        Mock::given(method("GET"))
            .and(path("/services/search/jobs/test-sid"))
            .and(query_param("output_mode", "json"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(make_status_response("test-sid", progress, is_done)),
            )
            .up_to_n_times(1)
            .mount(&mock_server)
            .await;
    }

    // Use Arc<Mutex<Vec<f64>>> to collect progress values across callbacks
    let progress_values: Arc<Mutex<Vec<f64>>> = Arc::new(Mutex::new(Vec::new()));
    let progress_clone: Arc<Mutex<Vec<f64>>> = Arc::clone(&progress_values);

    let mut callback = move |progress: f64| {
        // This tests that the Send bound allows thread-safe access
        let mut values = progress_clone.lock().unwrap();
        values.push(progress);
    };

    let client = Client::new();
    let result = endpoints::wait_for_job_with_progress(
        &client,
        &mock_server.uri(),
        "test-token",
        "test-sid",
        10,
        30,
        3,
        Some(&mut callback),
        None,
        None,
    )
    .await;

    assert!(result.is_ok());

    let received = progress_values.lock().unwrap();
    assert_eq!(received.len(), 3);
    // Verify values are in expected order
    assert!(received[0] < received[1]);
    assert!(received[1] < received[2]);
}

#[tokio::test]
async fn test_blocking_callback() {
    // Verify that a blocking callback doesn't deadlock the polling loop.
    // The callback blocks until the test releases it while poll interval is 10ms.
    let mock_server = MockServer::start().await;

    // Create 3 responses
    for i in 0..3 {
        let progress = (i + 1) as f64 / 3.0;
        let is_done = i == 2;

        Mock::given(method("GET"))
            .and(path("/services/search/jobs/test-sid"))
            .and(query_param("output_mode", "json"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(make_status_response("test-sid", progress, is_done)),
            )
            .up_to_n_times(1)
            .mount(&mock_server)
            .await;
    }

    let callback_count = Arc::new(AtomicUsize::new(0));
    let count_clone = Arc::clone(&callback_count);
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();

    let release_thread = std::thread::spawn(move || {
        for _ in 0..3 {
            entered_rx
                .recv()
                .expect("callback entry signal should be received");
            release_tx.send(()).expect("release signal should be sent");
        }
    });

    let mut callback = move |_progress: f64| {
        entered_tx
            .send(())
            .expect("callback entry signal should be sent");
        release_rx
            .recv()
            .expect("release signal should be received");
        count_clone.fetch_add(1, Ordering::SeqCst);
    };

    let client = Client::new();
    let result = endpoints::wait_for_job_with_progress(
        &client,
        &mock_server.uri(),
        "test-token",
        "test-sid",
        10, // poll interval is shorter than callback duration
        30,
        3,
        Some(&mut callback),
        None,
        None,
    )
    .await;

    assert!(
        result.is_ok(),
        "Blocking callback should not prevent completion"
    );

    // All callbacks should still be invoked
    let final_count = callback_count.load(Ordering::SeqCst);
    assert_eq!(
        final_count, 3,
        "All callbacks should be invoked despite blocking"
    );

    release_thread
        .join()
        .expect("release thread should join cleanly");
}

#[tokio::test]
async fn test_callback_panic_propagates() {
    // Verify that a panicking callback propagates the panic.
    // This documents the current behavior: panics are NOT caught.
    let mock_server = MockServer::start().await;

    // Create responses that will trigger panic at progress > 0.5
    let progress_values = vec![0.25_f64, 0.75, 1.0];

    for progress in &progress_values {
        Mock::given(method("GET"))
            .and(path("/services/search/jobs/test-sid"))
            .and(query_param("output_mode", "json"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(make_status_response(
                    "test-sid",
                    *progress,
                    *progress >= 1.0,
                )),
            )
            .up_to_n_times(1)
            .mount(&mock_server)
            .await;
    }

    let mut callback = |progress: f64| {
        if progress > 0.5 {
            panic!("Callback panicked at progress {}", progress);
        }
    };

    let client = Client::new();

    // Use AssertUnwindSafe to allow catching the panic in the test
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        // We need to block on the async function
        let rt = tokio::runtime::Handle::current();
        rt.block_on(endpoints::wait_for_job_with_progress(
            &client,
            &mock_server.uri(),
            "test-token",
            "test-sid",
            10,
            30,
            3,
            Some(&mut callback),
            None,
            None,
        ))
    }));

    assert!(result.is_err(), "Callback panic should propagate");
}

#[tokio::test]
async fn test_wait_for_job_uses_none_callback() {
    // Verify that wait_for_job correctly calls wait_for_job_with_progress with None.
    // This is an integration test of the public API.
    let mock_server = MockServer::start().await;

    // First call: job not done
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid"))
        .and(query_param("output_mode", "json"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(make_status_response("test-sid", 0.5, false)),
        )
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    // Second call: job done
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid"))
        .and(query_param("output_mode", "json"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(make_status_response("test-sid", 1.0, true)),
        )
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::wait_for_job(
        &client,
        &mock_server.uri(),
        "test-token",
        "test-sid",
        10,
        30,
        3,
        None,
        None,
    )
    .await;

    assert!(result.is_ok(), "wait_for_job should succeed");
    let status = result.unwrap();
    assert!(status.is_done);
}

#[tokio::test]
async fn test_progress_callback_with_timeout() {
    // Verify that timeout still works when using a progress callback.
    let mock_server = MockServer::start().await;

    // Job never completes
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid"))
        .and(query_param("output_mode", "json"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(make_status_response("test-sid", 0.5, false)),
        )
        .mount(&mock_server)
        .await;

    let callback_count = Arc::new(AtomicUsize::new(0));
    let count_clone = Arc::clone(&callback_count);

    let mut callback = move |progress: f64| {
        assert_eq!(progress, 0.5);
        count_clone.fetch_add(1, Ordering::SeqCst);
    };

    let client = Client::new();
    let result = endpoints::wait_for_job_with_progress(
        &client,
        &mock_server.uri(),
        "test-token",
        "test-sid",
        50, // 50ms poll interval
        1,  // 1 second max wait
        3,
        Some(&mut callback),
        None,
        None,
    )
    .await;

    assert!(result.is_err(), "Should timeout when job doesn't complete");
    let err = result.unwrap_err();
    let err_string = err.to_string();
    assert!(
        err_string.to_lowercase().contains("timeout")
            || err_string.to_lowercase().contains("timed out"),
        "Error should indicate timeout: {}",
        err
    );

    // Callback should have been invoked multiple times before timeout
    let count = callback_count.load(Ordering::SeqCst);
    assert!(
        count >= 10,
        "Callback should be invoked multiple times before timeout, got {}",
        count
    );
}
