//! Saved searches screen input handler.
//!
//! Responsibilities:
//! - Handle Enter key to run a saved search
//! - Handle Ctrl+C copy of selected saved search name
//! - Handle Ctrl+E export of saved searches list
//! - Handle 'e' key to edit selected saved search
//! - Handle 'n' key to create a new saved search
//! - Handle 'd' key to delete selected saved search
//! - Handle 't' key to toggle enabled/disabled state
//!
//! Does NOT handle:
//! - Does NOT handle global navigation (handled by keymap)
//! - Does NOT render the UI (handled by render module)
//! - Does NOT fetch saved searches data (handled by actions)

use crate::action::Action;
use crate::app::App;
use crate::app::export::ExportTarget;
use crate::app::input::helpers::{
    handle_copy_with_toast, handle_list_export, is_copy_key, is_export_key, should_export_list,
};
use crate::app::state::CurrentScreen;
use crate::ui::Toast;
use crossterm::event::{KeyCode, KeyEvent};
use splunk_client::SearchMode;

impl App {
    /// Handle input for the saved searches screen.
    pub fn handle_saved_searches_input(&mut self, key: KeyEvent) -> Option<Action> {
        if is_copy_key(key) {
            let content = self.saved_searches.as_ref().and_then(|searches| {
                self.saved_searches_state
                    .selected()
                    .and_then(|i| searches.get(i))
                    .map(|s| s.name.clone())
            });

            return handle_copy_with_toast(self, content);
        }

        match key.code {
            KeyCode::Char('e') if is_export_key(key) => {
                let can_export = should_export_list(self.saved_searches.as_ref());
                handle_list_export(self, can_export, ExportTarget::SavedSearches)
            }
            KeyCode::Enter => {
                let query = self.saved_searches.as_ref().and_then(|searches| {
                    self.saved_searches_state.selected().and_then(|selected| {
                        searches.get(selected).map(|search| search.search.clone())
                    })
                });

                if let Some(query) = query {
                    self.search_input.set_value(query.clone());
                    self.current_screen = CurrentScreen::Search;
                    self.add_to_history(query.clone());
                    self.search_status = format!("Running: {}", query);
                    return Some(Action::RunSearch {
                        query,
                        search_defaults: self.search_defaults.clone(),
                        search_mode: SearchMode::Normal,
                        realtime_window: None,
                    });
                }
                None
            }
            // 'n' key: Create new saved search
            KeyCode::Char('n') => Some(Action::OpenCreateSavedSearchDialog),
            // 'd' key: Delete selected saved search (with confirmation)
            KeyCode::Char('d') => {
                if let Some(search) = self.saved_searches.as_ref().and_then(|searches| {
                    self.saved_searches_state
                        .selected()
                        .and_then(|i| searches.get(i))
                }) {
                    return Some(Action::OpenDeleteSavedSearchConfirm {
                        name: search.name.clone(),
                    });
                }
                self.toasts.push(Toast::info("No saved search selected"));
                None
            }
            // 't' key: Toggle enabled/disabled state of selected saved search
            KeyCode::Char('t') => {
                if let Some(search) = self.saved_searches.as_ref().and_then(|searches| {
                    self.saved_searches_state
                        .selected()
                        .and_then(|i| searches.get(i))
                }) {
                    // Toggle the disabled state
                    return Some(Action::ToggleSavedSearch {
                        name: search.name.clone(),
                        disabled: !search.disabled,
                    });
                } else {
                    self.toasts.push(Toast::info("No saved search selected"));
                }
                None
            }
            _ => None,
        }
    }
}
