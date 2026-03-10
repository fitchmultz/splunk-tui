//! Saved search popup handlers.
//!
//! Responsibilities:
//! - Handle saved search editing popup
//! - Form navigation and input handling
//!
//! Does NOT handle:
//! - Does NOT render popups (handled by ui::popup module)
//! - Does NOT execute the actual saved search operations (just returns Action variants)

use crate::action::Action;
use crate::app::App;
use crate::ui::popup::{PopupType, SavedSearchField};
use crate::undo::{SavedSearchRecoveryData, UndoableOperation};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::common::optional_string;

impl App {
    /// Handle saved search-related popups (EditSavedSearch).
    pub fn handle_saved_search_popup(&mut self, key: KeyEvent) -> Option<Action> {
        match (
            self.popup.as_ref().map(|popup| popup.kind.clone()),
            key.code,
        ) {
            (
                Some(PopupType::EditSavedSearch {
                    search_name,
                    search_input,
                    description_input,
                    disabled,
                    ..
                }),
                KeyCode::Enter,
            ) => {
                self.popup = None;
                Some(Action::UpdateSavedSearch {
                    name: search_name,
                    search: optional_string(search_input),
                    description: optional_string(description_input),
                    disabled: Some(disabled),
                })
            }
            (Some(PopupType::EditSavedSearch { .. }), KeyCode::Esc) => {
                self.popup = None;
                None
            }
            (
                Some(PopupType::CreateSavedSearch {
                    name_input,
                    search_input,
                    description_input,
                    disabled,
                    ..
                }),
                KeyCode::Enter,
            ) => {
                if name_input.is_empty() || search_input.is_empty() {
                    return None;
                }

                self.popup = None;
                Some(Action::CreateSavedSearch {
                    name: name_input,
                    search: search_input,
                    description: optional_string(description_input),
                    disabled,
                })
            }
            (Some(PopupType::CreateSavedSearch { .. }), KeyCode::Esc) => {
                self.popup = None;
                None
            }
            (Some(mut kind @ PopupType::EditSavedSearch { .. }), KeyCode::Tab)
            | (Some(mut kind @ PopupType::CreateSavedSearch { .. }), KeyCode::Tab) => {
                kind.navigate_fields(key.modifiers.contains(KeyModifiers::SHIFT));
                self.replace_popup_kind(kind);
                None
            }
            (Some(mut kind @ PopupType::EditSavedSearch { .. }), KeyCode::Up)
            | (Some(mut kind @ PopupType::CreateSavedSearch { .. }), KeyCode::Up) => {
                kind.navigate_fields(true);
                self.replace_popup_kind(kind);
                None
            }
            (Some(mut kind @ PopupType::EditSavedSearch { .. }), KeyCode::Down)
            | (Some(mut kind @ PopupType::CreateSavedSearch { .. }), KeyCode::Down) => {
                kind.navigate_fields(false);
                self.replace_popup_kind(kind);
                None
            }
            (Some(mut kind @ PopupType::EditSavedSearch { .. }), KeyCode::Char(c))
            | (Some(mut kind @ PopupType::CreateSavedSearch { .. }), KeyCode::Char(c)) => {
                if update_saved_search_char(&mut kind, c) {
                    self.replace_popup_kind(kind);
                }
                None
            }
            (Some(mut kind @ PopupType::EditSavedSearch { .. }), KeyCode::Backspace)
            | (Some(mut kind @ PopupType::CreateSavedSearch { .. }), KeyCode::Backspace) => {
                if update_saved_search_backspace(&mut kind) {
                    self.replace_popup_kind(kind);
                }
                None
            }
            (
                Some(PopupType::DeleteSavedSearchConfirm { search_name }),
                KeyCode::Char('y') | KeyCode::Enter,
            ) => {
                let name = search_name.clone();
                let description = format!("Delete saved search '{}'", search_name);
                self.popup = None;
                // Try to capture original saved search data for recovery
                let original = self.get_saved_search_recovery_data(&name);
                Some(Action::QueueUndoableOperation {
                    operation: UndoableOperation::DeleteSavedSearch { name, original },
                    description,
                })
            }
            // DeleteSavedSearchConfirm - cancel
            (
                Some(PopupType::DeleteSavedSearchConfirm { .. }),
                KeyCode::Char('n') | KeyCode::Esc,
            ) => {
                self.popup = None;
                None
            }
            // Default: ignore other keys
            _ => None,
        }
    }

    /// Get saved search recovery data for undo.
    ///
    /// Attempts to find the saved search in the currently loaded list
    /// and extract its data for potential recovery.
    fn get_saved_search_recovery_data(&self, name: &str) -> Option<SavedSearchRecoveryData> {
        self.saved_searches.as_ref().and_then(|searches| {
            searches
                .iter()
                .find(|s| s.name == name)
                .map(|s| SavedSearchRecoveryData {
                    search: s.search.clone(),
                    description: s.description.clone(),
                    disabled: s.disabled,
                })
        })
    }
}

