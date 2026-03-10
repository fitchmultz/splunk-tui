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
use crate::app::export::ExportTarget;
use crate::app::input::helpers::{
    handle_copy_with_toast, handle_list_export, is_copy_key, is_export_key, should_export_list,
};
use crossterm::event::{KeyCode, KeyEvent};

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
        if is_copy_key(key) {
            let content = self.inputs.as_ref().and_then(|inputs| {
                self.inputs_state
                    .selected()
                    .and_then(|selected| inputs.get(selected))
                    .map(|input| input.name.clone())
            });

            return handle_copy_with_toast(self, content);
        }

        if is_export_key(key) {
            let can_export = should_export_list(self.inputs.as_ref());
            return handle_list_export(self, can_export, ExportTarget::Inputs);
        }

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
                        input_type: input.input_type.to_string(),
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
                        input_type: input.input_type.to_string(),
                        name: input.name.clone(),
                    });
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ConnectionContext;
    use crate::ui::popup::PopupType;
    use crossterm::event::KeyModifiers;
    use splunk_client::models::{Input, InputType};

    fn ctrl_key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    fn sample_input(name: &str) -> Input {
        Input {
            name: name.to_string(),
            input_type: InputType::Monitor,
            disabled: false,
            host: None,
            source: None,
            sourcetype: None,
            connection_host: None,
            port: None,
            path: None,
            blacklist: None,
            whitelist: None,
            recursive: None,
            command: None,
            interval: None,
        }
    }

    #[test]
    fn test_ctrl_c_without_selected_input_shows_toast() {
        let mut app = App::new(None, ConnectionContext::default());
        app.inputs = Some(vec![]);

        let action = app.handle_inputs_input(ctrl_key('c'));

        assert!(action.is_none());
        assert_eq!(
            app.toasts.last().map(|toast| toast.message.as_str()),
            Some("Nothing to copy")
        );
    }

    #[test]
    fn test_ctrl_e_without_inputs_does_not_open_export_popup() {
        let mut app = App::new(None, ConnectionContext::default());

        let action = app.handle_inputs_input(ctrl_key('e'));

        assert!(action.is_none());
        assert!(app.popup.is_none());
        assert!(app.export_target.is_none());
    }

    #[test]
    fn test_ctrl_e_with_inputs_opens_export_popup() {
        let mut app = App::new(None, ConnectionContext::default());
        app.inputs = Some(vec![sample_input("monitor:///tmp/test.log")]);

        let action = app.handle_inputs_input(ctrl_key('e'));

        assert!(action.is_none());
        assert_eq!(app.export_target, Some(ExportTarget::Inputs));
        assert!(matches!(
            app.popup.as_ref().map(|popup| &popup.kind),
            Some(PopupType::ExportSearch)
        ));
    }
}
