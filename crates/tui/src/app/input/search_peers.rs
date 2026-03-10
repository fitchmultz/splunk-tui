//! Input handling for the search peers screen.
//!
//! Responsibilities:
//! - Handle keyboard input for the search peers screen
//! - Dispatch actions based on key presses
//!
//! Does NOT handle:
//! - Rendering (handled by screen module)
//! - Data fetching (handled by side effects)

use crossterm::event::{KeyCode, KeyEvent};

use crate::action::Action;
use crate::app::App;
use crate::app::export::ExportTarget;
use crate::app::input::helpers::{handle_list_export, is_export_key, should_export_list};

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
                self.next_item();
                None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.previous_item();
                None
            }
            KeyCode::PageDown => {
                self.next_page();
                None
            }
            KeyCode::PageUp => {
                self.previous_page();
                None
            }
            KeyCode::Home => {
                self.go_to_top();
                None
            }
            KeyCode::End => {
                self.go_to_bottom();
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
            KeyCode::Char('e') if is_export_key(key) => {
                let can_export = should_export_list(self.search_peers.as_ref());
                handle_list_export(self, can_export, ExportTarget::SearchPeers)
            }

            // Load more (if available)
            KeyCode::Char('n') if self.search_peers_pagination.can_load_more() => {
                Some(Action::LoadMoreSearchPeers)
            }

            _ => None,
        }
    }
}
