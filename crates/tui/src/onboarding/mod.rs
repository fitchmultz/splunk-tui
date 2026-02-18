//! Interactive first-run tutorial and onboarding wizard.

pub mod checklist;
pub mod state;
pub mod steps;
pub mod tutorial_keybindings;

#[cfg(test)]
mod tests;

pub use checklist::{
    AUTO_HIDE_SESSIONS, HINT_COOLDOWN_SECS, MAX_HINTS_PER_SESSION, OnboardingChecklistState,
    OnboardingMilestone, OnboardingMilestones,
};
pub use state::{TutorialState, TutorialStep};
pub use steps::TutorialSteps;
pub use tutorial_keybindings::generate_keybinding_section;
