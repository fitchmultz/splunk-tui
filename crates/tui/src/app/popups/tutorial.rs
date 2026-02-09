//! Tutorial wizard popup handler.

use crate::action::Action;
use crate::app::App;
use crate::onboarding::TutorialStep;
use crate::ui::Toast;
use crate::ui::popup::{Popup, PopupType};
use crossterm::event::{KeyCode, KeyEvent};

impl App {
    /// Handle input for the tutorial wizard popup.
    pub fn handle_tutorial_popup(&mut self, key: KeyEvent) -> Option<Action> {
        let popup_type = self.popup.as_ref().map(|p| &p.kind);

        let Some(PopupType::TutorialWizard { state }) = popup_type else {
            return None;
        };

        let mut tutorial_state = state.clone();

        match key.code {
            // Close tutorial on Escape (mark as skipped, not completed)
            KeyCode::Esc => {
                self.popup = None;
                Some(Action::TutorialSkipped)
            }

            // Advance to next step or complete
            KeyCode::Enter => {
                match tutorial_state.current_step {
                    TutorialStep::Welcome => {
                        tutorial_state.next_step();
                        self.popup = Some(
                            Popup::builder(PopupType::TutorialWizard {
                                state: tutorial_state,
                            })
                            .build(),
                        );
                        None
                    }
                    TutorialStep::ProfileCreation => {
                        // Close tutorial popup and open profile creation
                        self.popup = None;
                        Some(Action::OpenCreateProfileDialog {
                            from_tutorial: true,
                        })
                    }
                    TutorialStep::ConnectionTest => {
                        // Should not be editable - this step is for display only
                        None
                    }
                    TutorialStep::FirstSearch => {
                        // Navigate to search screen
                        self.popup = None;
                        self.current_screen = crate::app::CurrentScreen::Search;
                        tutorial_state.next_step();
                        // Store tutorial state to continue after search
                        self.tutorial_state = Some(tutorial_state);
                        Some(Action::LoadSearchScreenForTutorial)
                    }
                    TutorialStep::KeybindingTutorial => {
                        tutorial_state.next_step();
                        self.popup = Some(
                            Popup::builder(PopupType::TutorialWizard {
                                state: tutorial_state,
                            })
                            .build(),
                        );
                        None
                    }
                    TutorialStep::ExportDemo => {
                        tutorial_state.next_step();
                        self.popup = Some(
                            Popup::builder(PopupType::TutorialWizard {
                                state: tutorial_state,
                            })
                            .build(),
                        );
                        None
                    }
                    TutorialStep::Complete => {
                        // Close tutorial and mark as completed
                        self.popup = None;
                        tutorial_state.complete();
                        Some(Action::TutorialCompleted)
                    }
                }
            }

            // Go back to previous step
            KeyCode::Left | KeyCode::BackTab => {
                if tutorial_state.previous_step() {
                    self.popup = Some(
                        Popup::builder(PopupType::TutorialWizard {
                            state: tutorial_state,
                        })
                        .build(),
                    );
                }
                None
            }

            // Skip to next step (for optional steps)
            KeyCode::Tab => {
                if tutorial_state.current_step == TutorialStep::ProfileCreation {
                    // Skip profile creation
                    tutorial_state.next_step();
                    self.popup = Some(
                        Popup::builder(PopupType::TutorialWizard {
                            state: tutorial_state,
                        })
                        .build(),
                    );
                }
                None
            }

            // Scroll keybinding help
            KeyCode::Up | KeyCode::Char('k') => {
                if tutorial_state.current_step == TutorialStep::KeybindingTutorial {
                    tutorial_state.keybinding_scroll_offset =
                        tutorial_state.keybinding_scroll_offset.saturating_sub(1);
                    self.popup = Some(
                        Popup::builder(PopupType::TutorialWizard {
                            state: tutorial_state,
                        })
                        .build(),
                    );
                }
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if tutorial_state.current_step == TutorialStep::KeybindingTutorial {
                    tutorial_state.keybinding_scroll_offset =
                        tutorial_state.keybinding_scroll_offset.saturating_add(1);
                    self.popup = Some(
                        Popup::builder(PopupType::TutorialWizard {
                            state: tutorial_state,
                        })
                        .build(),
                    );
                }
                None
            }
            KeyCode::PageUp => {
                if tutorial_state.current_step == TutorialStep::KeybindingTutorial {
                    tutorial_state.keybinding_scroll_offset =
                        tutorial_state.keybinding_scroll_offset.saturating_sub(10);
                    self.popup = Some(
                        Popup::builder(PopupType::TutorialWizard {
                            state: tutorial_state,
                        })
                        .build(),
                    );
                }
                None
            }
            KeyCode::PageDown => {
                if tutorial_state.current_step == TutorialStep::KeybindingTutorial {
                    tutorial_state.keybinding_scroll_offset =
                        tutorial_state.keybinding_scroll_offset.saturating_add(10);
                    self.popup = Some(
                        Popup::builder(PopupType::TutorialWizard {
                            state: tutorial_state,
                        })
                        .build(),
                    );
                }
                None
            }

            // Space to continue (like Enter for most steps)
            KeyCode::Char(' ') => {
                if tutorial_state.current_step == TutorialStep::KeybindingTutorial {
                    tutorial_state.next_step();
                    self.popup = Some(
                        Popup::builder(PopupType::TutorialWizard {
                            state: tutorial_state,
                        })
                        .build(),
                    );
                }
                None
            }

            _ => None,
        }
    }

    /// Update tutorial state after profile creation.
    pub fn handle_tutorial_profile_created(&mut self, profile_name: String) {
        if let Some(ref mut state) = self.tutorial_state {
            state.pending_profile_name = Some(profile_name);
            state.connection_test_result = None;
            state.next_step(); // Move to ConnectionTest

            // Re-open tutorial with updated state
            self.popup = Some(
                Popup::builder(PopupType::TutorialWizard {
                    state: state.clone(),
                })
                .build(),
            );
        }
    }

    /// Update tutorial state after connection test.
    pub fn handle_tutorial_connection_result(&mut self, success: bool) {
        if let Some(ref mut state) = self.tutorial_state {
            state.connection_test_result = Some(success);
            if success {
                state.next_step(); // Auto-advance to FirstSearch on success
                self.toasts.push(Toast::info(
                    "Connection successful! Continue with the tutorial.".to_string(),
                ));
            } else {
                self.toasts.push(Toast::warning(
                    "Connection failed. You can retry or skip to continue.".to_string(),
                ));
            }

            // Update popup with result
            self.popup = Some(
                Popup::builder(PopupType::TutorialWizard {
                    state: state.clone(),
                })
                .build(),
            );
        }
    }
}
