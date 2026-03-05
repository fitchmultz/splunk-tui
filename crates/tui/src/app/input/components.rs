//! Rich input component wrappers using tui-input and tui-textarea.
//!
//! Provides enhanced text editing with proper cursor management,
//! history support, and placeholder text.

use ratatui::{layout::Rect, style::Style, widgets::Block};
use tui_input::{Input, InputRequest};

/// Single-line input wrapper with enhanced functionality.
#[derive(Debug, Clone, Default)]
pub struct SingleLineInput {
    input: Input,
    placeholder: Option<String>,
}

impl SingleLineInput {
    /// Create a new empty single-line input.
    pub fn new() -> Self {
        Self {
            input: Input::default(),
            placeholder: None,
        }
    }

    /// Create a new input with the given value.
    pub fn with_value(value: impl Into<String>) -> Self {
        Self {
            input: Input::new(value.into()),
            placeholder: None,
        }
    }

    /// Create a new input with a placeholder.
    pub fn with_placeholder(placeholder: impl Into<String>) -> Self {
        Self {
            input: Input::default(),
            placeholder: Some(placeholder.into()),
        }
    }

    /// Create a new input with both value and placeholder.
    pub fn with_value_and_placeholder(
        value: impl Into<String>,
        placeholder: impl Into<String>,
    ) -> Self {
        Self {
            input: Input::new(value.into()),
            placeholder: Some(placeholder.into()),
        }
    }

    /// Handle key event using InputRequest pattern.
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::{KeyCode, KeyModifiers};

        // Handle word navigation with Ctrl
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Left => {
                    self.input.handle(InputRequest::GoToPrevWord);
                    return;
                }
                KeyCode::Right => {
                    self.input.handle(InputRequest::GoToNextWord);
                    return;
                }
                // Ctrl+U: clear to beginning of line
                KeyCode::Char('u') => {
                    self.input.handle(InputRequest::DeleteLine);
                    return;
                }
                // Ctrl+K: clear to end of line
                KeyCode::Char('k') => {
                    let cursor = self.input.cursor();
                    let value = self.input.value();
                    let new_value: String = value.chars().take(cursor).collect();
                    self.input = Input::new(new_value);
                    return;
                }
                // Ctrl+A: select all (we'll just go to start for now)
                KeyCode::Char('a') => {
                    self.input.handle(InputRequest::GoToStart);
                    return;
                }
                // Ctrl+E: go to end (same as End key)
                KeyCode::Char('e') => {
                    self.input.handle(InputRequest::GoToEnd);
                    return;
                }
                _ => {}
            }
        }

        let req = match key.code {
            KeyCode::Char(c) => Some(InputRequest::InsertChar(c)),
            KeyCode::Backspace => Some(InputRequest::DeletePrevChar),
            KeyCode::Delete => Some(InputRequest::DeleteNextChar),
            KeyCode::Left => Some(InputRequest::GoToPrevChar),
            KeyCode::Right => Some(InputRequest::GoToNextChar),
            KeyCode::Home => Some(InputRequest::GoToStart),
            KeyCode::End => Some(InputRequest::GoToEnd),
            _ => None,
        };

        if let Some(r) = req {
            self.input.handle(r);
        }
    }

    /// Get current value.
    pub fn value(&self) -> &str {
        self.input.value()
    }

    /// Set value programmatically.
    pub fn set_value(&mut self, value: impl Into<String>) {
        self.input = Input::new(value.into());
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.input.value().is_empty()
    }

    /// Get cursor position (character index).
    pub fn cursor_position(&self) -> usize {
        self.input.cursor()
    }

    /// Set cursor position (clamped to valid range).
    pub fn set_cursor_position(&mut self, pos: usize) {
        let len = self.input.value().chars().count();
        let clamped = pos.min(len);
        // Move to start then forward to position
        self.input.handle(InputRequest::GoToStart);
        for _ in 0..clamped {
            self.input.handle(InputRequest::GoToNextChar);
        }
    }

    /// Push a character to the end of the input.
    pub fn push(&mut self, c: char) {
        self.input.handle(InputRequest::GoToEnd);
        self.input.handle(InputRequest::InsertChar(c));
    }

    /// Clear the input.
    pub fn clear(&mut self) {
        self.input.handle(InputRequest::DeleteLine);
    }

    /// Pop the last character from the input.
    pub fn pop(&mut self) -> Option<char> {
        let value = self.input.value().to_string();
        if let Some(c) = value.chars().last() {
            self.input.handle(InputRequest::GoToEnd);
            self.input.handle(InputRequest::DeletePrevChar);
            Some(c)
        } else {
            None
        }
    }

    /// Get the placeholder text if any.
    pub fn placeholder(&self) -> Option<&str> {
        self.placeholder.as_deref()
    }

    /// Set placeholder text.
    pub fn set_placeholder(&mut self, placeholder: impl Into<String>) {
        self.placeholder = Some(placeholder.into());
    }

    /// Get the length of the input value.
    pub fn len(&self) -> usize {
        self.input.value().len()
    }

    /// Access the underlying Input for rendering.
    pub fn inner(&self) -> &Input {
        &self.input
    }
}

