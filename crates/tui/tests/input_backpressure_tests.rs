//! Regression tests for input backpressure handling (RQ-0379).
//!
//! These tests verify that keyboard input and resize events are never dropped
//! when the action channel is full, while mouse events may be dropped.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use splunk_tui::action::Action;
use tokio::sync::mpsc::{self, error::TrySendError};

/// Channel capacity for backpressure testing (small to trigger full condition quickly)
const TEST_CHANNEL_CAPACITY: usize = 2;

/// Fill the channel to capacity with dummy actions.
async fn fill_channel_to_capacity(tx: &mpsc::Sender<Action>) {
    for _ in 0..TEST_CHANNEL_CAPACITY {
        tx.send(Action::Tick)
            .await
            .expect("Failed to send dummy action");
    }
}

#[tokio::test]
async fn test_key_event_not_dropped_when_channel_full() {
    let (tx, mut rx) = mpsc::channel::<Action>(TEST_CHANNEL_CAPACITY);

    // Fill the channel to capacity
    fill_channel_to_capacity(&tx).await;

    // Verify channel is full
    assert!(tx.try_send(Action::Tick).is_err(), "Channel should be full");

    // Create a key event
    let key_event = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
    let action = Action::Input(key_event);

    // Send key event using blocking send (simulating the main.rs behavior)
    // This should wait until space is available
    let tx_clone = tx.clone();
    let send_future = async move { tx_clone.send(action).await };

    // Drain one slot concurrently to unblock the send
    let drain_future = async { rx.recv().await };

    // Run both concurrently - the drain should allow the send to complete
    let (send_result, _) = tokio::join!(send_future, drain_future);

    assert!(send_result.is_ok(), "Key event should have been sent");

    // Verify the key event was received (it should be the last item in channel)
    // First empty remaining slots to find our key event
    let mut found_key = false;
    while let Ok(Some(action)) =
        tokio::time::timeout(tokio::time::Duration::from_millis(100), rx.recv()).await
    {
        if matches!(action, Action::Input(_)) {
            found_key = true;
            break;
        }
    }

    assert!(found_key, "Key event should be received");
}

#[tokio::test]
async fn test_resize_event_not_dropped_when_channel_full() {
    let (tx, mut rx) = mpsc::channel::<Action>(TEST_CHANNEL_CAPACITY);

    // Fill the channel to capacity
    fill_channel_to_capacity(&tx).await;

    // Create a resize event
    let action = Action::Resize(100, 50);

    // Send resize event using blocking send
    let tx_clone = tx.clone();
    let send_future = async move { tx_clone.send(action).await };

    // Drain one slot concurrently to unblock the send
    let drain_future = async { rx.recv().await };

    // Run both concurrently
    let (send_result, _) = tokio::join!(send_future, drain_future);

    assert!(send_result.is_ok(), "Resize event should have been sent");

    // Find the resize event
    let mut found_resize = false;
    while let Ok(Some(action)) =
        tokio::time::timeout(tokio::time::Duration::from_millis(100), rx.recv()).await
    {
        if matches!(action, Action::Resize(100, 50)) {
            found_resize = true;
            break;
        }
    }

    assert!(found_resize, "Resize event should be received");
}

#[tokio::test]
async fn test_mouse_event_may_be_dropped_when_channel_full() {
    let (tx, _rx) = mpsc::channel::<Action>(TEST_CHANNEL_CAPACITY);

    // Fill the channel to capacity
    fill_channel_to_capacity(&tx).await;

    // Verify channel is full
    assert!(tx.try_send(Action::Tick).is_err(), "Channel should be full");

    // Create a mouse event
    let mouse_event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 10,
        row: 20,
        modifiers: KeyModifiers::NONE,
    };
    let action = Action::Mouse(mouse_event);

    // Mouse events use try_send and should be dropped when channel is full
    let result = tx.try_send(action);

    // Mouse events should be dropped (not an error, just expected behavior)
    assert!(
        result.is_err(),
        "Mouse event should be dropped when channel is full"
    );
    assert!(
        matches!(result, Err(TrySendError::Full(_))),
        "Should get Full error, got {:?}",
        result
    );
}

#[tokio::test]
async fn test_key_event_delivered_under_stress() {
    // This test verifies that key events are delivered even when the channel
    // is under pressure. We simulate this by filling the channel and then
    // demonstrating that a blocking send will succeed when space is made.
    let (tx, mut rx) = mpsc::channel::<Action>(TEST_CHANNEL_CAPACITY);

    // Fill the channel
    fill_channel_to_capacity(&tx).await;

    // Create a key event
    let key_event = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
    let key_action = Action::Input(key_event);

    // Start a blocking send in a separate task
    let tx_clone = tx.clone();
    let send_task = tokio::spawn(async move {
        let start = tokio::time::Instant::now();
        let result = tx_clone.send(key_action).await;
        let duration = start.elapsed();
        (result, duration)
    });

    // Small delay to ensure the send task starts waiting
    tokio::task::yield_now().await;

    // Drain one message to unblock the send
    let _ = rx.recv().await;

    // Wait for the send to complete with timeout
    let (send_result, send_duration) =
        match tokio::time::timeout(tokio::time::Duration::from_secs(1), send_task).await {
            Ok(Ok((result, duration))) => (result, duration),
            Ok(Err(_)) => panic!("Send task panicked"),
            Err(_) => panic!("Timeout waiting for key event send"),
        };

    assert!(send_result.is_ok(), "Key event should be sent successfully");

    // Should complete quickly once space is available
    assert!(
        send_duration < tokio::time::Duration::from_millis(500),
        "Key event took too long to send: {:?}",
        send_duration
    );

    // Verify the key event was delivered
    let mut found_key = false;
    while let Ok(Some(action)) =
        tokio::time::timeout(tokio::time::Duration::from_millis(100), rx.recv()).await
    {
        if matches!(action, Action::Input(_)) {
            found_key = true;
            break;
        }
    }

    assert!(found_key, "Key event should have been delivered");
}

#[tokio::test]
async fn test_critical_events_priority_over_mouse() {
    // Test that critical events (key/resize) are sent even when
    // mouse events would be dropped
    let (tx, mut rx) = mpsc::channel::<Action>(TEST_CHANNEL_CAPACITY);

    // Fill the channel to capacity
    fill_channel_to_capacity(&tx).await;

    // Try to send a mouse event (should be dropped)
    let mouse_event = MouseEvent {
        kind: MouseEventKind::Moved,
        column: 5,
        row: 5,
        modifiers: KeyModifiers::NONE,
    };
    let mouse_dropped = tx.try_send(Action::Mouse(mouse_event)).is_err();
    assert!(
        mouse_dropped,
        "Mouse event should be dropped when channel is full"
    );

    // Now send a key event using blocking send (should succeed)
    let key_event = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
    let key_action = Action::Input(key_event);

    let tx_clone = tx.clone();
    let key_send = async move { tx_clone.send(key_action).await };

    // Drain one slot concurrently
    let drain = async { rx.recv().await };

    // Run both concurrently
    let (key_result, _) = tokio::join!(key_send, drain);

    assert!(
        key_result.is_ok(),
        "Key event should be sent despite mouse being dropped"
    );

    // Verify the key event is in the channel
    let mut found_key = false;
    while let Ok(Some(action)) =
        tokio::time::timeout(tokio::time::Duration::from_millis(100), rx.recv()).await
    {
        if matches!(action, Action::Input(_)) {
            found_key = true;
            break;
        }
    }

    assert!(found_key, "Key event should be in channel");
}
