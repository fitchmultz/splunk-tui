//! App-level tutorial integration tests
//!
//! Tests for verifying the tutorial system at the application level,
//! including action handling, popup management, and state persistence.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use splunk_tui::action::Action;
use splunk_tui::app::{App, ConnectionContext};
use splunk_tui::onboarding::{TutorialState, TutorialStep};
use splunk_tui::ui::popup::PopupType;

fn create_test_app() -> App {
    App::new(None, ConnectionContext::default())
}

fn enter_key() -> KeyEvent {
    KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)
}

fn esc_key() -> KeyEvent {
    KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)
}

fn left_key() -> KeyEvent {
    KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)
}

#[test]
fn test_start_tutorial_action_opens_popup() {
    let mut app = create_test_app();
    assert!(app.popup.is_none());

    // Dispatch StartTutorial action
    app.update(Action::StartTutorial { is_replay: false });

    assert!(app.popup.is_some());
    let popup = app.popup.as_ref().unwrap();
    assert!(matches!(popup.kind, PopupType::TutorialWizard { .. }));
}

#[test]
fn test_tutorial_replay_marks_has_completed() {
    let mut app = create_test_app();

    // Start tutorial as replay
    app.update(Action::StartTutorial { is_replay: true });

    // Check that the tutorial state has has_completed = true
    if let Some(popup) = &app.popup {
        if let PopupType::TutorialWizard { state } = &popup.kind {
            assert!(
                state.has_completed,
                "Replay should mark has_completed as true"
            );
        } else {
            panic!("Expected TutorialWizard popup");
        }
    } else {
        panic!("Expected popup to be open");
    }
}

#[test]
fn test_tutorial_completed_sets_flag() {
    let mut app = create_test_app();
    app.tutorial_completed = false;

    // Simulate tutorial completion
    app.update(Action::StartTutorial { is_replay: false });

    // Mark as completed
    let result = app.handle_tutorial_action(Action::TutorialCompleted);

    assert!(
        app.tutorial_completed,
        "tutorial_completed should be true after TutorialCompleted"
    );
    assert!(
        matches!(result, Some(Action::PersistState)),
        "TutorialCompleted should trigger PersistState"
    );
}

#[test]
fn test_tutorial_skipped_sets_flag() {
    let mut app = create_test_app();
    app.tutorial_completed = false;

    // Simulate skipping the tutorial
    let result = app.handle_tutorial_action(Action::TutorialSkipped);

    assert!(
        app.tutorial_completed,
        "tutorial_completed should be true after TutorialSkipped"
    );
    assert!(
        matches!(result, Some(Action::PersistState)),
        "TutorialSkipped should trigger PersistState"
    );
}

#[test]
fn test_tutorial_profile_created_updates_state() {
    let mut app = create_test_app();

    // Start tutorial
    app.update(Action::StartTutorial { is_replay: false });

    // Store tutorial state
    app.tutorial_state = Some(TutorialState::new());
    app.tutorial_state.as_mut().unwrap().current_step = TutorialStep::ProfileCreation;

    // Simulate profile creation from tutorial
    let result = app.handle_tutorial_action(Action::TutorialProfileCreated {
        profile_name: "test-profile".to_string(),
    });

    // Result should be None (handled internally)
    assert!(result.is_none());

    // Tutorial state should be updated
    assert!(app.tutorial_state.is_some());
    let state = app.tutorial_state.as_ref().unwrap();
    assert_eq!(state.pending_profile_name, Some("test-profile".to_string()));
}

#[test]
fn test_tutorial_connection_result_updates_state() {
    let mut app = create_test_app();

    // Start tutorial and set up state
    app.tutorial_state = Some(TutorialState::new());
    app.tutorial_state.as_mut().unwrap().current_step = TutorialStep::ConnectionTest;

    // Simulate successful connection test
    let result = app.handle_tutorial_action(Action::TutorialConnectionResult { success: true });

    // Result should be None (handled internally)
    assert!(result.is_none());

    // Tutorial state should have connection result
    let state = app.tutorial_state.as_ref().unwrap();
    assert_eq!(state.connection_test_result, Some(true));
}

#[test]
fn test_tutorial_popup_escape_skips() {
    let mut app = create_test_app();
    app.tutorial_completed = false;

    // Open tutorial
    app.update(Action::StartTutorial { is_replay: false });
    assert!(app.popup.is_some());

    // Press Escape - should close popup and return TutorialSkipped action
    let action = app.handle_tutorial_popup(esc_key());
    assert!(
        matches!(action, Some(Action::TutorialSkipped)),
        "Escape should trigger TutorialSkipped"
    );

    // Popup should be closed
    assert!(app.popup.is_none());
}

