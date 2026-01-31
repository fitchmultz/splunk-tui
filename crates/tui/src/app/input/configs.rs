//! Configs screen keyboard input handler.
//!
//! Responsibilities:
//! - Handle keyboard input for the configs screen
//! - Trigger config file and stanza loading
//! - Handle navigation between view modes
//!
//! Does NOT handle:
//! - Direct state modification (returns Actions)
//! - UI rendering

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::Action;
use crate::app::App;
use crate::ui::screens::configs::ConfigViewMode;

impl App {
    /// Handle keyboard input for the configs screen.
    ///
    /// Keybindings:
    /// - 'r' / F5: Refresh configs list
    /// - Enter: Select config file or view stanza details
    /// - 'h' / Left / Esc: Navigate back
    /// - 'j' / Down: Next item
    /// - 'k' / Up: Previous item
    /// - 'y': Copy selected stanza name to clipboard
    /// - '/': Search stanzas
    /// - '?': Show help
    pub fn handle_configs_input(&mut self, key: KeyEvent) -> Option<Action> {
        let view_mode = self.config_view_mode;

        match key.code {
            // Refresh
            KeyCode::Char('r') | KeyCode::F(5) => self.handle_configs_refresh(view_mode),

            // Navigation - Enter
            KeyCode::Enter | KeyCode::Char('l') | KeyCode::Right => {
                self.handle_configs_enter(view_mode)
            }

            // Navigation - Back
            KeyCode::Char('h') | KeyCode::Left | KeyCode::Esc => {
                self.handle_configs_back(view_mode)
            }

            // Navigation - Up/Down
            KeyCode::Up | KeyCode::Char('k') => {
                self.previous_configs_item(view_mode);
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.next_configs_item(view_mode);
                None
            }

            // Copy to clipboard
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.handle_configs_copy(view_mode)
            }

            // Search
            KeyCode::Char('/') => {
                self.enter_config_search_mode();
                None
            }

            // Help
            KeyCode::Char('?') => {
                // TODO: Show help popup
                None
            }

            _ => None,
        }
    }

    /// Handle refresh action based on current view mode.
    fn handle_configs_refresh(&self, view_mode: ConfigViewMode) -> Option<Action> {
        match view_mode {
            ConfigViewMode::FileList => Some(Action::LoadConfigFiles),
            ConfigViewMode::StanzaList | ConfigViewMode::StanzaDetail => self
                .selected_config_file
                .as_ref()
                .map(|config_file| Action::LoadConfigStanzas {
                    config_file: config_file.clone(),
                    count: 100,
                    offset: 0,
                }),
        }
    }

    /// Handle enter action based on current view mode.
    fn handle_configs_enter(&mut self, view_mode: ConfigViewMode) -> Option<Action> {
        match view_mode {
            ConfigViewMode::FileList => {
                // Select a config file and load its stanzas
                if let Some(ref files) = self.config_files
                    && let Some(selected) = self.config_files_state.selected()
                    && let Some(file) = files.get(selected)
                {
                    self.selected_config_file = Some(file.name.clone());
                    self.config_view_mode = ConfigViewMode::StanzaList;
                    return Some(Action::LoadConfigStanzas {
                        config_file: file.name.clone(),
                        count: 100,
                        offset: 0,
                    });
                }
                None
            }
            ConfigViewMode::StanzaList => {
                // Select a stanza and view its details
                if let Some(stanza) = self.get_selected_stanza() {
                    self.selected_stanza = Some(stanza.clone());
                    self.config_view_mode = ConfigViewMode::StanzaDetail;
                }
                None
            }
            ConfigViewMode::StanzaDetail => {
                // Already at detail view, nothing to do
                None
            }
        }
    }

    /// Handle back action based on current view mode.
    fn handle_configs_back(&mut self, view_mode: ConfigViewMode) -> Option<Action> {
        match view_mode {
            ConfigViewMode::FileList => {
                // At top level, go to search screen (home)
                self.current_screen = crate::app::state::CurrentScreen::Search;
                None
            }
            ConfigViewMode::StanzaList => {
                // Go back to file list
                self.config_view_mode = ConfigViewMode::FileList;
                self.selected_config_file = None;
                self.config_stanzas = None;
                None
            }
            ConfigViewMode::StanzaDetail => {
                // Go back to stanza list
                self.config_view_mode = ConfigViewMode::StanzaList;
                self.selected_stanza = None;
                None
            }
        }
    }

    /// Handle copy action based on current view mode.
    fn handle_configs_copy(&self, view_mode: ConfigViewMode) -> Option<Action> {
        match view_mode {
            ConfigViewMode::FileList => {
                if let Some(ref files) = self.config_files
                    && let Some(selected) = self.config_files_state.selected()
                    && let Some(file) = files.get(selected)
                {
                    return Some(Action::CopyToClipboard(file.name.clone()));
                }
                None
            }
            ConfigViewMode::StanzaList => {
                if let Some(stanza) = self.get_selected_stanza() {
                    return Some(Action::CopyToClipboard(stanza.name.clone()));
                }
                None
            }
            ConfigViewMode::StanzaDetail => {
                if let Some(ref stanza) = self.selected_stanza {
                    return Some(Action::CopyToClipboard(stanza.name.clone()));
                }
                None
            }
        }
    }

    /// Move to the next item based on view mode.
    fn next_configs_item(&mut self, view_mode: ConfigViewMode) {
        match view_mode {
            ConfigViewMode::FileList => {
                self.next_config_file();
            }
            ConfigViewMode::StanzaList => {
                self.next_config_stanza();
            }
            ConfigViewMode::StanzaDetail => {
                // No navigation in detail view
            }
        }
    }

    /// Move to the previous item based on view mode.
    fn previous_configs_item(&mut self, view_mode: ConfigViewMode) {
        match view_mode {
            ConfigViewMode::FileList => {
                self.previous_config_file();
            }
            ConfigViewMode::StanzaList => {
                self.previous_config_stanza();
            }
            ConfigViewMode::StanzaDetail => {
                // No navigation in detail view
            }
        }
    }

    /// Move to the next config file.
    fn next_config_file(&mut self) {
        if let Some(ref files) = self.config_files {
            let i = match self.config_files_state.selected() {
                Some(i) => {
                    if i >= files.len().saturating_sub(1) {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.config_files_state.select(Some(i));
        }
    }

    /// Move to the previous config file.
    fn previous_config_file(&mut self) {
        if let Some(ref files) = self.config_files {
            let i = match self.config_files_state.selected() {
                Some(i) => {
                    if i == 0 {
                        files.len().saturating_sub(1)
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.config_files_state.select(Some(i));
        }
    }

    /// Move to the next config stanza.
    fn next_config_stanza(&mut self) {
        let count = if self.config_search_query.is_empty() {
            self.config_stanzas.as_ref().map(|s| s.len()).unwrap_or(0)
        } else {
            self.filtered_stanza_indices.len()
        };

        if count > 0 {
            let i = match self.config_stanzas_state.selected() {
                Some(i) => {
                    if i >= count.saturating_sub(1) {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.config_stanzas_state.select(Some(i));
        }
    }

    /// Move to the previous config stanza.
    fn previous_config_stanza(&mut self) {
        let count = if self.config_search_query.is_empty() {
            self.config_stanzas.as_ref().map(|s| s.len()).unwrap_or(0)
        } else {
            self.filtered_stanza_indices.len()
        };

        if count > 0 {
            let i = match self.config_stanzas_state.selected() {
                Some(i) => {
                    if i == 0 {
                        count.saturating_sub(1)
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.config_stanzas_state.select(Some(i));
        }
    }

    /// Enter search mode for config stanzas.
    fn enter_config_search_mode(&mut self) {
        // Only allow search in StanzaList view mode
        if self.config_view_mode != ConfigViewMode::StanzaList {
            return;
        }
        self.config_search_mode = true;
        self.config_search_before_edit = Some(self.config_search_query.clone());
    }

    /// Handle keyboard input when in config search mode.
    pub(crate) fn handle_config_search_input(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Esc => {
                self.config_search_mode = false;
                // Restore previous search query if canceling edit
                if let Some(saved) = self.config_search_before_edit.take() {
                    self.config_search_query = saved;
                    self.rebuild_filtered_stanza_indices();
                }
                None
            }
            KeyCode::Enter => {
                self.config_search_mode = false;
                self.config_search_before_edit = None;
                if !self.config_search_query.is_empty() {
                    self.rebuild_filtered_stanza_indices();
                } else {
                    self.clear_config_search();
                }
                None
            }
            KeyCode::Backspace => {
                self.config_search_query.pop();
                None
            }
            KeyCode::Char(c) => {
                self.config_search_query.push(c);
                None
            }
            _ => None,
        }
    }

    /// Clear the config search filter.
    fn clear_config_search(&mut self) {
        self.config_search_query.clear();
        self.filtered_stanza_indices.clear();
    }

    /// Rebuild filtered stanza indices based on current search query.
    /// Searches stanza names and settings (key=value pairs).
    pub(crate) fn rebuild_filtered_stanza_indices(&mut self) {
        let Some(stanzas) = &self.config_stanzas else {
            self.filtered_stanza_indices.clear();
            return;
        };

        if self.config_search_query.is_empty() {
            self.filtered_stanza_indices = (0..stanzas.len()).collect();
        } else {
            let lower_query = self.config_search_query.to_lowercase();
            self.filtered_stanza_indices = stanzas
                .iter()
                .enumerate()
                .filter(|(_, stanza)| {
                    // Search in stanza name
                    if stanza.name.to_lowercase().contains(&lower_query) {
                        return true;
                    }
                    // Search in settings (key=value pairs)
                    stanza.settings.iter().any(|(key, value)| {
                        key.to_lowercase().contains(&lower_query)
                            || value.to_string().to_lowercase().contains(&lower_query)
                    })
                })
                .map(|(i, _)| i)
                .collect();
        }

        // Clamp selection to filtered list length
        let filtered_len = self.filtered_stanza_indices.len();
        if let Some(selected) = self.config_stanzas_state.selected() {
            if filtered_len == 0 {
                self.config_stanzas_state.select(None);
            } else if selected >= filtered_len {
                self.config_stanzas_state.select(Some(filtered_len - 1));
            }
        }
    }

    /// Get the currently selected stanza, accounting for any active search filter.
    pub fn get_selected_stanza(&self) -> Option<&splunk_client::models::ConfigStanza> {
        let selected = self.config_stanzas_state.selected()?;
        let original_idx = self.filtered_stanza_indices.get(selected)?;
        self.config_stanzas.as_ref()?.get(*original_idx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crate::app::ConnectionContext;
    use crate::app::state::CurrentScreen;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use splunk_client::models::ConfigStanza;

    fn create_test_app() -> App {
        App::new(None, ConnectionContext::default())
    }

    fn create_test_stanzas() -> Vec<ConfigStanza> {
        use serde_json::Value;
        use std::collections::HashMap;

        vec![
            ConfigStanza {
                name: "stanza1".to_string(),
                config_file: "props".to_string(),
                settings: {
                    let mut m = HashMap::new();
                    m.insert("KEY1".to_string(), Value::String("value1".to_string()));
                    m
                },
            },
            ConfigStanza {
                name: "test_stanza".to_string(),
                config_file: "props".to_string(),
                settings: {
                    let mut m = HashMap::new();
                    m.insert("OTHER".to_string(), Value::String("searchable".to_string()));
                    m
                },
            },
            ConfigStanza {
                name: "another".to_string(),
                config_file: "props".to_string(),
                settings: {
                    let mut m = HashMap::new();
                    m.insert(
                        "MATCHING_KEY".to_string(),
                        Value::String("some_value".to_string()),
                    );
                    m.insert(
                        "OTHER_KEY".to_string(),
                        Value::String("another_value".to_string()),
                    );
                    m
                },
            },
        ]
    }

    #[test]
    fn test_enter_search_mode_only_in_stanza_list() {
        let mut app = create_test_app();

        // Should not enter search mode in FileList view
        app.config_view_mode = ConfigViewMode::FileList;
        app.enter_config_search_mode();
        assert!(!app.config_search_mode);

        // Should enter search mode in StanzaList view
        app.config_view_mode = ConfigViewMode::StanzaList;
        app.enter_config_search_mode();
        assert!(app.config_search_mode);
        assert_eq!(app.config_search_before_edit, Some(String::new()));

        // Should not enter search mode in StanzaDetail view
        app.config_search_mode = false;
        app.config_view_mode = ConfigViewMode::StanzaDetail;
        app.enter_config_search_mode();
        assert!(!app.config_search_mode);
    }

    #[test]
    fn test_search_input_typing() {
        let mut app = create_test_app();
        app.config_view_mode = ConfigViewMode::StanzaList;
        app.enter_config_search_mode();

        // Type 't'
        let key = KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE);
        app.handle_config_search_input(key);
        assert_eq!(app.config_search_query, "t");

        // Type 'e'
        let key = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE);
        app.handle_config_search_input(key);
        assert_eq!(app.config_search_query, "te");

        // Backspace
        let key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
        app.handle_config_search_input(key);
        assert_eq!(app.config_search_query, "t");
    }

    #[test]
    fn test_search_apply_with_enter() {
        let mut app = create_test_app();
        app.config_view_mode = ConfigViewMode::StanzaList;
        app.config_stanzas = Some(create_test_stanzas());
        app.enter_config_search_mode();

        // Type 'test'
        for c in "test".chars() {
            let key = KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE);
            app.handle_config_search_input(key);
        }

        // Press Enter to apply
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        app.handle_config_search_input(key);

        assert!(!app.config_search_mode);
        assert_eq!(app.config_search_query, "test");
        assert_eq!(app.filtered_stanza_indices.len(), 1);
        assert_eq!(app.filtered_stanza_indices[0], 1); // test_stanza
    }

    #[test]
    fn test_search_cancel_with_esc() {
        let mut app = create_test_app();
        app.config_view_mode = ConfigViewMode::StanzaList;
        app.config_stanzas = Some(create_test_stanzas());
        app.config_search_query = "existing".to_string();
        app.rebuild_filtered_stanza_indices();

        app.enter_config_search_mode();

        // Clear and type 'newquery' (simulating user clearing and typing new)
        app.config_search_query.clear();
        for c in "newquery".chars() {
            let key = KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE);
            app.handle_config_search_input(key);
        }
        assert_eq!(app.config_search_query, "newquery");

        // Press Esc to cancel
        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        app.handle_config_search_input(key);

        assert!(!app.config_search_mode);
        assert_eq!(app.config_search_query, "existing"); // Restored to previous
    }

    #[test]
    fn test_filter_by_name() {
        let mut app = create_test_app();
        app.config_stanzas = Some(create_test_stanzas());

        app.config_search_query = "stanza1".to_string();
        app.rebuild_filtered_stanza_indices();

        assert_eq!(app.filtered_stanza_indices.len(), 1);
        assert_eq!(app.filtered_stanza_indices[0], 0);
    }

    #[test]
    fn test_filter_by_name_case_insensitive() {
        let mut app = create_test_app();
        app.config_stanzas = Some(create_test_stanzas());

        app.config_search_query = "TEST".to_string();
        app.rebuild_filtered_stanza_indices();

        assert_eq!(app.filtered_stanza_indices.len(), 1);
        assert_eq!(app.filtered_stanza_indices[0], 1);
    }

    #[test]
    fn test_filter_by_setting_value() {
        let mut app = create_test_app();
        app.config_stanzas = Some(create_test_stanzas());

        app.config_search_query = "searchable".to_string();
        app.rebuild_filtered_stanza_indices();

        assert_eq!(app.filtered_stanza_indices.len(), 1);
        assert_eq!(app.filtered_stanza_indices[0], 1);
    }

    #[test]
    fn test_filter_by_setting_key() {
        let mut app = create_test_app();
        app.config_stanzas = Some(create_test_stanzas());

        app.config_search_query = "MATCHING".to_string();
        app.rebuild_filtered_stanza_indices();

        assert_eq!(app.filtered_stanza_indices.len(), 1);
        assert_eq!(app.filtered_stanza_indices[0], 2);
    }

    #[test]
    fn test_empty_search_shows_all() {
        let mut app = create_test_app();
        app.config_stanzas = Some(create_test_stanzas());

        app.config_search_query = "".to_string();
        app.rebuild_filtered_stanza_indices();

        assert_eq!(app.filtered_stanza_indices.len(), 3);
        assert_eq!(app.filtered_stanza_indices, vec![0, 1, 2]);
    }

    #[test]
    fn test_no_match_search() {
        let mut app = create_test_app();
        app.config_stanzas = Some(create_test_stanzas());

        app.config_search_query = "nonexistent".to_string();
        app.rebuild_filtered_stanza_indices();

        assert!(app.filtered_stanza_indices.is_empty());
    }

    #[test]
    fn test_get_selected_stanza_with_filter() {
        let mut app = create_test_app();
        app.config_stanzas = Some(create_test_stanzas());
        app.config_stanzas_state.select(Some(0));

        // Initialize filtered indices (normally done when stanzas are loaded)
        app.rebuild_filtered_stanza_indices();

        // Without filter, should return first stanza
        let stanza = app.get_selected_stanza();
        assert_eq!(stanza.unwrap().name, "stanza1");

        // With filter matching test_stanza
        app.config_search_query = "test".to_string();
        app.rebuild_filtered_stanza_indices();
        app.config_stanzas_state.select(Some(0));

        let stanza = app.get_selected_stanza();
        assert_eq!(stanza.unwrap().name, "test_stanza");
    }

    #[test]
    fn test_navigation_with_filtered_results() {
        let mut app = create_test_app();
        app.config_stanzas = Some(create_test_stanzas());
        app.config_stanzas_state.select(Some(0));

        // Filter to only show stanza at index 1 (test_stanza)
        app.config_search_query = "test_stanza".to_string();
        app.rebuild_filtered_stanza_indices();

        // Navigation should work with filtered count
        app.next_config_stanza();
        // Should wrap to 0 since there's only 1 filtered result
        assert_eq!(app.config_stanzas_state.selected(), Some(0));

        app.previous_config_stanza();
        // Should still be 0
        assert_eq!(app.config_stanzas_state.selected(), Some(0));
    }

    #[test]
    fn test_clear_config_search() {
        let mut app = create_test_app();
        app.config_stanzas = Some(create_test_stanzas());
        app.config_search_query = "test".to_string();
        app.rebuild_filtered_stanza_indices();

        app.clear_config_search();

        assert!(app.config_search_query.is_empty());
        assert!(app.filtered_stanza_indices.is_empty());
    }

    #[test]
    fn test_slash_keybinding_enters_search_mode() {
        let mut app = create_test_app();
        app.current_screen = CurrentScreen::Configs;
        app.config_view_mode = ConfigViewMode::StanzaList;

        let key = KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE);
        app.handle_configs_input(key);

        assert!(app.config_search_mode);
    }
}
