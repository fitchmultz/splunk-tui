//! Input handling for the forwarders screen.
//!
//! Responsibilities:
//! - Handle keyboard input for the forwarders screen
//! - Dispatch actions based on key presses
//!
//! Does NOT handle:
//! - Rendering (handled by screen module)
//! - Data fetching (handled by side effects)

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::Action;
use crate::app::App;
use crate::app::export::ExportTarget;

impl App {
    /// Handle keyboard input for the forwarders screen.
    ///
    /// # Arguments
    /// * `key` - The key event to process
    ///
    /// # Returns
    /// * `Some(Action)` - Action to execute
    /// * `None` - No action to execute
    pub fn handle_forwarders_input(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            // Navigation
            KeyCode::Down | KeyCode::Char('j') => {
                let next = self.forwarders_state.selected().map(|i| i + 1).unwrap_or(0);
                let max = self.forwarders.as_ref().map(|f| f.len()).unwrap_or(0);
                if next < max {
                    self.forwarders_state.select(Some(next));
                }
                None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let prev = self
                    .forwarders_state
                    .selected()
                    .map(|i| i.saturating_sub(1))
                    .unwrap_or(0);
                self.forwarders_state.select(Some(prev));
                None
            }

            // Refresh
            KeyCode::Char('r') => {
                self.loading = true;
                Some(Action::LoadForwarders {
                    count: self.forwarders_pagination.page_size,
                    offset: 0,
                })
            }

            // Export
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.begin_export(ExportTarget::Forwarders);
                None
            }

            // Load more (if available)
            KeyCode::Char('n') if self.forwarders_pagination.can_load_more() => {
                Some(Action::LoadMoreForwarders)
            }

            _ => None,
        }
    }
}
