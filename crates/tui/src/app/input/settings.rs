//! Settings screen input handler.
//!
//! Responsibilities:
//! - Handle 'a' key to toggle auto-refresh
//! - Handle 's' key to cycle sort column
//! - Handle 'd' key to toggle sort direction
//! - Handle 'c' key to clear search history
//!
//! Non-responsibilities:
//! - Does NOT handle global navigation (handled by keymap)
//! - Does NOT render the UI (handled by render module)
//! - Does NOT persist settings (handled by actions)

use crate::action::Action;
use crate::app::App;
use crate::app::state::{SortColumn, SortDirection};
use crate::ui::Toast;
use crossterm::event::{KeyCode, KeyEvent};

impl App {
    /// Handle input for the settings screen.
    pub fn handle_settings_input(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Char('a') => {
                self.auto_refresh = !self.auto_refresh;
                self.toasts.push(Toast::info(format!(
                    "Auto-refresh: {}",
                    if self.auto_refresh { "On" } else { "Off" }
                )));
                None
            }
            KeyCode::Char('s') => {
                self.sort_state.column = match self.sort_state.column {
                    SortColumn::Sid => SortColumn::Status,
                    SortColumn::Status => SortColumn::Duration,
                    SortColumn::Duration => SortColumn::Results,
                    SortColumn::Results => SortColumn::Events,
                    SortColumn::Events => SortColumn::Sid,
                };
                self.toasts.push(Toast::info(format!(
                    "Sort column: {}",
                    self.sort_state.column.as_str()
                )));
                None
            }
            KeyCode::Char('d') => {
                self.sort_state.direction = match self.sort_state.direction {
                    SortDirection::Asc => SortDirection::Desc,
                    SortDirection::Desc => SortDirection::Asc,
                };
                self.toasts.push(Toast::info(format!(
                    "Sort direction: {}",
                    self.sort_state.direction.as_str()
                )));
                None
            }
            KeyCode::Char('c') => {
                self.search_history.clear();
                self.toasts.push(Toast::info("Search history cleared"));
                None
            }
            KeyCode::Char('e') => {
                // Open edit profile dialog for current profile (if selected)
                if let Some(profile_name) = &self.profile_name {
                    Some(Action::OpenEditProfileDialog {
                        name: profile_name.clone(),
                    })
                } else {
                    self.toasts.push(crate::ui::Toast::warning(
                        "No profile selected to edit. Use 'p' to switch to a profile first.",
                    ));
                    None
                }
            }
            KeyCode::Char('x') => {
                // Open delete confirmation for current profile (if selected)
                if let Some(profile_name) = &self.profile_name {
                    Some(Action::OpenDeleteProfileConfirm {
                        name: profile_name.clone(),
                    })
                } else {
                    self.toasts.push(crate::ui::Toast::warning(
                        "No profile selected to delete. Use 'p' to switch to a profile first.",
                    ));
                    None
                }
            }
            _ => None,
        }
    }
}
