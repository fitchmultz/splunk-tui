//! Search macros input handling.
//!
//! Responsibilities:
//! - Handle keyboard input for the macros screen.
//! - Return actions for macro operations.
//!
//! Does NOT handle:
//! - Does not render (see ui/screens/macros.rs).
//! - Does not execute async operations (see runtime/side_effects/macros.rs).

use crossterm::event::{KeyCode, KeyEvent};

use crate::action::Action;
use crate::app::App;
use crate::app::input::helpers::{handle_copy_with_toast, is_copy_key};

impl App {
    /// Handle input for the macros screen.
    pub fn handle_macros_input(&mut self, key: KeyEvent) -> Option<Action> {
        if is_copy_key(key) {
            let content = self.macros.as_ref().and_then(|macros| {
                self.macros_state
                    .selected()
                    .and_then(|selected| macros.get(selected))
                    .map(|macro_item| macro_item.definition.clone())
            });

            return handle_copy_with_toast(self, content);
        }

        match key.code {
            // Refresh macros list
            KeyCode::Char('r') if key.modifiers.is_empty() => Some(Action::LoadMacros),

            // Edit selected macro
            KeyCode::Char('e') if key.modifiers.is_empty() => Some(Action::EditMacro),

            // Create new macro
            KeyCode::Char('n') if key.modifiers.is_empty() => Some(Action::OpenCreateMacroDialog),

            // Delete selected macro
            KeyCode::Char('d') if key.modifiers.is_empty() => {
                if let Some(macros) = &self.macros
                    && let Some(selected) = self.macros_state.selected()
                    && let Some(macro_item) = macros.get(selected)
                {
                    return Some(Action::DeleteMacro {
                        name: macro_item.name.clone(),
                    });
                }
                None
            }

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

            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConnectionContext;

    fn key(c: KeyCode) -> KeyEvent {
        KeyEvent::new(c, crossterm::event::KeyModifiers::empty())
    }

    #[test]
    fn test_n_key_opens_create_macro_dialog() {
        let mut app = App::new(None, ConnectionContext::default());

        let action = app.handle_macros_input(key(KeyCode::Char('n')));

        assert!(matches!(action, Some(Action::OpenCreateMacroDialog)));
    }

    #[test]
    fn test_r_key_loads_macros() {
        let mut app = App::new(None, ConnectionContext::default());

        let action = app.handle_macros_input(key(KeyCode::Char('r')));

        assert!(matches!(action, Some(Action::LoadMacros)));
    }

    #[test]
    fn test_e_key_edits_macro() {
        let mut app = App::new(None, ConnectionContext::default());

        let action = app.handle_macros_input(key(KeyCode::Char('e')));

        assert!(matches!(action, Some(Action::EditMacro)));
    }
}
