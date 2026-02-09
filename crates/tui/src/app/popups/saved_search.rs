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
use crate::ui::popup::{Popup, PopupType, SavedSearchField};
use crate::undo::{SavedSearchRecoveryData, UndoableOperation};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl App {
    /// Handle saved search-related popups (EditSavedSearch).
    pub fn handle_saved_search_popup(&mut self, key: KeyEvent) -> Option<Action> {
        match (self.popup.as_ref().map(|p| &p.kind), key.code) {
            // EditSavedSearch - submit
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
                let name = search_name.clone();
                // Only include non-empty values in the update
                let search = if search_input.is_empty() {
                    None
                } else {
                    Some(search_input.clone())
                };
                let description = if description_input.is_empty() {
                    None
                } else {
                    Some(description_input.clone())
                };
                // Only include disabled if it's different from the original
                let disabled_flag = Some(*disabled);
                self.popup = None;
                Some(Action::UpdateSavedSearch {
                    name,
                    search,
                    description,
                    disabled: disabled_flag,
                })
            }
            // EditSavedSearch - close
            (Some(PopupType::EditSavedSearch { .. }), KeyCode::Esc) => {
                self.popup = None;
                None
            }
            // EditSavedSearch - Tab navigation
            (
                Some(PopupType::EditSavedSearch {
                    search_name,
                    search_input,
                    description_input,
                    disabled,
                    selected_field,
                }),
                KeyCode::Tab,
            ) => {
                let new_field = if key.modifiers.contains(KeyModifiers::SHIFT) {
                    selected_field.previous()
                } else {
                    selected_field.next()
                };
                self.popup = Some(
                    Popup::builder(PopupType::EditSavedSearch {
                        search_name: search_name.clone(),
                        search_input: search_input.clone(),
                        description_input: description_input.clone(),
                        disabled: *disabled,
                        selected_field: new_field,
                    })
                    .build(),
                );
                None
            }
            // EditSavedSearch - Up navigation
            (
                Some(PopupType::EditSavedSearch {
                    search_name,
                    search_input,
                    description_input,
                    disabled,
                    selected_field,
                }),
                KeyCode::Up,
            ) => {
                self.popup = Some(
                    Popup::builder(PopupType::EditSavedSearch {
                        search_name: search_name.clone(),
                        search_input: search_input.clone(),
                        description_input: description_input.clone(),
                        disabled: *disabled,
                        selected_field: selected_field.previous(),
                    })
                    .build(),
                );
                None
            }
            // EditSavedSearch - Down navigation
            (
                Some(PopupType::EditSavedSearch {
                    search_name,
                    search_input,
                    description_input,
                    disabled,
                    selected_field,
                }),
                KeyCode::Down,
            ) => {
                self.popup = Some(
                    Popup::builder(PopupType::EditSavedSearch {
                        search_name: search_name.clone(),
                        search_input: search_input.clone(),
                        description_input: description_input.clone(),
                        disabled: *disabled,
                        selected_field: selected_field.next(),
                    })
                    .build(),
                );
                None
            }
            // EditSavedSearch - character input
            (
                Some(PopupType::EditSavedSearch {
                    search_name,
                    search_input,
                    description_input,
                    disabled,
                    selected_field,
                }),
                KeyCode::Char(c),
            ) => {
                match selected_field {
                    SavedSearchField::Search => {
                        let mut new_search = search_input.clone();
                        new_search.push(c);
                        self.popup = Some(
                            Popup::builder(PopupType::EditSavedSearch {
                                search_name: search_name.clone(),
                                search_input: new_search,
                                description_input: description_input.clone(),
                                disabled: *disabled,
                                selected_field: *selected_field,
                            })
                            .build(),
                        );
                    }
                    SavedSearchField::Description => {
                        let mut new_desc = description_input.clone();
                        new_desc.push(c);
                        self.popup = Some(
                            Popup::builder(PopupType::EditSavedSearch {
                                search_name: search_name.clone(),
                                search_input: search_input.clone(),
                                description_input: new_desc,
                                disabled: *disabled,
                                selected_field: *selected_field,
                            })
                            .build(),
                        );
                    }
                    SavedSearchField::Disabled => {
                        // Toggle disabled on space
                        if c == ' ' {
                            self.popup = Some(
                                Popup::builder(PopupType::EditSavedSearch {
                                    search_name: search_name.clone(),
                                    search_input: search_input.clone(),
                                    description_input: description_input.clone(),
                                    disabled: !*disabled,
                                    selected_field: *selected_field,
                                })
                                .build(),
                            );
                        }
                    }
                    SavedSearchField::Name => {
                        // No-op: Name field only exists in CreateSavedSearch, not EditSavedSearch
                    }
                }
                None
            }
            // EditSavedSearch - backspace
            (
                Some(PopupType::EditSavedSearch {
                    search_name,
                    search_input,
                    description_input,
                    disabled,
                    selected_field,
                }),
                KeyCode::Backspace,
            ) => {
                match selected_field {
                    SavedSearchField::Search => {
                        let mut new_search = search_input.clone();
                        new_search.pop();
                        self.popup = Some(
                            Popup::builder(PopupType::EditSavedSearch {
                                search_name: search_name.clone(),
                                search_input: new_search,
                                description_input: description_input.clone(),
                                disabled: *disabled,
                                selected_field: *selected_field,
                            })
                            .build(),
                        );
                    }
                    SavedSearchField::Description => {
                        let mut new_desc = description_input.clone();
                        new_desc.pop();
                        self.popup = Some(
                            Popup::builder(PopupType::EditSavedSearch {
                                search_name: search_name.clone(),
                                search_input: search_input.clone(),
                                description_input: new_desc,
                                disabled: *disabled,
                                selected_field: *selected_field,
                            })
                            .build(),
                        );
                    }
                    SavedSearchField::Disabled => {
                        // No-op for disabled field
                    }
                    SavedSearchField::Name => {
                        // No-op: Name field only exists in CreateSavedSearch, not EditSavedSearch
                    }
                }
                None
            }
            // CreateSavedSearch - submit
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
                // Name and search are required
                if name_input.is_empty() || search_input.is_empty() {
                    return None;
                }

                let name = name_input.clone();
                let search = search_input.clone();
                let description = if description_input.is_empty() {
                    None
                } else {
                    Some(description_input.clone())
                };
                let disabled_flag = *disabled;

                self.popup = None;
                Some(Action::CreateSavedSearch {
                    name,
                    search,
                    description,
                    disabled: disabled_flag,
                })
            }
            // CreateSavedSearch - close
            (Some(PopupType::CreateSavedSearch { .. }), KeyCode::Esc) => {
                self.popup = None;
                None
            }
            // CreateSavedSearch - Tab navigation
            (
                Some(PopupType::CreateSavedSearch {
                    name_input,
                    search_input,
                    description_input,
                    disabled,
                    selected_field,
                }),
                KeyCode::Tab,
            ) => {
                let new_field = if key.modifiers.contains(KeyModifiers::SHIFT) {
                    selected_field.previous()
                } else {
                    selected_field.next()
                };
                self.popup = Some(
                    Popup::builder(PopupType::CreateSavedSearch {
                        name_input: name_input.clone(),
                        search_input: search_input.clone(),
                        description_input: description_input.clone(),
                        disabled: *disabled,
                        selected_field: new_field,
                    })
                    .build(),
                );
                None
            }
            // CreateSavedSearch - Up navigation
            (
                Some(PopupType::CreateSavedSearch {
                    name_input,
                    search_input,
                    description_input,
                    disabled,
                    selected_field,
                }),
                KeyCode::Up,
            ) => {
                self.popup = Some(
                    Popup::builder(PopupType::CreateSavedSearch {
                        name_input: name_input.clone(),
                        search_input: search_input.clone(),
                        description_input: description_input.clone(),
                        disabled: *disabled,
                        selected_field: selected_field.previous(),
                    })
                    .build(),
                );
                None
            }
            // CreateSavedSearch - Down navigation
            (
                Some(PopupType::CreateSavedSearch {
                    name_input,
                    search_input,
                    description_input,
                    disabled,
                    selected_field,
                }),
                KeyCode::Down,
            ) => {
                self.popup = Some(
                    Popup::builder(PopupType::CreateSavedSearch {
                        name_input: name_input.clone(),
                        search_input: search_input.clone(),
                        description_input: description_input.clone(),
                        disabled: *disabled,
                        selected_field: selected_field.next(),
                    })
                    .build(),
                );
                None
            }
            // CreateSavedSearch - character input
            (
                Some(PopupType::CreateSavedSearch {
                    name_input,
                    search_input,
                    description_input,
                    disabled,
                    selected_field,
                }),
                KeyCode::Char(c),
            ) => {
                match selected_field {
                    SavedSearchField::Name => {
                        let mut new_name = name_input.clone();
                        new_name.push(c);
                        self.popup = Some(
                            Popup::builder(PopupType::CreateSavedSearch {
                                name_input: new_name,
                                search_input: search_input.clone(),
                                description_input: description_input.clone(),
                                disabled: *disabled,
                                selected_field: *selected_field,
                            })
                            .build(),
                        );
                    }
                    SavedSearchField::Search => {
                        let mut new_search = search_input.clone();
                        new_search.push(c);
                        self.popup = Some(
                            Popup::builder(PopupType::CreateSavedSearch {
                                name_input: name_input.clone(),
                                search_input: new_search,
                                description_input: description_input.clone(),
                                disabled: *disabled,
                                selected_field: *selected_field,
                            })
                            .build(),
                        );
                    }
                    SavedSearchField::Description => {
                        let mut new_desc = description_input.clone();
                        new_desc.push(c);
                        self.popup = Some(
                            Popup::builder(PopupType::CreateSavedSearch {
                                name_input: name_input.clone(),
                                search_input: search_input.clone(),
                                description_input: new_desc,
                                disabled: *disabled,
                                selected_field: *selected_field,
                            })
                            .build(),
                        );
                    }
                    SavedSearchField::Disabled => {
                        // Toggle disabled on space
                        if c == ' ' {
                            self.popup = Some(
                                Popup::builder(PopupType::CreateSavedSearch {
                                    name_input: name_input.clone(),
                                    search_input: search_input.clone(),
                                    description_input: description_input.clone(),
                                    disabled: !*disabled,
                                    selected_field: *selected_field,
                                })
                                .build(),
                            );
                        }
                    }
                }
                None
            }
            // CreateSavedSearch - backspace
            (
                Some(PopupType::CreateSavedSearch {
                    name_input,
                    search_input,
                    description_input,
                    disabled,
                    selected_field,
                }),
                KeyCode::Backspace,
            ) => {
                match selected_field {
                    SavedSearchField::Name => {
                        let mut new_name = name_input.clone();
                        new_name.pop();
                        self.popup = Some(
                            Popup::builder(PopupType::CreateSavedSearch {
                                name_input: new_name,
                                search_input: search_input.clone(),
                                description_input: description_input.clone(),
                                disabled: *disabled,
                                selected_field: *selected_field,
                            })
                            .build(),
                        );
                    }
                    SavedSearchField::Search => {
                        let mut new_search = search_input.clone();
                        new_search.pop();
                        self.popup = Some(
                            Popup::builder(PopupType::CreateSavedSearch {
                                name_input: name_input.clone(),
                                search_input: new_search,
                                description_input: description_input.clone(),
                                disabled: *disabled,
                                selected_field: *selected_field,
                            })
                            .build(),
                        );
                    }
                    SavedSearchField::Description => {
                        let mut new_desc = description_input.clone();
                        new_desc.pop();
                        self.popup = Some(
                            Popup::builder(PopupType::CreateSavedSearch {
                                name_input: name_input.clone(),
                                search_input: search_input.clone(),
                                description_input: new_desc,
                                disabled: *disabled,
                                selected_field: *selected_field,
                            })
                            .build(),
                        );
                    }
                    SavedSearchField::Disabled => {
                        // No-op for disabled field
                    }
                }
                None
            }
            // DeleteSavedSearchConfirm - confirm (queue as undoable operation)
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
