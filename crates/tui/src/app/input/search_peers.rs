//! Input handling for the search peers screen.
//!
//! Responsibilities:
//! - Handle keyboard input for the search peers screen
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
    /// Handle keyboard input for the search peers screen.
    ///
    /// # Arguments
    /// * `key` - The key event to process
    ///
    /// # Returns
    /// * `Some(Action)` - Action to execute
    /// * `None` - No action to execute
    pub fn handle_search_peers_input(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            // Navigation
            KeyCode::Down | KeyCode::Char('j') => {
                let next = self
                    .search_peers_state
                    .selected()
                    .map(|i| i + 1)
                    .unwrap_or(0);
                let max = self.search_peers.as_ref().map(|p| p.len()).unwrap_or(0);
                if next < max {
                    self.search_peers_state.select(Some(next));
                }
                None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let prev = self
                    .search_peers_state
                    .selected()
                    .map(|i| i.saturating_sub(1))
                    .unwrap_or(0);
                self.search_peers_state.select(Some(prev));
                None
            }

            // Refresh
            KeyCode::Char('r') => {
                self.loading = true;
                Some(Action::LoadSearchPeers {
                    count: self.search_peers_pagination.page_size,
                    offset: 0,
                })
            }

            // Export
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.begin_export(ExportTarget::SearchPeers);
                None
            }

            // Load more (if available)
            KeyCode::Char('n') if self.search_peers_pagination.can_load_more() => {
                Some(Action::LoadMoreSearchPeers)
            }

            _ => None,
        }
    }
}
