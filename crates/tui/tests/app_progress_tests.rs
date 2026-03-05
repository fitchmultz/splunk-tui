//! Tests for progress callback bridge (RQ-0128).
//!
//! This module tests:
//! - Progress callback sends correct Action
//! - Progress values are clamped to valid range [0.0, 1.0]
//! - Boundary values are preserved
//!
//! ## Invariants
//! - Progress values must be clamped to [0.0, 1.0]
//! - Callback must send Progress action via channel
//!
//! ## Test Organization
//! Tests focus on progress callback behavior.

use splunk_tui::action::{Action, progress_callback_to_action_sender};
use tokio::sync::mpsc::channel;

#[test]
fn test_progress_callback_bridge_sends_action() {
    let (tx, mut rx) = channel::<Action>(256);
    let mut callback = progress_callback_to_action_sender(tx);

    // Call the callback with a progress value
    callback(0.5);

    // Verify the action was sent
    let action = rx.try_recv().expect("Should receive action");
    assert!(
        matches!(action, Action::Progress(0.5)),
        "Should receive Progress action with value 0.5"
    );
}

#[test]
fn test_progress_callback_bridge_clamps_to_valid_range() {
    let (tx, mut rx) = channel::<Action>(256);
    let mut callback = progress_callback_to_action_sender(tx);

    // Test values outside [0.0, 1.0] range
    callback(-0.5);
    let action = rx.try_recv().expect("Should receive action");
    assert!(
        matches!(action, Action::Progress(0.0)),
        "Negative progress should be clamped to 0.0"
    );

    callback(1.5);
    let action = rx.try_recv().expect("Should receive action");
    assert!(
        matches!(action, Action::Progress(1.0)),
        "Progress > 1.0 should be clamped to 1.0"
    );
}

#[test]
fn test_progress_callback_bridge_preserves_valid_values() {
    let (tx, mut rx) = channel::<Action>(256);
    let mut callback = progress_callback_to_action_sender(tx);

    // Test boundary values
    callback(0.0);
    let action = rx.try_recv().expect("Should receive action");
    assert!(
        matches!(action, Action::Progress(0.0)),
        "Progress 0.0 should be preserved"
    );

    callback(1.0);
    let action = rx.try_recv().expect("Should receive action");
    assert!(
        matches!(action, Action::Progress(1.0)),
        "Progress 1.0 should be preserved"
    );

    callback(0.75);
    let action = rx.try_recv().expect("Should receive action");
    assert!(
        matches!(action, Action::Progress(0.75)),
        "Progress 0.75 should be preserved"
    );
}
