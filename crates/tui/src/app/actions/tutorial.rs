//! Tutorial action handlers.

use crate::action::Action;
use crate::app::App;
use crate::app::state::CurrentScreen;
use crate::onboarding::TutorialState;
use crate::ui::popup::{Popup, PopupType};

impl App {
    /// Handle tutorial-related actions.
    pub fn handle_tutorial_action(&mut self, action: Action) -> Option<Action> {
        match action {
            Action::StartTutorial { is_replay } => {
                let mut state = TutorialState::new();
                if is_replay {
                    // For replays, mark as already completed so we know user has seen it
                    state.has_completed = true;
                }
                self.popup = Some(Popup::builder(PopupType::TutorialWizard { state }).build());
                None
            }
            Action::TutorialCompleted => {
                // Persist tutorial completion
                self.tutorial_completed = true;
                self.tutorial_state = None;
                Some(Action::PersistState)
            }
            Action::TutorialSkipped => {
                // Mark as completed so we don't show again on startup
                self.tutorial_completed = true;
                self.tutorial_state = None;
                Some(Action::PersistState)
            }
            Action::TutorialProfileCreated { profile_name } => {
                self.handle_tutorial_profile_created(profile_name);
                None
            }
            Action::TutorialConnectionResult { success } => {
                self.handle_tutorial_connection_result(success);
                None
            }
            Action::LoadSearchScreenForTutorial => {
                self.current_screen = CurrentScreen::Search;
                None
            }
            Action::OpenCreateProfileDialog { from_tutorial } => {
                // Store tutorial state if opening from tutorial
                if from_tutorial {
                    // The popup will be replaced, but we need to remember we're in tutorial mode
                    // The tutorial state should already be in self.tutorial_state
                }
                // Delegate to existing profile dialog handler
                Some(Action::OpenCreateProfileDialog { from_tutorial })
            }
            _ => Some(action),
        }
    }
}
