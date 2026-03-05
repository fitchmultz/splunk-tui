//! Command palette popup input handling.
//!
//! Responsibilities:
//! - Handle keyboard input for command palette fuzzy search
//! - Navigate and select commands from filtered results
//! - Execute selected commands
//!
//! Does NOT handle:
//! - Does NOT render the palette (handled by ui::popup module)
//! - Does NOT perform fuzzy search (handled by CommandPaletteState)

use crate::action::Action;
use crate::app::App;
use crate::ui::popup::{Popup, PopupType};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl App {
    /// Handle CommandPalette popup input.
    pub fn handle_command_palette_popup(&mut self, key: KeyEvent) -> Option<Action> {
        let popup = self.popup.as_ref()?;
        let PopupType::CommandPalette {
            input,
            selected_index,
            filtered_items,
        } = &popup.kind
        else {
            return None;
        };

        let input = input.clone();
        let mut selected_index = *selected_index;
        let filtered_items = filtered_items.clone();

        match key.code {
            // Close palette
            KeyCode::Esc | KeyCode::Char('q') => {
                self.popup = None;
                None
            }

            // Execute selected command
            KeyCode::Enter => {
                if let Some(item) = filtered_items.get(selected_index) {
                    let action = item.action.clone();
                    self.popup = None;
                    // Record in recent commands
                    self.command_palette_state.record_command(&action);
                    Some(action)
                } else {
                    None
                }
            }

            // Navigate down
            KeyCode::Char('j') | KeyCode::Down | KeyCode::Tab => {
                if !filtered_items.is_empty() {
                    selected_index = (selected_index + 1) % filtered_items.len();
                    self.update_command_palette_selection(selected_index);
                }
                None
            }

            // Navigate up
            KeyCode::Char('k') | KeyCode::Up | KeyCode::BackTab => {
                if !filtered_items.is_empty() {
                    selected_index = selected_index.saturating_sub(1);
                    if selected_index == 0 && filtered_items.len() > 1 && key.code == KeyCode::Up {
                        // Wrap around to end only for Up arrow at position 0
                        selected_index = filtered_items.len() - 1;
                    }
                    self.update_command_palette_selection(selected_index);
                }
                None
            }

            // Go to first
            KeyCode::Home => {
                selected_index = 0;
                self.update_command_palette_selection(selected_index);
                None
            }

            // Go to last
            KeyCode::End => {
                if !filtered_items.is_empty() {
                    selected_index = filtered_items.len() - 1;
                    self.update_command_palette_selection(selected_index);
                }
                None
            }

            // Clear search (Ctrl+U)
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.update_command_palette_input(String::new());
                None
            }

            // Backspace
            KeyCode::Backspace => {
                let mut new_input = input;
                new_input.pop();
                self.update_command_palette_input(new_input);
                None
            }

            // Character input (only if no modifiers except SHIFT)
            KeyCode::Char(c)
                if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT =>
            {
                let mut new_input = input;
                new_input.push(c);
                self.update_command_palette_input(new_input);
                None
            }

            _ => None,
        }
    }

    /// Open the command palette popup.
    pub fn open_command_palette(&mut self) {
        let current_screen = self.current_screen;
        let items = self.command_palette_state.search("", current_screen);

        self.popup = Some(
            Popup::builder(PopupType::CommandPalette {
                input: String::new(),
                selected_index: 0,
                filtered_items: items,
            })
            .build(),
        );
    }

    /// Update command palette search input and refilter.
    pub fn update_command_palette_input(&mut self, new_input: String) {
        if let Some(ref mut popup) = self.popup {
            if let PopupType::CommandPalette {
                ref mut input,
                ref mut selected_index,
                ref mut filtered_items,
                ..
            } = popup.kind
            {
                *input = new_input.clone();
                *selected_index = 0;
                *filtered_items = self
                    .command_palette_state
                    .search(&new_input, self.current_screen);
            }
        }
    }

    /// Update command palette selection.
    pub fn update_command_palette_selection(&mut self, new_index: usize) {
        if let Some(ref mut popup) = self.popup {
            if let PopupType::CommandPalette {
                ref mut selected_index,
                ..
            } = popup.kind
            {
                *selected_index = new_index;
            }
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

    fn char_key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty())
    }

    fn ctrl_key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    #[test]
    fn test_command_palette_opens_with_items() {
        let mut app = App::new(None, ConnectionContext::default());
        app.open_command_palette();

        assert!(app.popup.is_some());
        assert!(matches!(
            app.popup.as_ref().unwrap().kind,
            PopupType::CommandPalette { .. }
        ));
    }

    #[test]
    fn test_command_palette_close_on_esc() {
        let mut app = App::new(None, ConnectionContext::default());
        app.open_command_palette();

        let action = app.handle_command_palette_popup(key(KeyCode::Esc));
        assert!(action.is_none());
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_command_palette_close_on_q() {
        let mut app = App::new(None, ConnectionContext::default());
        app.open_command_palette();

        let action = app.handle_command_palette_popup(char_key('q'));
        assert!(action.is_none());
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_command_palette_character_input() {
        let mut app = App::new(None, ConnectionContext::default());
        app.open_command_palette();

        // Type "search"
        app.handle_command_palette_popup(char_key('s'));
        app.handle_command_palette_popup(char_key('e'));
        app.handle_command_palette_popup(char_key('a'));
        app.handle_command_palette_popup(char_key('r'));
        app.handle_command_palette_popup(char_key('c'));
        app.handle_command_palette_popup(char_key('h'));

        if let Some(PopupType::CommandPalette { input, .. }) = &app.popup.as_ref().map(|p| &p.kind)
        {
            assert_eq!(input, "search");
        } else {
            panic!("Expected CommandPalette popup");
        }
    }

    #[test]
    fn test_command_palette_backspace() {
        let mut app = App::new(None, ConnectionContext::default());
        app.open_command_palette();

        // Type "test"
        app.handle_command_palette_popup(char_key('t'));
        app.handle_command_palette_popup(char_key('e'));
        app.handle_command_palette_popup(char_key('s'));
        app.handle_command_palette_popup(char_key('t'));

        // Backspace
        app.handle_command_palette_popup(key(KeyCode::Backspace));

        if let Some(PopupType::CommandPalette { input, .. }) = &app.popup.as_ref().map(|p| &p.kind)
        {
            assert_eq!(input, "tes");
        } else {
            panic!("Expected CommandPalette popup");
        }
    }

    #[test]
    fn test_command_palette_clear_with_ctrl_u() {
        let mut app = App::new(None, ConnectionContext::default());
        app.open_command_palette();

        // Type "test"
        app.handle_command_palette_popup(char_key('t'));
        app.handle_command_palette_popup(char_key('e'));
        app.handle_command_palette_popup(char_key('s'));
        app.handle_command_palette_popup(char_key('t'));

        // Clear with Ctrl+U
        app.handle_command_palette_popup(ctrl_key('u'));

        if let Some(PopupType::CommandPalette { input, .. }) = &app.popup.as_ref().map(|p| &p.kind)
        {
            assert!(input.is_empty());
        } else {
            panic!("Expected CommandPalette popup");
        }
    }

    #[test]
    fn test_command_palette_navigation_down() {
        let mut app = App::new(None, ConnectionContext::default());
        app.open_command_palette();

        // Initial selection should be 0
        if let Some(PopupType::CommandPalette {
            selected_index,
            filtered_items,
            ..
        }) = &app.popup.as_ref().map(|p| &p.kind)
        {
            assert_eq!(*selected_index, 0);
            assert!(!filtered_items.is_empty());
        } else {
            panic!("Expected CommandPalette popup");
        }

        // Navigate down with j
        app.handle_command_palette_popup(char_key('j'));

        if let Some(PopupType::CommandPalette { selected_index, .. }) =
            &app.popup.as_ref().map(|p| &p.kind)
        {
            assert_eq!(*selected_index, 1);
        } else {
            panic!("Expected CommandPalette popup");
        }
    }

    #[test]
    fn test_command_palette_navigation_up() {
        let mut app = App::new(None, ConnectionContext::default());
        app.open_command_palette();

        // Navigate down first
        app.handle_command_palette_popup(char_key('j'));
        app.handle_command_palette_popup(char_key('j'));

        // Navigate up with k
        app.handle_command_palette_popup(char_key('k'));

        if let Some(PopupType::CommandPalette { selected_index, .. }) =
            &app.popup.as_ref().map(|p| &p.kind)
        {
            assert_eq!(*selected_index, 1);
        } else {
            panic!("Expected CommandPalette popup");
        }
    }

    #[test]
    fn test_command_palette_navigation_wrap() {
        let mut app = App::new(None, ConnectionContext::default());
        app.open_command_palette();

        // Get the item count
        let item_count = if let Some(PopupType::CommandPalette { filtered_items, .. }) =
            &app.popup.as_ref().map(|p| &p.kind)
        {
            filtered_items.len()
        } else {
            panic!("Expected CommandPalette popup");
        };

        // Navigate down past the end to wrap
        for _ in 0..item_count {
            app.handle_command_palette_popup(char_key('j'));
        }

        if let Some(PopupType::CommandPalette { selected_index, .. }) =
            &app.popup.as_ref().map(|p| &p.kind)
        {
            // Should wrap back to 0
            assert_eq!(*selected_index, 0);
        } else {
            panic!("Expected CommandPalette popup");
        }
    }

    #[test]
    fn test_command_palette_execute_with_enter() {
        let mut app = App::new(None, ConnectionContext::default());
        app.open_command_palette();

        // Execute the first item
        let action = app.handle_command_palette_popup(key(KeyCode::Enter));

        // Should return an action and close popup
        assert!(action.is_some());
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_command_palette_home_key() {
        let mut app = App::new(None, ConnectionContext::default());
        app.open_command_palette();

        // Navigate down
        app.handle_command_palette_popup(char_key('j'));
        app.handle_command_palette_popup(char_key('j'));

        // Go to home
        app.handle_command_palette_popup(key(KeyCode::Home));

        if let Some(PopupType::CommandPalette { selected_index, .. }) =
            &app.popup.as_ref().map(|p| &p.kind)
        {
            assert_eq!(*selected_index, 0);
        } else {
            panic!("Expected CommandPalette popup");
        }
    }

    #[test]
    fn test_command_palette_end_key() {
        let mut app = App::new(None, ConnectionContext::default());
        app.open_command_palette();

        // Go to end
        app.handle_command_palette_popup(key(KeyCode::End));

        if let Some(PopupType::CommandPalette {
            selected_index,
            filtered_items,
            ..
        }) = &app.popup.as_ref().map(|p| &p.kind)
        {
            assert_eq!(*selected_index, filtered_items.len() - 1);
        } else {
            panic!("Expected CommandPalette popup");
        }
    }
}
