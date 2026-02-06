//! Macro popup handlers.
//!
//! Responsibilities:
//! - Handle macro creation popup
//! - Form navigation and input handling
//!
//! Does NOT handle:
//! - Does NOT render popups (handled by ui::popup module)
//! - Does NOT execute the actual macro operations (just returns Action variants)

use crate::action::Action;
use crate::app::App;
use crate::ui::popup::{MacroField, Popup, PopupType};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl App {
    /// Handle macro-related popups (CreateMacro, EditMacro).
    pub fn handle_macro_popup(&mut self, key: KeyEvent) -> Option<Action> {
        match (self.popup.as_ref().map(|p| &p.kind), key.code) {
            // CreateMacro - submit
            (
                Some(PopupType::CreateMacro {
                    name_input,
                    definition_input,
                    args_input,
                    description_input,
                    disabled,
                    iseval,
                    ..
                }),
                KeyCode::Enter,
            ) => {
                // Name and definition are required
                if name_input.is_empty() || definition_input.is_empty() {
                    return None;
                }

                let name = name_input.clone();
                let definition = definition_input.clone();
                let args = if args_input.is_empty() {
                    None
                } else {
                    Some(args_input.clone())
                };
                let description = if description_input.is_empty() {
                    None
                } else {
                    Some(description_input.clone())
                };
                let disabled_flag = *disabled;
                let iseval_flag = *iseval;

                self.popup = None;
                Some(Action::CreateMacro {
                    name,
                    definition,
                    args,
                    description,
                    disabled: disabled_flag,
                    iseval: iseval_flag,
                })
            }
            // EditMacro - submit
            (
                Some(PopupType::EditMacro {
                    macro_name,
                    definition_input,
                    args_input,
                    description_input,
                    disabled,
                    iseval,
                    ..
                }),
                KeyCode::Enter,
            ) => {
                let name = macro_name.clone();
                // Empty inputs mean "don't change this field"
                let definition = if definition_input.is_empty() {
                    None
                } else {
                    Some(definition_input.clone())
                };
                let args = if args_input.is_empty() {
                    None
                } else {
                    Some(args_input.clone())
                };
                let description = if description_input.is_empty() {
                    None
                } else {
                    Some(description_input.clone())
                };
                let disabled_flag = *disabled;
                let iseval_flag = *iseval;

                self.popup = None;
                Some(Action::UpdateMacro {
                    name,
                    definition,
                    args,
                    description,
                    disabled: Some(disabled_flag),
                    iseval: Some(iseval_flag),
                })
            }
            // CreateMacro - close
            (Some(PopupType::CreateMacro { .. }), KeyCode::Esc) => {
                self.popup = None;
                None
            }
            // EditMacro - close
            (Some(PopupType::EditMacro { .. }), KeyCode::Esc) => {
                self.popup = None;
                None
            }
            // CreateMacro - Tab navigation
            (
                Some(PopupType::CreateMacro {
                    name_input,
                    definition_input,
                    args_input,
                    description_input,
                    disabled,
                    iseval,
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
                    Popup::builder(PopupType::CreateMacro {
                        name_input: name_input.clone(),
                        definition_input: definition_input.clone(),
                        args_input: args_input.clone(),
                        description_input: description_input.clone(),
                        disabled: *disabled,
                        iseval: *iseval,
                        selected_field: new_field,
                    })
                    .build(),
                );
                None
            }
            // EditMacro - Tab navigation
            (
                Some(PopupType::EditMacro {
                    macro_name,
                    definition_input,
                    args_input,
                    description_input,
                    disabled,
                    iseval,
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
                    Popup::builder(PopupType::EditMacro {
                        macro_name: macro_name.clone(),
                        definition_input: definition_input.clone(),
                        args_input: args_input.clone(),
                        description_input: description_input.clone(),
                        disabled: *disabled,
                        iseval: *iseval,
                        selected_field: new_field,
                    })
                    .build(),
                );
                None
            }
            // CreateMacro - Up navigation
            (
                Some(PopupType::CreateMacro {
                    name_input,
                    definition_input,
                    args_input,
                    description_input,
                    disabled,
                    iseval,
                    selected_field,
                }),
                KeyCode::Up,
            ) => {
                self.popup = Some(
                    Popup::builder(PopupType::CreateMacro {
                        name_input: name_input.clone(),
                        definition_input: definition_input.clone(),
                        args_input: args_input.clone(),
                        description_input: description_input.clone(),
                        disabled: *disabled,
                        iseval: *iseval,
                        selected_field: selected_field.previous(),
                    })
                    .build(),
                );
                None
            }
            // EditMacro - Up navigation
            (
                Some(PopupType::EditMacro {
                    macro_name,
                    definition_input,
                    args_input,
                    description_input,
                    disabled,
                    iseval,
                    selected_field,
                }),
                KeyCode::Up,
            ) => {
                self.popup = Some(
                    Popup::builder(PopupType::EditMacro {
                        macro_name: macro_name.clone(),
                        definition_input: definition_input.clone(),
                        args_input: args_input.clone(),
                        description_input: description_input.clone(),
                        disabled: *disabled,
                        iseval: *iseval,
                        selected_field: selected_field.previous(),
                    })
                    .build(),
                );
                None
            }
            // CreateMacro - Down navigation
            (
                Some(PopupType::CreateMacro {
                    name_input,
                    definition_input,
                    args_input,
                    description_input,
                    disabled,
                    iseval,
                    selected_field,
                }),
                KeyCode::Down,
            ) => {
                self.popup = Some(
                    Popup::builder(PopupType::CreateMacro {
                        name_input: name_input.clone(),
                        definition_input: definition_input.clone(),
                        args_input: args_input.clone(),
                        description_input: description_input.clone(),
                        disabled: *disabled,
                        iseval: *iseval,
                        selected_field: selected_field.next(),
                    })
                    .build(),
                );
                None
            }
            // EditMacro - Down navigation
            (
                Some(PopupType::EditMacro {
                    macro_name,
                    definition_input,
                    args_input,
                    description_input,
                    disabled,
                    iseval,
                    selected_field,
                }),
                KeyCode::Down,
            ) => {
                self.popup = Some(
                    Popup::builder(PopupType::EditMacro {
                        macro_name: macro_name.clone(),
                        definition_input: definition_input.clone(),
                        args_input: args_input.clone(),
                        description_input: description_input.clone(),
                        disabled: *disabled,
                        iseval: *iseval,
                        selected_field: selected_field.next(),
                    })
                    .build(),
                );
                None
            }
            // CreateMacro - character input
            (
                Some(PopupType::CreateMacro {
                    name_input,
                    definition_input,
                    args_input,
                    description_input,
                    disabled,
                    iseval,
                    selected_field,
                }),
                KeyCode::Char(c),
            ) => {
                match selected_field {
                    MacroField::Name => {
                        let mut new_name = name_input.clone();
                        new_name.push(c);
                        self.popup = Some(
                            Popup::builder(PopupType::CreateMacro {
                                name_input: new_name,
                                definition_input: definition_input.clone(),
                                args_input: args_input.clone(),
                                description_input: description_input.clone(),
                                disabled: *disabled,
                                iseval: *iseval,
                                selected_field: *selected_field,
                            })
                            .build(),
                        );
                    }
                    MacroField::Definition => {
                        let mut new_definition = definition_input.clone();
                        new_definition.push(c);
                        self.popup = Some(
                            Popup::builder(PopupType::CreateMacro {
                                name_input: name_input.clone(),
                                definition_input: new_definition,
                                args_input: args_input.clone(),
                                description_input: description_input.clone(),
                                disabled: *disabled,
                                iseval: *iseval,
                                selected_field: *selected_field,
                            })
                            .build(),
                        );
                    }
                    MacroField::Args => {
                        let mut new_args = args_input.clone();
                        new_args.push(c);
                        self.popup = Some(
                            Popup::builder(PopupType::CreateMacro {
                                name_input: name_input.clone(),
                                definition_input: definition_input.clone(),
                                args_input: new_args,
                                description_input: description_input.clone(),
                                disabled: *disabled,
                                iseval: *iseval,
                                selected_field: *selected_field,
                            })
                            .build(),
                        );
                    }
                    MacroField::Description => {
                        let mut new_desc = description_input.clone();
                        new_desc.push(c);
                        self.popup = Some(
                            Popup::builder(PopupType::CreateMacro {
                                name_input: name_input.clone(),
                                definition_input: definition_input.clone(),
                                args_input: args_input.clone(),
                                description_input: new_desc,
                                disabled: *disabled,
                                iseval: *iseval,
                                selected_field: *selected_field,
                            })
                            .build(),
                        );
                    }
                    MacroField::Disabled => {
                        // Toggle disabled on space
                        if c == ' ' {
                            self.popup = Some(
                                Popup::builder(PopupType::CreateMacro {
                                    name_input: name_input.clone(),
                                    definition_input: definition_input.clone(),
                                    args_input: args_input.clone(),
                                    description_input: description_input.clone(),
                                    disabled: !*disabled,
                                    iseval: *iseval,
                                    selected_field: *selected_field,
                                })
                                .build(),
                            );
                        }
                    }
                    MacroField::IsEval => {
                        // Toggle iseval on space
                        if c == ' ' {
                            self.popup = Some(
                                Popup::builder(PopupType::CreateMacro {
                                    name_input: name_input.clone(),
                                    definition_input: definition_input.clone(),
                                    args_input: args_input.clone(),
                                    description_input: description_input.clone(),
                                    disabled: *disabled,
                                    iseval: !*iseval,
                                    selected_field: *selected_field,
                                })
                                .build(),
                            );
                        }
                    }
                }
                None
            }
            // EditMacro - character input
            (
                Some(PopupType::EditMacro {
                    macro_name,
                    definition_input,
                    args_input,
                    description_input,
                    disabled,
                    iseval,
                    selected_field,
                }),
                KeyCode::Char(c),
            ) => {
                match selected_field {
                    MacroField::Name => {
                        // Name is readonly in edit mode - ignore input
                    }
                    MacroField::Definition => {
                        let mut new_definition = definition_input.clone();
                        new_definition.push(c);
                        self.popup = Some(
                            Popup::builder(PopupType::EditMacro {
                                macro_name: macro_name.clone(),
                                definition_input: new_definition,
                                args_input: args_input.clone(),
                                description_input: description_input.clone(),
                                disabled: *disabled,
                                iseval: *iseval,
                                selected_field: *selected_field,
                            })
                            .build(),
                        );
                    }
                    MacroField::Args => {
                        let mut new_args = args_input.clone();
                        new_args.push(c);
                        self.popup = Some(
                            Popup::builder(PopupType::EditMacro {
                                macro_name: macro_name.clone(),
                                definition_input: definition_input.clone(),
                                args_input: new_args,
                                description_input: description_input.clone(),
                                disabled: *disabled,
                                iseval: *iseval,
                                selected_field: *selected_field,
                            })
                            .build(),
                        );
                    }
                    MacroField::Description => {
                        let mut new_desc = description_input.clone();
                        new_desc.push(c);
                        self.popup = Some(
                            Popup::builder(PopupType::EditMacro {
                                macro_name: macro_name.clone(),
                                definition_input: definition_input.clone(),
                                args_input: args_input.clone(),
                                description_input: new_desc,
                                disabled: *disabled,
                                iseval: *iseval,
                                selected_field: *selected_field,
                            })
                            .build(),
                        );
                    }
                    MacroField::Disabled => {
                        // Toggle disabled on space
                        if c == ' ' {
                            self.popup = Some(
                                Popup::builder(PopupType::EditMacro {
                                    macro_name: macro_name.clone(),
                                    definition_input: definition_input.clone(),
                                    args_input: args_input.clone(),
                                    description_input: description_input.clone(),
                                    disabled: !*disabled,
                                    iseval: *iseval,
                                    selected_field: *selected_field,
                                })
                                .build(),
                            );
                        }
                    }
                    MacroField::IsEval => {
                        // Toggle iseval on space
                        if c == ' ' {
                            self.popup = Some(
                                Popup::builder(PopupType::EditMacro {
                                    macro_name: macro_name.clone(),
                                    definition_input: definition_input.clone(),
                                    args_input: args_input.clone(),
                                    description_input: description_input.clone(),
                                    disabled: *disabled,
                                    iseval: !*iseval,
                                    selected_field: *selected_field,
                                })
                                .build(),
                            );
                        }
                    }
                }
                None
            }
            // CreateMacro - backspace
            (
                Some(PopupType::CreateMacro {
                    name_input,
                    definition_input,
                    args_input,
                    description_input,
                    disabled,
                    iseval,
                    selected_field,
                }),
                KeyCode::Backspace,
            ) => {
                match selected_field {
                    MacroField::Name => {
                        let mut new_name = name_input.clone();
                        new_name.pop();
                        self.popup = Some(
                            Popup::builder(PopupType::CreateMacro {
                                name_input: new_name,
                                definition_input: definition_input.clone(),
                                args_input: args_input.clone(),
                                description_input: description_input.clone(),
                                disabled: *disabled,
                                iseval: *iseval,
                                selected_field: *selected_field,
                            })
                            .build(),
                        );
                    }
                    MacroField::Definition => {
                        let mut new_definition = definition_input.clone();
                        new_definition.pop();
                        self.popup = Some(
                            Popup::builder(PopupType::CreateMacro {
                                name_input: name_input.clone(),
                                definition_input: new_definition,
                                args_input: args_input.clone(),
                                description_input: description_input.clone(),
                                disabled: *disabled,
                                iseval: *iseval,
                                selected_field: *selected_field,
                            })
                            .build(),
                        );
                    }
                    MacroField::Args => {
                        let mut new_args = args_input.clone();
                        new_args.pop();
                        self.popup = Some(
                            Popup::builder(PopupType::CreateMacro {
                                name_input: name_input.clone(),
                                definition_input: definition_input.clone(),
                                args_input: new_args,
                                description_input: description_input.clone(),
                                disabled: *disabled,
                                iseval: *iseval,
                                selected_field: *selected_field,
                            })
                            .build(),
                        );
                    }
                    MacroField::Description => {
                        let mut new_desc = description_input.clone();
                        new_desc.pop();
                        self.popup = Some(
                            Popup::builder(PopupType::CreateMacro {
                                name_input: name_input.clone(),
                                definition_input: definition_input.clone(),
                                args_input: args_input.clone(),
                                description_input: new_desc,
                                disabled: *disabled,
                                iseval: *iseval,
                                selected_field: *selected_field,
                            })
                            .build(),
                        );
                    }
                    MacroField::Disabled | MacroField::IsEval => {
                        // No-op for toggle fields
                    }
                }
                None
            }
            // EditMacro - backspace
            (
                Some(PopupType::EditMacro {
                    macro_name,
                    definition_input,
                    args_input,
                    description_input,
                    disabled,
                    iseval,
                    selected_field,
                }),
                KeyCode::Backspace,
            ) => {
                match selected_field {
                    MacroField::Name => {
                        // Name is readonly in edit mode - ignore backspace
                    }
                    MacroField::Definition => {
                        let mut new_definition = definition_input.clone();
                        new_definition.pop();
                        self.popup = Some(
                            Popup::builder(PopupType::EditMacro {
                                macro_name: macro_name.clone(),
                                definition_input: new_definition,
                                args_input: args_input.clone(),
                                description_input: description_input.clone(),
                                disabled: *disabled,
                                iseval: *iseval,
                                selected_field: *selected_field,
                            })
                            .build(),
                        );
                    }
                    MacroField::Args => {
                        let mut new_args = args_input.clone();
                        new_args.pop();
                        self.popup = Some(
                            Popup::builder(PopupType::EditMacro {
                                macro_name: macro_name.clone(),
                                definition_input: definition_input.clone(),
                                args_input: new_args,
                                description_input: description_input.clone(),
                                disabled: *disabled,
                                iseval: *iseval,
                                selected_field: *selected_field,
                            })
                            .build(),
                        );
                    }
                    MacroField::Description => {
                        let mut new_desc = description_input.clone();
                        new_desc.pop();
                        self.popup = Some(
                            Popup::builder(PopupType::EditMacro {
                                macro_name: macro_name.clone(),
                                definition_input: definition_input.clone(),
                                args_input: args_input.clone(),
                                description_input: new_desc,
                                disabled: *disabled,
                                iseval: *iseval,
                                selected_field: *selected_field,
                            })
                            .build(),
                        );
                    }
                    MacroField::Disabled | MacroField::IsEval => {
                        // No-op for toggle fields
                    }
                }
                None
            }
            // Default: ignore other keys
            _ => None,
        }
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
    fn test_create_macro_popup_close() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::CreateMacro {
                name_input: String::new(),
                definition_input: String::new(),
                args_input: String::new(),
                description_input: String::new(),
                disabled: false,
                iseval: false,
                selected_field: MacroField::Name,
            })
            .build(),
        );

        // Close with Esc
        let action = app.handle_popup_input(key(KeyCode::Esc));
        assert!(action.is_none());
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_create_macro_popup_submit() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::CreateMacro {
                name_input: "test_macro".to_string(),
                definition_input: "index=main".to_string(),
                args_input: "arg1,arg2".to_string(),
                description_input: "Test description".to_string(),
                disabled: false,
                iseval: true,
                selected_field: MacroField::Name,
            })
            .build(),
        );

        // Submit with Enter
        let action = app.handle_popup_input(key(KeyCode::Enter));
        assert!(matches!(
            action,
            Some(Action::CreateMacro {
                name,
                definition,
                args: Some(a),
                description: Some(d),
                disabled: false,
                iseval: true,
            }) if name == "test_macro" && definition == "index=main" && a == "arg1,arg2" && d == "Test description"
        ));
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_create_macro_popup_empty_name() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::CreateMacro {
                name_input: String::new(),
                definition_input: "index=main".to_string(),
                args_input: String::new(),
                description_input: String::new(),
                disabled: false,
                iseval: false,
                selected_field: MacroField::Name,
            })
            .build(),
        );

        // Submit with empty name should not emit action
        let action = app.handle_popup_input(key(KeyCode::Enter));
        assert!(action.is_none());
        assert!(app.popup.is_some());
    }

    #[test]
    fn test_create_macro_popup_empty_definition() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::CreateMacro {
                name_input: "test_macro".to_string(),
                definition_input: String::new(),
                args_input: String::new(),
                description_input: String::new(),
                disabled: false,
                iseval: false,
                selected_field: MacroField::Name,
            })
            .build(),
        );

        // Submit with empty definition should not emit action
        let action = app.handle_popup_input(key(KeyCode::Enter));
        assert!(action.is_none());
        assert!(app.popup.is_some());
    }

    #[test]
    fn test_create_macro_popup_tab_navigation() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::CreateMacro {
                name_input: String::new(),
                definition_input: String::new(),
                args_input: String::new(),
                description_input: String::new(),
                disabled: false,
                iseval: false,
                selected_field: MacroField::Name,
            })
            .build(),
        );

        // Tab to next field
        let action = app.handle_popup_input(key(KeyCode::Tab));
        assert!(action.is_none());
        assert!(matches!(
            app.popup,
            Some(Popup {
                kind: PopupType::CreateMacro {
                    selected_field: MacroField::Definition,
                    ..
                },
                ..
            })
        ));

        // Tab again to Args
        let action = app.handle_popup_input(key(KeyCode::Tab));
        assert!(action.is_none());
        assert!(matches!(
            app.popup,
            Some(Popup {
                kind: PopupType::CreateMacro {
                    selected_field: MacroField::Args,
                    ..
                },
                ..
            })
        ));

        // Continue tabbing through all fields
        app.handle_popup_input(key(KeyCode::Tab)); // Description
        app.handle_popup_input(key(KeyCode::Tab)); // Disabled
        app.handle_popup_input(key(KeyCode::Tab)); // IsEval

        // Tab again wraps to Name
        let action = app.handle_popup_input(key(KeyCode::Tab));
        assert!(action.is_none());
        assert!(matches!(
            app.popup,
            Some(Popup {
                kind: PopupType::CreateMacro {
                    selected_field: MacroField::Name,
                    ..
                },
                ..
            })
        ));
    }

    #[test]
    fn test_create_macro_popup_shift_tab_navigation() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::CreateMacro {
                name_input: String::new(),
                definition_input: String::new(),
                args_input: String::new(),
                description_input: String::new(),
                disabled: false,
                iseval: false,
                selected_field: MacroField::Name,
            })
            .build(),
        );

        // Shift+Tab to previous field (wraps to IsEval)
        let action = app.handle_popup_input(shift_key(KeyCode::Tab));
        assert!(action.is_none());
        assert!(matches!(
            app.popup,
            Some(Popup {
                kind: PopupType::CreateMacro {
                    selected_field: MacroField::IsEval,
                    ..
                },
                ..
            })
        ));
    }

    #[test]
    fn test_create_macro_popup_up_down_navigation() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::CreateMacro {
                name_input: String::new(),
                definition_input: String::new(),
                args_input: String::new(),
                description_input: String::new(),
                disabled: false,
                iseval: false,
                selected_field: MacroField::Name,
            })
            .build(),
        );

        // Down to next field
        let action = app.handle_popup_input(key(KeyCode::Down));
        assert!(action.is_none());
        assert!(matches!(
            app.popup,
            Some(Popup {
                kind: PopupType::CreateMacro {
                    selected_field: MacroField::Definition,
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
                kind: PopupType::CreateMacro {
                    selected_field: MacroField::Name,
                    ..
                },
                ..
            })
        ));
    }

    #[test]
    fn test_create_macro_popup_character_input() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::CreateMacro {
                name_input: String::new(),
                definition_input: String::new(),
                args_input: String::new(),
                description_input: String::new(),
                disabled: false,
                iseval: false,
                selected_field: MacroField::Name,
            })
            .build(),
        );

        // Type in name field
        app.handle_popup_input(char_key('t'));
        app.handle_popup_input(char_key('e'));
        app.handle_popup_input(char_key('s'));
        app.handle_popup_input(char_key('t'));

        assert!(
            matches!(app.popup, Some(Popup { kind: PopupType::CreateMacro { ref name_input, .. }, .. }) if name_input == "test")
        );
    }

    #[test]
    fn test_create_macro_popup_backspace() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::CreateMacro {
                name_input: "test".to_string(),
                definition_input: String::new(),
                args_input: String::new(),
                description_input: String::new(),
                disabled: false,
                iseval: false,
                selected_field: MacroField::Name,
            })
            .build(),
        );

        // Backspace removes last character
        let action = app.handle_popup_input(key(KeyCode::Backspace));
        assert!(action.is_none());
        assert!(
            matches!(app.popup, Some(Popup { kind: PopupType::CreateMacro { ref name_input, .. }, .. }) if name_input == "tes")
        );
    }

    #[test]
    fn test_create_macro_popup_toggle_disabled() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::CreateMacro {
                name_input: String::new(),
                definition_input: String::new(),
                args_input: String::new(),
                description_input: String::new(),
                disabled: false,
                iseval: false,
                selected_field: MacroField::Disabled,
            })
            .build(),
        );

        // Space toggles disabled
        let action = app.handle_popup_input(char_key(' '));
        assert!(action.is_none());
        assert!(matches!(
            app.popup,
            Some(Popup {
                kind: PopupType::CreateMacro { disabled: true, .. },
                ..
            })
        ));

        // Space toggles back
        let action = app.handle_popup_input(char_key(' '));
        assert!(action.is_none());
        assert!(matches!(
            app.popup,
            Some(Popup {
                kind: PopupType::CreateMacro {
                    disabled: false,
                    ..
                },
                ..
            })
        ));
    }

    #[test]
    fn test_create_macro_popup_toggle_iseval() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::CreateMacro {
                name_input: String::new(),
                definition_input: String::new(),
                args_input: String::new(),
                description_input: String::new(),
                disabled: false,
                iseval: false,
                selected_field: MacroField::IsEval,
            })
            .build(),
        );

        // Space toggles iseval
        let action = app.handle_popup_input(char_key(' '));
        assert!(action.is_none());
        assert!(matches!(
            app.popup,
            Some(Popup {
                kind: PopupType::CreateMacro { iseval: true, .. },
                ..
            })
        ));

        // Space toggles back
        let action = app.handle_popup_input(char_key(' '));
        assert!(action.is_none());
        assert!(matches!(
            app.popup,
            Some(Popup {
                kind: PopupType::CreateMacro { iseval: false, .. },
                ..
            })
        ));
    }

    #[test]
    fn test_create_macro_popup_submit_empty_optional_fields() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::CreateMacro {
                name_input: "test_macro".to_string(),
                definition_input: "index=main".to_string(),
                args_input: String::new(),
                description_input: String::new(),
                disabled: false,
                iseval: false,
                selected_field: MacroField::Name,
            })
            .build(),
        );

        // Submit with empty optional fields - should work with None values
        let action = app.handle_popup_input(key(KeyCode::Enter));
        assert!(matches!(
            action,
            Some(Action::CreateMacro {
                name,
                definition,
                args: None,
                description: None,
                disabled: false,
                iseval: false,
            }) if name == "test_macro" && definition == "index=main"
        ));
        assert!(app.popup.is_none());
    }

    // EditMacro tests

    #[test]
    fn test_edit_macro_popup_close() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::EditMacro {
                macro_name: "test_macro".to_string(),
                definition_input: String::new(),
                args_input: String::new(),
                description_input: String::new(),
                disabled: false,
                iseval: false,
                selected_field: MacroField::Definition,
            })
            .build(),
        );

        // Close with Esc
        let action = app.handle_popup_input(key(KeyCode::Esc));
        assert!(action.is_none());
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_edit_macro_popup_submit() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::EditMacro {
                macro_name: "test_macro".to_string(),
                definition_input: "index=updated".to_string(),
                args_input: "arg1,arg2".to_string(),
                description_input: "Updated description".to_string(),
                disabled: true,
                iseval: true,
                selected_field: MacroField::Definition,
            })
            .build(),
        );

        // Submit with Enter
        let action = app.handle_popup_input(key(KeyCode::Enter));
        assert!(matches!(
            action,
            Some(Action::UpdateMacro {
                name,
                definition: Some(d),
                args: Some(a),
                description: Some(desc),
                disabled: Some(true),
                iseval: Some(true),
            }) if name == "test_macro" && d == "index=updated" && a == "arg1,arg2" && desc == "Updated description"
        ));
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_edit_macro_popup_empty_fields() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::EditMacro {
                macro_name: "test_macro".to_string(),
                definition_input: String::new(),
                args_input: String::new(),
                description_input: String::new(),
                disabled: false,
                iseval: false,
                selected_field: MacroField::Definition,
            })
            .build(),
        );

        // Submit with empty fields - should result in None values
        let action = app.handle_popup_input(key(KeyCode::Enter));
        assert!(matches!(
            action,
            Some(Action::UpdateMacro {
                name,
                definition: None,
                args: None,
                description: None,
                disabled: Some(false),
                iseval: Some(false),
            }) if name == "test_macro"
        ));
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_edit_macro_popup_tab_navigation() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::EditMacro {
                macro_name: "test_macro".to_string(),
                definition_input: String::new(),
                args_input: String::new(),
                description_input: String::new(),
                disabled: false,
                iseval: false,
                selected_field: MacroField::Definition,
            })
            .build(),
        );

        // Tab to next field (Args)
        let action = app.handle_popup_input(key(KeyCode::Tab));
        assert!(action.is_none());
        assert!(matches!(
            app.popup,
            Some(Popup {
                kind: PopupType::EditMacro {
                    selected_field: MacroField::Args,
                    ..
                },
                ..
            })
        ));

        // Tab again to Description
        let action = app.handle_popup_input(key(KeyCode::Tab));
        assert!(action.is_none());
        assert!(matches!(
            app.popup,
            Some(Popup {
                kind: PopupType::EditMacro {
                    selected_field: MacroField::Description,
                    ..
                },
                ..
            })
        ));
    }

    #[test]
    fn test_edit_macro_popup_character_input() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::EditMacro {
                macro_name: "test_macro".to_string(),
                definition_input: String::new(),
                args_input: String::new(),
                description_input: String::new(),
                disabled: false,
                iseval: false,
                selected_field: MacroField::Definition,
            })
            .build(),
        );

        // Type in definition field
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
            matches!(app.popup, Some(Popup { kind: PopupType::EditMacro { ref definition_input, .. }, .. }) if definition_input == "index=main")
        );
    }

    #[test]
    fn test_edit_macro_popup_name_readonly() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::EditMacro {
                macro_name: "test_macro".to_string(),
                definition_input: String::new(),
                args_input: String::new(),
                description_input: String::new(),
                disabled: false,
                iseval: false,
                selected_field: MacroField::Name,
            })
            .build(),
        );

        // Type in name field - should be ignored (readonly)
        app.handle_popup_input(char_key('x'));

        // Name should remain unchanged (macro_name doesn't change, and we can't type in Name field)
        assert!(
            matches!(app.popup, Some(Popup { kind: PopupType::EditMacro { macro_name: ref name, .. }, .. }) if name == "test_macro")
        );
    }

    #[test]
    fn test_edit_macro_popup_toggle_disabled() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::EditMacro {
                macro_name: "test_macro".to_string(),
                definition_input: String::new(),
                args_input: String::new(),
                description_input: String::new(),
                disabled: false,
                iseval: false,
                selected_field: MacroField::Disabled,
            })
            .build(),
        );

        // Space toggles disabled
        let action = app.handle_popup_input(char_key(' '));
        assert!(action.is_none());
        assert!(matches!(
            app.popup,
            Some(Popup {
                kind: PopupType::EditMacro { disabled: true, .. },
                ..
            })
        ));

        // Space toggles back
        let action = app.handle_popup_input(char_key(' '));
        assert!(action.is_none());
        assert!(matches!(
            app.popup,
            Some(Popup {
                kind: PopupType::EditMacro {
                    disabled: false,
                    ..
                },
                ..
            })
        ));
    }

    #[test]
    fn test_edit_macro_popup_toggle_iseval() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::EditMacro {
                macro_name: "test_macro".to_string(),
                definition_input: String::new(),
                args_input: String::new(),
                description_input: String::new(),
                disabled: false,
                iseval: false,
                selected_field: MacroField::IsEval,
            })
            .build(),
        );

        // Space toggles iseval
        let action = app.handle_popup_input(char_key(' '));
        assert!(action.is_none());
        assert!(matches!(
            app.popup,
            Some(Popup {
                kind: PopupType::EditMacro { iseval: true, .. },
                ..
            })
        ));

        // Space toggles back
        let action = app.handle_popup_input(char_key(' '));
        assert!(action.is_none());
        assert!(matches!(
            app.popup,
            Some(Popup {
                kind: PopupType::EditMacro { iseval: false, .. },
                ..
            })
        ));
    }

    #[test]
    fn test_edit_macro_popup_backspace() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::EditMacro {
                macro_name: "test_macro".to_string(),
                definition_input: "index=main".to_string(),
                args_input: String::new(),
                description_input: String::new(),
                disabled: false,
                iseval: false,
                selected_field: MacroField::Definition,
            })
            .build(),
        );

        // Backspace removes last character
        let action = app.handle_popup_input(key(KeyCode::Backspace));
        assert!(action.is_none());
        assert!(
            matches!(app.popup, Some(Popup { kind: PopupType::EditMacro { ref definition_input, .. }, .. }) if definition_input == "index=mai")
        );
    }

    #[test]
    fn test_edit_macro_popup_up_down_navigation() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::EditMacro {
                macro_name: "test_macro".to_string(),
                definition_input: String::new(),
                args_input: String::new(),
                description_input: String::new(),
                disabled: false,
                iseval: false,
                selected_field: MacroField::Definition,
            })
            .build(),
        );

        // Down to next field
        let action = app.handle_popup_input(key(KeyCode::Down));
        assert!(action.is_none());
        assert!(matches!(
            app.popup,
            Some(Popup {
                kind: PopupType::EditMacro {
                    selected_field: MacroField::Args,
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
                kind: PopupType::EditMacro {
                    selected_field: MacroField::Definition,
                    ..
                },
                ..
            })
        ));
    }
}
