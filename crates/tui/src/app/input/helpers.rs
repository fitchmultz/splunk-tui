//! Input handling helper functions.
//!
//! Responsibilities:
//! - Classify key events for input handling
//! - Determine if keys are printable, mode switches, cursor editing, copy, or export
//! - Provide shared copy/export guard helpers used by per-screen handlers
//!
//! Does NOT handle:
//! - Screen-specific content extraction
//! - Screen-specific business actions
//! - Rendering

use crate::action::Action;
use crate::app::App;
use crate::app::export::ExportTarget;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Check if a key event represents a printable character that should be inserted
/// into text input during QueryFocused mode.
///
/// A key is considered printable only if:
/// - It's a character key (KeyCode::Char)
/// - The character is not a control character
/// - No modifier keys (Ctrl, Alt, etc.) are pressed
pub fn is_printable_char(key: KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char(c) if !c.is_control() && key.modifiers.is_empty())
}

/// Check if a key event is used for mode switching in the search screen.
/// These keys should bypass global bindings when in QueryFocused mode.
pub fn is_mode_switch_key(_key: KeyEvent) -> bool {
    // Tab/BackTab are now handled by global keymap for screen navigation
    // Focus switching is done via Ctrl+Tab/Ctrl+Shift+Tab (NextFocus/PreviousFocus)
    false
}

/// Check if a key event is used for cursor movement/editing in the search query.
/// These keys should bypass global bindings when in QueryFocused mode (RQ-0110).
pub fn is_cursor_editing_key(key: KeyEvent) -> bool {
    matches!(
        key.code,
        KeyCode::Left
            | KeyCode::Right
            | KeyCode::Home
            | KeyCode::End
            | KeyCode::Delete
            | KeyCode::Backspace
    )
}

/// Shared copy shortcut:
/// - Ctrl+C
/// - bare 'y' (vim-style yank)
pub fn is_copy_key(key: KeyEvent) -> bool {
    (key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')))
        || (key.modifiers.is_empty() && matches!(key.code, KeyCode::Char('y')))
}

/// Shared export shortcut:
/// - Ctrl+E
pub fn is_export_key(key: KeyEvent) -> bool {
    key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('e'))
}

/// Return true only when a list exists and contains at least one item.
pub fn should_export_list<T>(collection: Option<&Vec<T>>) -> bool {
    collection.is_some_and(|items| !items.is_empty())
}

/// Return true only when a single payload exists.
pub fn should_export_single<T>(data: Option<&T>) -> bool {
    data.is_some()
}

/// Shared copy handler used after a screen extracts its copyable content.
pub fn handle_copy_with_toast(app: &mut App, content: Option<String>) -> Option<Action> {
    if let Some(content) = content.filter(|value| !value.trim().is_empty()) {
        return Some(Action::CopyToClipboard(content));
    }

    app.push_info_toast_once("Nothing to copy");
    None
}

/// Shared list export handler.
pub fn handle_list_export(app: &mut App, can_export: bool, target: ExportTarget) -> Option<Action> {
    if can_export {
        app.begin_export(target);
    }
    None
}

/// Shared single-payload export handler.
pub fn handle_single_export(
    app: &mut App,
    can_export: bool,
    target: ExportTarget,
) -> Option<Action> {
    if can_export {
        app.begin_export(target);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConnectionContext;

    fn ctrl_key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    fn key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    #[test]
    fn test_is_copy_key_recognizes_ctrl_c_and_y() {
        assert!(is_copy_key(ctrl_key('c')));
        assert!(is_copy_key(key('y')));
        assert!(!is_copy_key(key('c')));
        assert!(!is_copy_key(ctrl_key('e')));
    }

    #[test]
    fn test_is_export_key_recognizes_ctrl_e_only() {
        assert!(is_export_key(ctrl_key('e')));
        assert!(!is_export_key(key('e')));
        assert!(!is_export_key(ctrl_key('c')));
    }

    #[test]
    fn test_handle_copy_with_toast_returns_copy_action_for_non_empty_content() {
        let mut app = App::new(None, ConnectionContext::default());

        let action = handle_copy_with_toast(&mut app, Some("hello".to_string()));

        assert!(matches!(action, Some(Action::CopyToClipboard(s)) if s == "hello"));
        assert!(app.toasts.is_empty());
    }

    #[test]
    fn test_handle_copy_with_toast_toasts_for_empty_or_missing_content() {
        let mut app = App::new(None, ConnectionContext::default());

        let action = handle_copy_with_toast(&mut app, Some("   ".to_string()));
        assert!(action.is_none());
        assert!(!app.toasts.is_empty());

        let before = app.toasts.len();
        let action = handle_copy_with_toast(&mut app, None);
        assert!(action.is_none());
        assert_eq!(app.toasts.len(), before);
    }

    #[test]
    fn test_should_export_list_requires_non_empty_collection() {
        let empty: Vec<()> = vec![];
        let filled = vec![()];

        assert!(!should_export_list::<()>(None));
        assert!(!should_export_list(Some(&empty)));
        assert!(should_export_list(Some(&filled)));
    }

    #[test]
    fn test_handle_list_export_obeys_guard() {
        let mut app = App::new(None, ConnectionContext::default());

        handle_list_export(&mut app, false, ExportTarget::Apps);
        assert_eq!(app.export_target, None);

        handle_list_export(&mut app, true, ExportTarget::Apps);
        assert_eq!(app.export_target, Some(ExportTarget::Apps));
    }

    #[test]
    fn test_handle_single_export_obeys_guard() {
        let mut app = App::new(None, ConnectionContext::default());

        handle_single_export(&mut app, false, ExportTarget::Health);
        assert_eq!(app.export_target, None);

        handle_single_export(&mut app, true, ExportTarget::Health);
        assert_eq!(app.export_target, Some(ExportTarget::Health));
    }

    #[test]
    fn test_should_export_single_requires_presence() {
        assert!(!should_export_single::<usize>(None));
        assert!(should_export_single(Some(&42)));
    }
}
