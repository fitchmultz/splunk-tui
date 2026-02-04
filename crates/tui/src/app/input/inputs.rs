//! Input screen keyboard input handler.
//!
//! Responsibilities:
//! - Handle keyboard input for the inputs screen
//! - Trigger input refresh, enable/disable operations
//!
//! Does NOT handle:
//! - Direct state modification (returns Actions)
//! - UI rendering

use crate::action::Action;
use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl App {
    /// Handle keyboard input for the inputs screen.
    ///
    /// Keybindings:
    /// - 'r' / F5: Refresh inputs list
    /// - 'e': Enable selected input
    /// - 'd': Disable selected input
    /// - Enter: Show input details (not implemented yet)
    /// - Ctrl+E: Export inputs list
    pub fn handle_inputs_input(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Char('r') | KeyCode::F(5) => {
                // Refresh inputs list
                Some(Action::LoadInputs {
                    count: self.inputs_pagination.page_size,
                    offset: 0,
                })
            }
            KeyCode::Char('e') => {
                // Enable selected input
                if let Some(ref inputs) = self.inputs
                    && let Some(selected) = self.inputs_state.selected()
                    && let Some(input) = inputs.get(selected)
                {
                    return Some(Action::EnableInput {
                        input_type: input.input_type.clone(),
                        name: input.name.clone(),
                    });
                }
                None
            }
            KeyCode::Char('d') => {
                // Disable selected input
                if let Some(ref inputs) = self.inputs
                    && let Some(selected) = self.inputs_state.selected()
                    && let Some(input) = inputs.get(selected)
                {
                    return Some(Action::DisableInput {
                        input_type: input.input_type.clone(),
                        name: input.name.clone(),
                    });
                }
                None
            }
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Copy selected input name to clipboard (Ctrl+C)
                if let Some(ref inputs) = self.inputs
                    && let Some(selected) = self.inputs_state.selected()
                    && let Some(input) = inputs.get(selected)
                {
                    return Some(Action::CopyToClipboard(input.name.clone()));
                }
                None
            }
            KeyCode::Char('y') if key.modifiers.is_empty() => {
                // Copy selected input name to clipboard (vim-style 'y')
                if let Some(ref inputs) = self.inputs
                    && let Some(selected) = self.inputs_state.selected()
                    && let Some(input) = inputs.get(selected)
                {
                    return Some(Action::CopyToClipboard(input.name.clone()));
                }
                None
            }
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
