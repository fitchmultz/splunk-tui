//! Search macros input handling.
//!
//! Responsibilities:
//! - Handle keyboard input for the macros screen.
//! - Return actions for macro operations.
//!
//! Does NOT handle:
//! - Does not render (see ui/screens/macros.rs).
//! - Does not execute async operations (see runtime/side_effects/macros.rs).

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::Action;
use crate::app::App;

impl App {
    /// Handle input for the macros screen.
    pub fn handle_macros_input(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            // Refresh macros list
            KeyCode::Char('r') if key.modifiers.is_empty() => Some(Action::LoadMacros),

            // Copy macro definition to clipboard (Ctrl+C or 'y' vim-style)
            KeyCode::Char('c') if key.modifiers == KeyModifiers::CONTROL => {
                if let Some(macros) = &self.macros
                    && let Some(selected) = self.macros_state.selected()
                    && let Some(macro_item) = macros.get(selected)
                {
                    return Some(Action::CopyToClipboard(macro_item.definition.clone()));
                }
                None
            }
            KeyCode::Char('y') if key.modifiers.is_empty() => {
                if let Some(macros) = &self.macros
                    && let Some(selected) = self.macros_state.selected()
                    && let Some(macro_item) = macros.get(selected)
                {
                    return Some(Action::CopyToClipboard(macro_item.definition.clone()));
                }
                None
            }

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
                if let Some(macros) = &self.macros {
                    let current = self.macros_state.selected().unwrap_or(0);
                    let next = (current + 1).min(macros.len().saturating_sub(1));
                    self.macros_state.select(Some(next));
                }
                None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let current = self.macros_state.selected().unwrap_or(0);
                let prev = current.saturating_sub(1);
                self.macros_state.select(Some(prev));
                None
            }
            KeyCode::PageDown => {
                if let Some(macros) = &self.macros {
                    let current = self.macros_state.selected().unwrap_or(0);
                    let page_size = 10;
                    let next = (current + page_size).min(macros.len().saturating_sub(1));
                    self.macros_state.select(Some(next));
                }
                None
            }
            KeyCode::PageUp => {
                let current = self.macros_state.selected().unwrap_or(0);
                let page_size = 10;
                let prev = current.saturating_sub(page_size);
                self.macros_state.select(Some(prev));
                None
            }
            KeyCode::Home => {
                self.macros_state.select(Some(0));
                None
            }
            KeyCode::End => {
                if let Some(macros) = &self.macros {
                    let last = macros.len().saturating_sub(1);
                    self.macros_state.select(Some(last));
                }
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
        KeyEvent::new(c, KeyModifiers::empty())
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
