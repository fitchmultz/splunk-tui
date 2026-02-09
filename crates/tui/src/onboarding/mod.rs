//! Interactive first-run tutorial and onboarding wizard.

pub mod state;
pub mod steps;

#[cfg(test)]
mod tests;

pub use state::{TutorialState, TutorialStep};
pub use steps::TutorialSteps;
