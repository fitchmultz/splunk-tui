//! Saved searches screen input handler.
//!
//! Responsibilities:
//! - Handle Enter key to run a saved search
//! - Handle Ctrl+C copy of selected saved search name
//! - Handle Ctrl+E export of saved searches list
//! - Handle 'e' key to edit selected saved search
//!
//! Non-responsibilities:
//! - Does NOT handle global navigation (handled by keymap)
//! - Does NOT render the UI (handled by render module)
//! - Does NOT fetch saved searches data (handled by actions)

use crate::action::Action;
use crate::app::App;
use crate::app::export::ExportTarget;
use crate::app::state::CurrentScreen;
use crate::ui::Toast;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use splunk_client::SearchMode;

impl App {
    /// Handle input for the saved searches screen.
    pub fn handle_saved_searches_input(&mut self, key: KeyEvent) -> Option<Action> {
        // Ctrl+C: copy selected saved search name
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
            let content = self.saved_searches.as_ref().and_then(|searches| {
                self.saved_searches_state
                    .selected()
                    .and_then(|i| searches.get(i))
                    .map(|s| s.name.clone())
            });

            if let Some(content) = content.filter(|s| !s.trim().is_empty()) {
                return Some(Action::CopyToClipboard(content));
            }

            self.toasts.push(Toast::info("Nothing to copy"));
            return None;
        }

        match key.code {
            KeyCode::Char('e')
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && self
                        .saved_searches
                        .as_ref()
                        .map(|v| !v.is_empty())
                        .unwrap_or(false) =>
            {
                self.begin_export(ExportTarget::SavedSearches);
                None
            }
            KeyCode::Enter => {
                let query = self.saved_searches.as_ref().and_then(|searches| {
                    self.saved_searches_state.selected().and_then(|selected| {
                        searches.get(selected).map(|search| search.search.clone())
                    })
                });

                if let Some(query) = query {
                    self.search_input = query.clone();
                    self.search_cursor_position = query.len();
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
            _ => None,
        }
    }
}