impl From<String> for SingleLineInput {
    fn from(s: String) -> Self {
        Self::with_value(s)
    }
}

impl From<&str> for SingleLineInput {
    fn from(s: &str) -> Self {
        Self::with_value(s)
    }
}

impl std::fmt::Display for SingleLineInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.input.value())
    }
}

/// Multi-line textarea wrapper for SPL queries.
pub struct MultiLineInput<'a> {
    textarea: tui_textarea::TextArea<'a>,
}

impl<'a> MultiLineInput<'a> {
    /// Create a new empty multi-line input.
    pub fn new() -> Self {
        let textarea = tui_textarea::TextArea::default();
        Self { textarea }
    }

    /// Create a new input with a placeholder.
    pub fn with_placeholder(placeholder: impl Into<String>) -> Self {
        let mut textarea = tui_textarea::TextArea::default();
        textarea.set_placeholder_text(placeholder);
        Self { textarea }
    }

    /// Create a new input with the given value.
    pub fn with_value(value: impl AsRef<str>) -> Self {
        let lines: Vec<String> = value.as_ref().lines().map(|s| s.to_string()).collect();
        let textarea = tui_textarea::TextArea::new(lines);
        Self { textarea }
    }

    /// Handle key event.
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        self.textarea.input(key);
    }

    /// Get current value (concatenated lines).
    pub fn value(&self) -> String {
        self.textarea.lines().join("\n")
    }

    /// Set value programmatically.
    pub fn set_value(&mut self, value: impl AsRef<str>) {
        let lines: Vec<String> = value.as_ref().lines().map(|s| s.to_string()).collect();
        self.textarea = tui_textarea::TextArea::new(lines);
    }

    /// Check if empty (all lines are empty).
    pub fn is_empty(&self) -> bool {
        self.textarea.lines().iter().all(|line| line.is_empty())
    }

    /// Set styling for the input text.
    pub fn set_style(&mut self, style: Style) {
        self.textarea.set_style(style);
    }

    /// Set block (borders/title) for the textarea.
    pub fn set_block(&mut self, block: Block<'a>) {
        self.textarea.set_block(block);
    }

    /// Set placeholder text.
    pub fn set_placeholder(&mut self, placeholder: impl Into<String>) {
        self.textarea.set_placeholder_text(placeholder);
    }

    /// Set cursor style.
    pub fn set_cursor_style(&mut self, style: Style) {
        self.textarea.set_cursor_style(style);
    }

    /// Get the underlying textarea for direct access.
    pub fn inner(&self) -> &tui_textarea::TextArea<'a> {
        &self.textarea
    }

    /// Get mutable access to the underlying textarea.
    pub fn inner_mut(&mut self) -> &mut tui_textarea::TextArea<'a> {
        &mut self.textarea
    }
}