fn update_saved_search_char(kind: &mut PopupType, c: char) -> bool {
    match kind {
        PopupType::EditSavedSearch {
            search_input,
            description_input,
            disabled,
            selected_field,
            ..
        } => {
            match selected_field {
                SavedSearchField::Name => return false,
                SavedSearchField::Search => search_input.push(c),
                SavedSearchField::Description => description_input.push(c),
                SavedSearchField::Disabled if c == ' ' => *disabled = !*disabled,
                _ => return false,
            }
            true
        }
        PopupType::CreateSavedSearch {
            name_input,
            search_input,
            description_input,
            disabled,
            selected_field,
        } => {
            match selected_field {
                SavedSearchField::Name => name_input.push(c),
                SavedSearchField::Search => search_input.push(c),
                SavedSearchField::Description => description_input.push(c),
                SavedSearchField::Disabled if c == ' ' => *disabled = !*disabled,
                _ => return false,
            }
            true
        }
        _ => false,
    }
}

fn update_saved_search_backspace(kind: &mut PopupType) -> bool {
    match kind {
        PopupType::EditSavedSearch {
            search_input,
            description_input,
            selected_field,
            ..
        } => match selected_field {
            SavedSearchField::Name | SavedSearchField::Disabled => false,
            SavedSearchField::Search => {
                search_input.pop();
                true
            }
            SavedSearchField::Description => {
                description_input.pop();
                true
            }
        },
        PopupType::CreateSavedSearch {
            name_input,
            search_input,
            description_input,
            selected_field,
            ..
        } => match selected_field {
            SavedSearchField::Name => {
                name_input.pop();
                true
            }
            SavedSearchField::Search => {
                search_input.pop();
                true
            }
            SavedSearchField::Description => {
                description_input.pop();
                true
            }
            SavedSearchField::Disabled => false,
        },
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConnectionContext;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(c: KeyCode) -> KeyEvent {
        KeyEvent::new(c, KeyModifiers::empty())
    }

    fn shift_key(c: KeyCode) -> KeyEvent {
        KeyEvent::new(c, KeyModifiers::SHIFT)
    }

    fn char_key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty())
    }

    #[test]
    fn test_edit_saved_search_close() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::EditSavedSearch {
                search_name: "test-search".to_string(),
                search_input: String::new(),
                description_input: String::new(),
                disabled: false,
                selected_field: SavedSearchField::Search,
            })
            .build(),
        );

        // Close with Esc
        let action = app.handle_popup_input(key(KeyCode::Esc));
        assert!(action.is_none());
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_edit_saved_search_tab_navigation() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::EditSavedSearch {
                search_name: "test-search".to_string(),
                search_input: String::new(),
                description_input: String::new(),
                disabled: false,
                selected_field: SavedSearchField::Search,
            })
            .build(),
        );

        // Tab to next field
        let action = app.handle_popup_input(key(KeyCode::Tab));
        assert!(action.is_none());
        assert!(matches!(
            app.popup,
            Some(Popup {
                kind: PopupType::EditSavedSearch {
                    selected_field: SavedSearchField::Description,
                    ..
                },
                ..
            })
        ));

        // Tab again to Disabled
        let action = app.handle_popup_input(key(KeyCode::Tab));
        assert!(action.is_none());
        assert!(matches!(
            app.popup,
            Some(Popup {
                kind: PopupType::EditSavedSearch {
                    selected_field: SavedSearchField::Disabled,
                    ..
                },
                ..
            })
        ));

        // Tab again goes to Name (internal field, not displayed in EditSavedSearch)
        let action = app.handle_popup_input(key(KeyCode::Tab));
        assert!(action.is_none());
        assert!(matches!(
            app.popup,
            Some(Popup {
                kind: PopupType::EditSavedSearch {
                    selected_field: SavedSearchField::Name,
                    ..
                },
                ..
            })
        ));

        // Tab again wraps back to Search
        let action = app.handle_popup_input(key(KeyCode::Tab));
        assert!(action.is_none());
        assert!(matches!(
            app.popup,
            Some(Popup {
                kind: PopupType::EditSavedSearch {
                    selected_field: SavedSearchField::Search,
                    ..
                },
                ..
            })
        ));
    }

    #[test]
    fn test_edit_saved_search_shift_tab_navigation() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::EditSavedSearch {
                search_name: "test-search".to_string(),
                search_input: String::new(),
                description_input: String::new(),
                disabled: false,
                selected_field: SavedSearchField::Search,
            })
            .build(),
        );

        // Shift+Tab to previous field
        // Note: Navigation wraps through Name field internally. From Search,
        // previous() goes to Name, then previous from Name goes to Disabled.
        // Since EditSavedSearch doesn't have Name field, we need two Shift+Tabs
        // to get from Search to Disabled.
        let action = app.handle_popup_input(shift_key(KeyCode::Tab));
        assert!(action.is_none());
        // First Shift+Tab goes to Name (not visible in EditSavedSearch)
        let action = app.handle_popup_input(shift_key(KeyCode::Tab));
        assert!(action.is_none());
        // Second Shift+Tab goes to Disabled
        assert!(matches!(
            app.popup,
            Some(Popup {
                kind: PopupType::EditSavedSearch {
                    selected_field: SavedSearchField::Disabled,
                    ..
                },
                ..
            })
        ));
    }

    #[test]
    fn test_edit_saved_search_up_down_navigation() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::EditSavedSearch {
                search_name: "test-search".to_string(),
                search_input: String::new(),
                description_input: String::new(),
                disabled: false,
                selected_field: SavedSearchField::Search,
            })
            .build(),
        );

        // Down to next field
        let action = app.handle_popup_input(key(KeyCode::Down));
        assert!(action.is_none());
        assert!(matches!(
            app.popup,
            Some(Popup {
                kind: PopupType::EditSavedSearch {
                    selected_field: SavedSearchField::Description,
                    ..
                },
                ..
            })
        ));

        // Up to previous field
        let action = app.handle_popup_input(key(KeyCode::Up));
        assert!(action.is_none());
        assert!(matches!(
            app.popup,
            Some(Popup {
                kind: PopupType::EditSavedSearch {
                    selected_field: SavedSearchField::Search,
                    ..
                },
                ..
            })
        ));
    }

    #[test]
    fn test_edit_saved_search_character_input() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::EditSavedSearch {
                search_name: "test-search".to_string(),
                search_input: String::new(),
                description_input: String::new(),
                disabled: false,
                selected_field: SavedSearchField::Search,
            })
            .build(),
        );

        // Type in search field
        app.handle_popup_input(char_key('i'));
        app.handle_popup_input(char_key('n'));
        app.handle_popup_input(char_key('d'));
        app.handle_popup_input(char_key('e'));
        app.handle_popup_input(char_key('x'));
        app.handle_popup_input(char_key('='));
        app.handle_popup_input(char_key('m'));
        app.handle_popup_input(char_key('a'));
        app.handle_popup_input(char_key('i'));
        app.handle_popup_input(char_key('n'));

        assert!(
            matches!(app.popup, Some(Popup { kind: PopupType::EditSavedSearch { ref search_input, .. }, .. }) if search_input == "index=main")
        );
    }

    #[test]
    fn test_edit_saved_search_backspace() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::EditSavedSearch {
                search_name: "test-search".to_string(),
                search_input: "index=main".to_string(),
                description_input: String::new(),
                disabled: false,
                selected_field: SavedSearchField::Search,
            })
            .build(),
        );

        // Backspace removes last character
        let action = app.handle_popup_input(key(KeyCode::Backspace));
        assert!(action.is_none());
        assert!(
            matches!(app.popup, Some(Popup { kind: PopupType::EditSavedSearch { ref search_input, .. }, .. }) if search_input == "index=mai")
        );
    }

    #[test]
    fn test_edit_saved_search_toggle_disabled() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::EditSavedSearch {
                search_name: "test-search".to_string(),
                search_input: String::new(),
                description_input: String::new(),
                disabled: false,
                selected_field: SavedSearchField::Disabled,
            })
            .build(),
        );

        // Space toggles disabled
        let action = app.handle_popup_input(char_key(' '));
        assert!(action.is_none());
        assert!(matches!(
            app.popup,
            Some(Popup {
                kind: PopupType::EditSavedSearch { disabled: true, .. },
                ..
            })
        ));

        // Space toggles back
        let action = app.handle_popup_input(char_key(' '));
        assert!(action.is_none());
        assert!(matches!(
            app.popup,
            Some(Popup {
                kind: PopupType::EditSavedSearch {
                    disabled: false,
                    ..
                },
                ..
            })
        ));
    }

    #[test]
    fn test_edit_saved_search_submit() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::EditSavedSearch {
                search_name: "test-search".to_string(),
                search_input: "index=main".to_string(),
                description_input: "Test description".to_string(),
                disabled: true,
                selected_field: SavedSearchField::Search,
            })
            .build(),
        );

        // Submit with Enter
        let action = app.handle_popup_input(key(KeyCode::Enter));
        assert!(matches!(
            action,
            Some(Action::UpdateSavedSearch {
                name,
                search: Some(s),
                description: Some(d),
                disabled: Some(dis),
            }) if name == "test-search" && s == "index=main" && d == "Test description" && dis
        ));
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_edit_saved_search_submit_empty_fields() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::EditSavedSearch {
                search_name: "test-search".to_string(),
                search_input: String::new(),
                description_input: String::new(),
                disabled: false,
                selected_field: SavedSearchField::Search,
            })
            .build(),
        );

        // Submit with empty fields - should still work but with None values
        let action = app.handle_popup_input(key(KeyCode::Enter));
        assert!(matches!(
            action,
            Some(Action::UpdateSavedSearch {
                name,
                search: None,
                description: None,
                disabled: Some(false),
            }) if name == "test-search"
        ));
        assert!(app.popup.is_none());
    }
}
