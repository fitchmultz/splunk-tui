//! Snapshot tests for tutorial wizard popup rendering.
//!
//! Provides regression coverage for all 7 tutorial steps to ensure
//! onboarding UX remains consistent across refactors.

mod helpers;

use helpers::TuiHarness;
use splunk_tui::Popup;
use splunk_tui::onboarding::{TutorialState, TutorialStep};
use splunk_tui::ui::popup::PopupType;

fn create_tutorial_popup(step: TutorialStep) -> splunk_tui::ui::popup::Popup {
    let mut state = TutorialState::new();
    state.current_step = step;
    Popup::builder(PopupType::TutorialWizard { state }).build()
}

#[test]
fn snapshot_tutorial_welcome() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.popup = Some(create_tutorial_popup(TutorialStep::Welcome));
    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_tutorial_profile_creation() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.popup = Some(create_tutorial_popup(TutorialStep::ProfileCreation));
    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_tutorial_connection_test() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.popup = Some(create_tutorial_popup(TutorialStep::ConnectionTest));
    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_tutorial_first_search() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.popup = Some(create_tutorial_popup(TutorialStep::FirstSearch));
    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_tutorial_keybinding_tutorial() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.popup = Some(create_tutorial_popup(TutorialStep::KeybindingTutorial));
    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_tutorial_export_demo() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.popup = Some(create_tutorial_popup(TutorialStep::ExportDemo));
    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_tutorial_complete() {
    let mut harness = TuiHarness::new(80, 24);
    let mut state = TutorialState::new();
    state.current_step = TutorialStep::Complete;
    state.has_completed = true;
    harness.app.popup = Some(Popup::builder(PopupType::TutorialWizard { state }).build());
    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_tutorial_small_terminal() {
    // Test with narrow terminal (40x20) to verify scroll behavior
    let mut harness = TuiHarness::new(40, 20);
    harness.app.popup = Some(create_tutorial_popup(TutorialStep::KeybindingTutorial));
    insta::assert_snapshot!(harness.render());
}