impl<'a> Default for MultiLineInput<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> ratatui::widgets::Widget for &MultiLineInput<'a> {
    fn render(self, area: Rect, buf: &mut ratatui::buffer::Buffer) {
        ratatui::widgets::Widget::render(&self.textarea, area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn char_key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    fn left_key() -> KeyEvent {
        KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)
    }

    fn right_key() -> KeyEvent {
        KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)
    }

    fn backspace_key() -> KeyEvent {
        KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)
    }

    fn delete_key() -> KeyEvent {
        KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE)
    }

    fn home_key() -> KeyEvent {
        KeyEvent::new(KeyCode::Home, KeyModifiers::NONE)
    }

    fn end_key() -> KeyEvent {
        KeyEvent::new(KeyCode::End, KeyModifiers::NONE)
    }

    #[test]
    fn test_single_line_input_new_is_empty() {
        let input = SingleLineInput::new();
        assert!(input.is_empty());
        assert_eq!(input.value(), "");
        assert_eq!(input.cursor_position(), 0);
    }

    #[test]
    fn test_single_line_input_with_value() {
        let input = SingleLineInput::with_value("hello");
        assert!(!input.is_empty());
        assert_eq!(input.value(), "hello");
        assert_eq!(input.cursor_position(), 5);
    }

    #[test]
    fn test_single_line_input_character_insertion() {
        let mut input = SingleLineInput::new();
        input.handle_key(char_key('h'));
        input.handle_key(char_key('i'));
        assert_eq!(input.value(), "hi");
        assert_eq!(input.cursor_position(), 2);
    }

    #[test]
    fn test_single_line_input_cursor_movement() {
        let mut input = SingleLineInput::with_value("hello");
        assert_eq!(input.cursor_position(), 5);

        // Move left twice
        input.handle_key(left_key());
        input.handle_key(left_key());
        assert_eq!(input.cursor_position(), 3);

        // Move right
        input.handle_key(right_key());
        assert_eq!(input.cursor_position(), 4);

        // Home
        input.handle_key(home_key());
        assert_eq!(input.cursor_position(), 0);

        // End
        input.handle_key(end_key());
        assert_eq!(input.cursor_position(), 5);
    }

    #[test]
    fn test_single_line_input_backspace() {
        let mut input = SingleLineInput::with_value("hello");
        input.handle_key(left_key());
        input.handle_key(left_key());
        input.handle_key(backspace_key());

        assert_eq!(input.value(), "helo");
        assert_eq!(input.cursor_position(), 2);
    }

    #[test]
    fn test_single_line_input_delete() {
        let mut input = SingleLineInput::with_value("hello");
        input.handle_key(home_key());
        input.handle_key(delete_key());

        assert_eq!(input.value(), "ello");
        assert_eq!(input.cursor_position(), 0);
    }

    #[test]
    fn test_single_line_input_backspace_at_start() {
        let mut input = SingleLineInput::with_value("a");
        input.handle_key(home_key());
        input.handle_key(backspace_key());

        assert_eq!(input.value(), "a"); // Unchanged
        assert_eq!(input.cursor_position(), 0);
    }

    #[test]
    fn test_single_line_input_delete_at_end() {
        let mut input = SingleLineInput::with_value("a");
        input.handle_key(end_key());
        input.handle_key(delete_key());

        assert_eq!(input.value(), "a"); // Unchanged
        assert_eq!(input.cursor_position(), 1);
    }

    #[test]
    fn test_single_line_input_unicode_handling() {
        let mut input = SingleLineInput::new();
        input.handle_key(char_key('中'));
        input.handle_key(char_key('文'));

        assert_eq!(input.value(), "中文");
        assert_eq!(input.cursor_position(), 2); // Character positions, not bytes
    }

    #[test]
    fn test_single_line_input_unicode_cursor() {
        let mut input = SingleLineInput::with_value("héllo");

        // Move to start
        input.handle_key(home_key());
        assert_eq!(input.cursor_position(), 0);

        // Move right through the accented character
        input.handle_key(right_key());
        input.handle_key(right_key());
        assert_eq!(input.cursor_position(), 2);
    }

    #[test]
    fn test_single_line_input_insert_in_middle() {
        let mut input = SingleLineInput::with_value("hello");
        // Move to position 2
        input.handle_key(home_key());
        input.handle_key(right_key());
        input.handle_key(right_key());
        assert_eq!(input.cursor_position(), 2);

        // Insert 'X'
        input.handle_key(char_key('X'));
        assert_eq!(input.value(), "heXllo");
        assert_eq!(input.cursor_position(), 3);
    }

    #[test]
    fn test_single_line_input_set_value() {
        let mut input = SingleLineInput::new();
        input.set_value("new value");
        assert_eq!(input.value(), "new value");
        assert_eq!(input.cursor_position(), 9);
    }

    #[test]
    fn test_single_line_input_placeholder() {
        let input = SingleLineInput::with_placeholder("Enter text...");
        assert_eq!(input.placeholder(), Some("Enter text..."));
        assert!(input.is_empty());
    }

    #[test]
    fn test_from_string() {
        let input: SingleLineInput = "test string".to_string().into();
        assert_eq!(input.value(), "test string");
    }

    #[test]
    fn test_from_str() {
        let input: SingleLineInput = "test string".into();
        assert_eq!(input.value(), "test string");
    }

    #[test]
    fn test_multi_line_input_new_is_empty() {
        let input: MultiLineInput = MultiLineInput::new();
        assert!(input.is_empty());
        assert_eq!(input.value(), "");
    }

    #[test]
    fn test_multi_line_input_with_value() {
        let input = MultiLineInput::with_value("line1\nline2");
        assert!(!input.is_empty());
        assert_eq!(input.value(), "line1\nline2");
    }

    #[test]
    fn test_multi_line_input_single_line() {
        let input = MultiLineInput::with_value("single line");
        assert_eq!(input.value(), "single line");
    }

    #[test]
    fn test_multi_line_input_set_value() {
        let mut input = MultiLineInput::new();
        input.set_value("new\nvalue");
        assert_eq!(input.value(), "new\nvalue");
    }

    #[test]
    fn test_ctrl_navigation() {
        let mut input = SingleLineInput::with_value("hello world");
        input.handle_key(home_key());
        assert_eq!(input.cursor_position(), 0);

        // Ctrl+Right should move to next word (start of "world")
        let ctrl_right = KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL);
        input.handle_key(ctrl_right);
        // tui-input moves to start of next word which includes the space
        assert_eq!(input.cursor_position(), 6); // After "hello "

        // Ctrl+Left should move to previous word
        let ctrl_left = KeyEvent::new(KeyCode::Left, KeyModifiers::CONTROL);
        input.handle_key(ctrl_left);
        assert_eq!(input.cursor_position(), 0);
    }

    #[test]
    fn test_ctrl_k_clear_to_end() {
        let mut input = SingleLineInput::with_value("hello world");
        input.handle_key(home_key());
        input.handle_key(right_key());
        input.handle_key(right_key());
        input.handle_key(right_key());
        // Cursor at position 3, before "lo world"

        let ctrl_k = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL);
        input.handle_key(ctrl_k);

        assert_eq!(input.value(), "hel");
        assert_eq!(input.cursor_position(), 3);
    }

    #[test]
    fn test_ctrl_e_go_to_end() {
        let mut input = SingleLineInput::with_value("hello");
        input.handle_key(home_key());
        assert_eq!(input.cursor_position(), 0);

        let ctrl_e = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL);
        input.handle_key(ctrl_e);
        assert_eq!(input.cursor_position(), 5);
    }
}