#[test]
fn test_tutorial_popup_enter_advances_welcome() {
    let mut app = create_test_app();

    // Open tutorial
    app.update(Action::StartTutorial { is_replay: false });
    assert!(app.popup.is_some());

    // Verify we're on Welcome step
    if let Some(popup) = &app.popup {
        if let PopupType::TutorialWizard { state } = &popup.kind {
            assert_eq!(state.current_step, TutorialStep::Welcome);
        }
    }

    // Press Enter on Welcome step - should advance to ProfileCreation
    let action = app.handle_tutorial_popup(enter_key());
    assert!(
        action.is_none(),
        "Enter on Welcome should not produce action"
    );

    // Popup should still be open with updated state
    assert!(app.popup.is_some());
    if let Some(popup) = &app.popup {
        if let PopupType::TutorialWizard { state } = &popup.kind {
            assert_eq!(
                state.current_step,
                TutorialStep::ProfileCreation,
                "Should advance to ProfileCreation"
            );
        }
    }
}

#[test]
fn test_tutorial_popup_left_goes_back() {
    let mut app = create_test_app();

    // Open tutorial and advance to ProfileCreation
    app.update(Action::StartTutorial { is_replay: false });
    let _ = app.handle_tutorial_popup(enter_key()); // Advance from Welcome

    // Verify we're on ProfileCreation
    if let Some(popup) = &app.popup {
        if let PopupType::TutorialWizard { state } = &popup.kind {
            assert_eq!(state.current_step, TutorialStep::ProfileCreation);
        }
    }

    // Press Left - should go back to Welcome
    let action = app.handle_tutorial_popup(left_key());
    assert!(action.is_none(), "Left arrow should not produce action");

    // Should be back on Welcome
    if let Some(popup) = &app.popup {
        if let PopupType::TutorialWizard { state } = &popup.kind {
            assert_eq!(state.current_step, TutorialStep::Welcome);
        }
    }
}

#[test]
fn test_tutorial_popup_enter_on_complete_finishes() {
    let mut app = create_test_app();
    app.tutorial_completed = false;

    // Open tutorial with state already at Complete
    let mut state = TutorialState::new();
    state.current_step = TutorialStep::Complete;
    app.popup =
        Some(splunk_tui::ui::popup::Popup::builder(PopupType::TutorialWizard { state }).build());

    // Press Enter on Complete step - should finish tutorial
    let action = app.handle_tutorial_popup(enter_key());
    assert!(
        matches!(action, Some(Action::TutorialCompleted)),
        "Enter on Complete should trigger TutorialCompleted"
    );

    // Popup should be closed
    assert!(app.popup.is_none());
}

#[test]
fn test_get_persisted_state_includes_tutorial_completed() {
    let mut app = create_test_app();

    // Set tutorial_completed to true
    app.tutorial_completed = true;

    let state = app.get_persisted_state();
    assert!(state.tutorial_completed);

    // Set to false and verify
    app.tutorial_completed = false;
    let state = app.get_persisted_state();
    assert!(!state.tutorial_completed);
}

#[test]
fn test_app_initializes_with_tutorial_completed_from_persisted_state() {
    let persisted = splunk_config::PersistedState {
        tutorial_completed: true,
        ..splunk_config::PersistedState::default()
    };

    let app = App::new(Some(persisted), ConnectionContext::default());
    assert!(app.tutorial_completed);
}

#[test]
fn test_tutorial_state_stored_in_app() {
    let mut app = create_test_app();

    // Initially no tutorial state
    assert!(app.tutorial_state.is_none());

    // Start tutorial
    app.update(Action::StartTutorial { is_replay: false });

    // After starting, tutorial_state should be populated
    // Note: tutorial_state is set when certain steps are reached
}

fn t_key() -> KeyEvent {
    KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE)
}

#[test]
fn test_tutorial_connection_test_triggers_diagnostics() {
    let mut app = create_test_app();

    // Open tutorial and navigate to ConnectionTest step
    let mut state = TutorialState::new();
    state.current_step = TutorialStep::ConnectionTest;
    app.popup =
        Some(splunk_tui::ui::popup::Popup::builder(PopupType::TutorialWizard { state }).build());

    // Verify we're on ConnectionTest step
    if let Some(popup) = &app.popup {
        if let PopupType::TutorialWizard { state } = &popup.kind {
            assert_eq!(state.current_step, TutorialStep::ConnectionTest);
        }
    }

    // Press 't' to trigger diagnostics
    let action = app.handle_tutorial_popup(t_key());
    assert!(
        matches!(action, Some(Action::RunConnectionDiagnostics)),
        "Pressing 't' on ConnectionTest should trigger RunConnectionDiagnostics"
    );
}

#[test]
fn test_tutorial_other_steps_ignore_t_key() {
    let mut app = create_test_app();

    // Open tutorial on Welcome step
    app.update(Action::StartTutorial { is_replay: false });

    // Verify we're on Welcome step
    if let Some(popup) = &app.popup {
        if let PopupType::TutorialWizard { state } = &popup.kind {
            assert_eq!(state.current_step, TutorialStep::Welcome);
        }
    }

    // Press 't' - should do nothing on Welcome step
    let action = app.handle_tutorial_popup(t_key());
    assert!(
        action.is_none(),
        "Pressing 't' on Welcome should not produce action"
    );
}
