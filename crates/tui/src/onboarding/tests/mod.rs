//! Tutorial integration tests
//!
//! Tests for the tutorial system that verify:
//! - State machine transitions through all steps
//! - Progress calculation
//! - Reset behavior
//! - Step content and titles

use crate::onboarding::{TutorialState, TutorialStep};

#[test]
fn test_tutorial_state_machine_transitions() {
    let mut state = TutorialState::new();
    assert_eq!(state.current_step, TutorialStep::Welcome);
    assert!(!state.has_completed);

    // Advance through all steps
    state.next_step();
    assert_eq!(state.current_step, TutorialStep::ProfileCreation);

    state.next_step();
    assert_eq!(state.current_step, TutorialStep::ConnectionTest);

    state.next_step();
    assert_eq!(state.current_step, TutorialStep::FirstSearch);

    state.next_step();
    assert_eq!(state.current_step, TutorialStep::KeybindingTutorial);

    state.next_step();
    assert_eq!(state.current_step, TutorialStep::ExportDemo);

    state.next_step();
    assert_eq!(state.current_step, TutorialStep::Complete);
    assert!(!state.has_completed); // complete() must be called separately

    state.complete();
    assert!(state.has_completed);
}

#[test]
fn test_tutorial_progress_percent() {
    let mut state = TutorialState::new();
    assert_eq!(state.progress_percent(), 0);

    state.current_step = TutorialStep::ProfileCreation;
    assert_eq!(state.progress_percent(), 16);

    state.current_step = TutorialStep::ConnectionTest;
    assert_eq!(state.progress_percent(), 33);

    state.current_step = TutorialStep::FirstSearch;
    assert_eq!(state.progress_percent(), 50);

    state.current_step = TutorialStep::KeybindingTutorial;
    assert_eq!(state.progress_percent(), 66);

    state.current_step = TutorialStep::ExportDemo;
    assert_eq!(state.progress_percent(), 83);

    state.current_step = TutorialStep::Complete;
    assert_eq!(state.progress_percent(), 100);
}

#[test]
fn test_tutorial_reset_preserves_has_completed() {
    let mut state = TutorialState::new();
    state.complete();
    assert!(state.has_completed);

    state.reset();
    assert_eq!(state.current_step, TutorialStep::Welcome);
    assert!(state.has_completed); // Should be preserved
}

#[test]
fn test_tutorial_step_titles() {
    assert!(TutorialStep::Welcome.title().contains("Welcome"));
    assert!(TutorialStep::ProfileCreation.title().contains("Profile"));
    assert!(TutorialStep::ConnectionTest.title().contains("Connection"));
    assert!(TutorialStep::FirstSearch.title().contains("Search"));
    assert!(
        TutorialStep::KeybindingTutorial
            .title()
            .contains("Keybinding")
    );
    assert!(TutorialStep::ExportDemo.title().contains("Export"));
    assert!(TutorialStep::Complete.title().contains("All Set"));
}

#[test]
fn test_tutorial_is_complete_check() {
    let mut state = TutorialState::new();
    assert!(!state.is_complete());

    state.current_step = TutorialStep::Complete;
    assert!(state.is_complete());
}

#[test]
fn test_tutorial_is_at_start_check() {
    let mut state = TutorialState::new();
    assert!(state.is_at_start());

    state.next_step();
    assert!(!state.is_at_start());
}

#[test]
fn test_tutorial_pending_profile_name() {
    let mut state = TutorialState::new();
    assert!(state.pending_profile_name.is_none());

    state.set_pending_profile_name("test-profile");
    assert_eq!(state.pending_profile_name, Some("test-profile".to_string()));
}

#[test]
fn test_tutorial_connection_test_result() {
    let mut state = TutorialState::new();
    assert!(state.connection_test_result.is_none());

    state.set_connection_test_result(true);
    assert_eq!(state.connection_test_result, Some(true));

    state.set_connection_test_result(false);
    assert_eq!(state.connection_test_result, Some(false));
}

#[test]
fn test_tutorial_first_search_marking() {
    let mut state = TutorialState::new();
    assert!(!state.has_run_first_search);

    state.mark_first_search_complete();
    assert!(state.has_run_first_search);
}

#[test]
fn test_tutorial_export_marking() {
    let mut state = TutorialState::new();
    assert!(!state.has_exported);

    state.mark_export_complete();
    assert!(state.has_exported);
}

#[test]
fn test_tutorial_keybinding_scroll() {
    let mut state = TutorialState::new();
    assert_eq!(state.keybinding_scroll_offset, 0);

    state.scroll_keybindings_down(5);
    assert_eq!(state.keybinding_scroll_offset, 5);

    state.scroll_keybindings_up(2);
    assert_eq!(state.keybinding_scroll_offset, 3);

    // Should saturate at 0
    state.scroll_keybindings_up(10);
    assert_eq!(state.keybinding_scroll_offset, 0);
}

#[test]
fn test_tutorial_previous_step_from_welcome() {
    let mut state = TutorialState::new();
    assert_eq!(state.current_step, TutorialStep::Welcome);

    // Should not go back from Welcome
    let result = state.previous_step();
    assert!(!result);
    assert_eq!(state.current_step, TutorialStep::Welcome);
}

#[test]
fn test_tutorial_next_step_from_complete() {
    let mut state = TutorialState::new();
    state.current_step = TutorialStep::Complete;

    // Should not advance from Complete
    let result = state.next_step();
    assert!(!result);
    assert_eq!(state.current_step, TutorialStep::Complete);
}
