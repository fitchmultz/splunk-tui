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
                // TODO: Implement search mode
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
                if let Some(ref stanzas) = self.config_stanzas
                    && let Some(selected) = self.config_stanzas_state.selected()
                    && let Some(stanza) = stanzas.get(selected)
                {
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
                if let Some(ref stanzas) = self.config_stanzas
                    && let Some(selected) = self.config_stanzas_state.selected()
                    && let Some(stanza) = stanzas.get(selected)
                {
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
        if let Some(ref stanzas) = self.config_stanzas {
            let i = match self.config_stanzas_state.selected() {
                Some(i) => {
                    if i >= stanzas.len().saturating_sub(1) {
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
        if let Some(ref stanzas) = self.config_stanzas {
            let i = match self.config_stanzas_state.selected() {
                Some(i) => {
                    if i == 0 {
                        stanzas.len().saturating_sub(1)
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.config_stanzas_state.select(Some(i));
        }
    }
}
