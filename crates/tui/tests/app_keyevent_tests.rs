//! Tests for KeyEventKind filtering (RQ-0107 fix).
//!
//! This module tests that the helper functions create key events with the
//! correct KeyEventKind for testing input filtering.
//!
//! ## Invariants
//! - Release events must have KeyEventKind::Release
//! - Repeat events must have KeyEventKind::Repeat
//!
//! ## Test Organization
//! Tests are grouped by event kind verification.

mod helpers;
use crossterm::event::KeyCode;
use helpers::*;

// NOTE: The filtering for KeyEventKind happens in main.rs at the input task level.
// These tests verify that the helper functions create events with the correct kind.
// The app.handle_input() method does NOT check key.kind - it only looks at
// key.code and key.modifiers, which is why filtering must happen earlier in the pipeline.

#[test]
fn test_release_event_helper_creates_correct_kind() {
    let release = release_key('a');

    assert_eq!(release.kind, crossterm::event::KeyEventKind::Release);
    assert_eq!(release.code, KeyCode::Char('a'));
}

#[test]
fn test_repeat_event_helper_creates_correct_kind() {
    let repeat = repeat_key('b');

    assert_eq!(repeat.kind, crossterm::event::KeyEventKind::Repeat);
    assert_eq!(repeat.code, KeyCode::Char('b'));
}
