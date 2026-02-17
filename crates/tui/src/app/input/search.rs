//! Search screen input handler.
//!
//! Responsibilities:
//! - Handle query input and editing (QueryFocused mode)
//! - Handle result navigation (ResultsFocused mode)
//! - Handle search history navigation
//! - Handle Ctrl+C copy from results
//! - Trigger SPL validation on input changes (debounced)
//!
//! Does NOT handle:
//! - Does NOT handle global navigation (handled by keymap)
//! - Does NOT render the UI (handled by render module)
//! - Does NOT perform actual validation (handled by side effects)

use crate::action::Action;
use crate::app::App;
use crate::app::export::ExportTarget;
use crate::app::state::SearchInputMode;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use splunk_client::SearchMode;
use splunk_config::constants::DEFAULT_VALIDATION_DEBOUNCE_MS;
use std::time::Instant;

/// Debounce delay for SPL validation in milliseconds.
const VALIDATION_DEBOUNCE_MS: u64 = DEFAULT_VALIDATION_DEBOUNCE_MS;

impl App {
    /// Trigger SPL validation with debouncing.
    ///
    /// Called whenever the search input changes. Sets up the validation
    /// pending flag and timestamp so the tick handler can dispatch
    /// the actual validation request after the debounce delay.
    fn trigger_validation(&mut self) {
        // Reset validation state for new input
        self.spl_validation_pending = true;
        self.last_input_change = Some(Instant::now());
    }

    /// Clear validation state.
    ///
    /// Should be called when leaving the search screen or when
    /// the search input is no longer relevant.
    pub fn clear_validation_state(&mut self) {
        self.spl_validation_pending = false;
        self.last_input_change = None;
        self.spl_validation_state.valid = None;
        self.spl_validation_state.errors.clear();
        self.spl_validation_state.warnings.clear();
    }

    /// Handle debounced validation in tick.
    ///
    /// Called from the main tick handler. If validation is pending and
    /// the debounce delay has passed, dispatches the validation action.
    pub fn handle_validation_tick(&mut self) -> Option<Action> {
        if !self.spl_validation_pending {
            return None;
        }

        if let Some(last_change) = self.last_input_change
            && last_change.elapsed().as_millis() >= VALIDATION_DEBOUNCE_MS as u128
        {
            self.spl_validation_pending = false;
            self.validation_request_id += 1;
            return Some(Action::ValidateSpl {
                search: self.search_input.value().to_string(),
                request_id: self.validation_request_id,
            });
        }

        None
    }

    /// Handle input for the search screen.
    pub fn handle_search_input(&mut self, key: KeyEvent) -> Option<Action> {
        // Handle copy shortcut early (early return pattern)
        if let Some(action) = self.try_handle_search_copy(key) {
            return action;
        }

        // Handle mode switching and input based on current mode
        match key.code {
            // Tab now navigates to next screen (handled by global keymap)
            KeyCode::Tab | KeyCode::BackTab => {
                // Let global keymap handle screen navigation
                None // Return None so the key event propagates to keymap resolution
            }
            // Esc switches back to QueryFocused mode
            KeyCode::Esc => self.handle_search_esc(),
            // Delegate to mode-specific handler
            _ => self.handle_search_by_mode(key),
        }
    }

    /// Try to handle copy shortcut (Ctrl+C or 'y').
    ///
    /// Returns `Some(action)` if copy was handled, `None` otherwise.
    fn try_handle_search_copy(&mut self, key: KeyEvent) -> Option<Option<Action>> {
        let is_copy = (key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('c')))
            || (key.modifiers.is_empty() && matches!(key.code, KeyCode::Char('y')));

        if !is_copy {
            return None;
        }

        // Decision:
        // - If results exist, copy the JSON for the "current" result (at scroll offset).
        // - Otherwise, copy the current search query.
        let content = if !self.search_results.is_empty() {
            let idx = self
                .search_scroll_offset
                .min(self.search_results.len().saturating_sub(1));
            self.search_results
                .get(idx)
                .and_then(|v| serde_json::to_string_pretty(v).ok())
                .unwrap_or_else(|| "<invalid>".to_string())
        } else {
            self.search_input.value().to_string()
        };

        if content.trim().is_empty() {
            self.toasts.push(crate::ui::Toast::info("Nothing to copy"));
            return Some(None);
        }

        Some(Some(Action::CopyToClipboard(content)))
    }

    /// Handle Esc key in search screen.
    fn handle_search_esc(&mut self) -> Option<Action> {
        if matches!(self.search_input_mode, SearchInputMode::ResultsFocused) {
            self.search_input_mode = SearchInputMode::QueryFocused;
        }
        None
    }

    /// Dispatch to mode-specific handler based on current input mode.
    fn handle_search_by_mode(&mut self, key: KeyEvent) -> Option<Action> {
        match self.search_input_mode {
            SearchInputMode::QueryFocused => self.handle_search_query_focused(key),
            SearchInputMode::ResultsFocused => self.handle_search_results_focused(key),
        }
    }

    /// Handle input when in QueryFocused mode.
    fn handle_search_query_focused(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Enter => self.execute_search(),
            KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.toggle_search_mode()
            }
            KeyCode::Down => self.navigate_search_history_forward(),
            KeyCode::Up => self.navigate_search_history_backward(),
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.begin_search_export()
            }
            _ => {
                // For all other keys, use tui-input's InputRequest handling
                // This handles: character input, backspace, delete, cursor movement
                self.history_index = None;
                self.search_input.handle_key(key);
                self.trigger_validation();
                None
            }
        }
    }

    /// Handle input when in ResultsFocused mode.
    fn handle_search_results_focused(&mut self, _key: KeyEvent) -> Option<Action> {
        // In ResultsFocused mode, navigation keys are handled by global bindings
        // We just return None here and let the global bindings handle navigation
        None
    }

    /// Execute the search with current query.
    fn execute_search(&mut self) -> Option<Action> {
        if self.search_input.is_empty() {
            return None;
        }

        let query = self.search_input.value().to_string();
        self.add_to_history(query.clone());
        self.search_status = format!("Running: {}", query);
        // Switch to ResultsFocused after running search
        self.search_input_mode = SearchInputMode::ResultsFocused;
        Some(Action::RunSearch {
            query,
            search_defaults: self.search_defaults.clone(),
            search_mode: self.search_mode,
            realtime_window: self.realtime_window,
        })
    }

    /// Toggle search mode between Normal and Realtime.
    fn toggle_search_mode(&mut self) -> Option<Action> {
        self.search_mode = match self.search_mode {
            SearchMode::Normal => SearchMode::Realtime,
            SearchMode::Realtime => SearchMode::Normal,
        };
        let mode_str = match self.search_mode {
            SearchMode::Normal => "Normal",
            SearchMode::Realtime => "Realtime",
        };
        self.toasts
            .push(crate::ui::Toast::info(format!("Search mode: {}", mode_str)));
        None
    }

    /// Navigate forward in search history (towards newer queries).
    fn navigate_search_history_forward(&mut self) -> Option<Action> {
        if let Some(curr) = self.history_index {
            if curr > 0 {
                self.history_index = Some(curr - 1);
                self.search_input
                    .set_value(self.search_history[curr - 1].clone());
            } else {
                self.history_index = None;
                // Restore saved input
                self.search_input.set_value(self.saved_search_input.value());
            }
        }
        self.trigger_validation();
        None
    }

    /// Navigate backward in search history (towards older queries).
    fn navigate_search_history_backward(&mut self) -> Option<Action> {
        if self.search_history.is_empty() {
            return None;
        }

        if let Some(curr) = self.history_index {
            if curr < self.search_history.len().saturating_sub(1) {
                self.history_index = Some(curr + 1);
            }
        } else {
            // Save current input before navigating
            self.saved_search_input.set_value(self.search_input.value());
            self.history_index = Some(0);
        }

        if let Some(idx) = self.history_index {
            self.search_input
                .set_value(self.search_history[idx].clone());
        }
        self.trigger_validation();
        None
    }

    /// Begin export of search results.
    fn begin_search_export(&mut self) -> Option<Action> {
        if self.search_results.is_empty() {
            return None;
        }
        self.begin_export(ExportTarget::SearchResults);
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ConnectionContext;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    fn ctrl_key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    fn backspace_key() -> KeyEvent {
        KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)
    }

    fn delete_key() -> KeyEvent {
        KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE)
    }

    fn left_key() -> KeyEvent {
        KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)
    }

    fn right_key() -> KeyEvent {
        KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)
    }

    fn home_key() -> KeyEvent {
        KeyEvent::new(KeyCode::Home, KeyModifiers::NONE)
    }

    fn end_key() -> KeyEvent {
        KeyEvent::new(KeyCode::End, KeyModifiers::NONE)
    }

    #[test]
    fn test_search_input_character_typing() {
        let mut app = App::new(None, ConnectionContext::default());

        app.handle_search_input(key('i'));
        app.handle_search_input(key('n'));
        app.handle_search_input(key('d'));
        app.handle_search_input(key('e'));
        app.handle_search_input(key('x'));

        assert_eq!(app.search_input.value(), "index");
        assert_eq!(app.search_input.cursor_position(), 5);
    }

    #[test]
    fn test_search_input_cursor_movement() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_input.set_value("hello");

        // Move cursor left
        app.handle_search_input(left_key());
        app.handle_search_input(left_key());
        assert_eq!(app.search_input.cursor_position(), 3);

        // Move cursor right
        app.handle_search_input(right_key());
        assert_eq!(app.search_input.cursor_position(), 4);

        // Home key
        app.handle_search_input(home_key());
        assert_eq!(app.search_input.cursor_position(), 0);

        // End key
        app.handle_search_input(end_key());
        assert_eq!(app.search_input.cursor_position(), 5);
    }

    #[test]
    fn test_search_input_backspace() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_input.set_value("hello");

        // Move cursor to middle and backspace
        app.handle_search_input(left_key());
        app.handle_search_input(left_key());
        app.handle_search_input(backspace_key());

        assert_eq!(app.search_input.value(), "helo");
        assert_eq!(app.search_input.cursor_position(), 2);
    }

    #[test]
    fn test_search_input_delete() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_input.set_value("hello");

        // Move cursor to start and delete
        app.handle_search_input(home_key());
        app.handle_search_input(delete_key());

        assert_eq!(app.search_input.value(), "ello");
        assert_eq!(app.search_input.cursor_position(), 0);
    }

    #[test]
    fn test_search_input_unicode_handling() {
        let mut app = App::new(None, ConnectionContext::default());

        // Type unicode characters
        app.handle_search_input(key('中'));
        app.handle_search_input(key('文'));

        assert_eq!(app.search_input.value(), "中文");
        assert_eq!(app.search_input.cursor_position(), 2);
    }

    #[test]
    fn test_empty_input_enter_does_nothing() {
        let mut app = App::new(None, ConnectionContext::default());

        let action = app.handle_search_input(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert!(action.is_none());
    }

    #[test]
    fn test_enter_with_input_triggers_search() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_input.set_value("index=_internal");

        let action = app.handle_search_input(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert!(matches!(action, Some(Action::RunSearch { .. })));
    }

    #[test]
    fn test_search_history_navigation() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_input.set_value("current");

        // Add some history
        app.search_history = vec!["old1".to_string(), "old2".to_string()];

        // Press Up to go back in history
        app.handle_search_input(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(app.search_input.value(), "old1");

        // Press Up again for older history
        app.handle_search_input(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        assert_eq!(app.search_input.value(), "old2");

        // Press Down to go forward
        app.handle_search_input(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.search_input.value(), "old1");

        // Press Down to restore original input
        app.handle_search_input(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        assert_eq!(app.search_input.value(), "current");
    }

    #[test]
    fn test_ctrl_r_toggles_search_mode() {
        let mut app = App::new(None, ConnectionContext::default());
        assert!(matches!(app.search_mode, SearchMode::Normal));

        app.handle_search_input(ctrl_key('r'));
        assert!(matches!(app.search_mode, SearchMode::Realtime));

        app.handle_search_input(ctrl_key('r'));
        assert!(matches!(app.search_mode, SearchMode::Normal));
    }

    #[test]
    fn test_tab_returns_none_for_global_keymap() {
        // Tab now returns None so the global keymap can handle NextScreen action
        let mut app = App::new(None, ConnectionContext::default());
        assert!(matches!(
            app.search_input_mode,
            SearchInputMode::QueryFocused
        ));

        // Tab should return None (not toggle mode anymore)
        let action = app.handle_search_input(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert!(
            action.is_none(),
            "Tab should return None for global keymap to handle"
        );
        // Mode should stay as QueryFocused
        assert!(
            matches!(app.search_input_mode, SearchInputMode::QueryFocused),
            "Mode should not change when Tab is pressed"
        );
    }

    #[test]
    fn test_esc_from_results_focused_switches_to_query_focused() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_input_mode = SearchInputMode::ResultsFocused;

        app.handle_search_input(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

        assert!(matches!(
            app.search_input_mode,
            SearchInputMode::QueryFocused
        ));
    }

    #[test]
    fn test_backtab_returns_none_for_global_keymap() {
        // Shift+Tab (BackTab) also returns None for global keymap to handle
        let mut app = App::new(None, ConnectionContext::default());
        app.search_input_mode = SearchInputMode::ResultsFocused;

        let action = app.handle_search_input(KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE));
        assert!(
            action.is_none(),
            "BackTab should return None for global keymap to handle"
        );
    }

    #[test]
    fn test_ctrl_c_copies_search_query_when_no_results() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_input.set_value("test query");
        app.search_results.clear();

        let action = app.handle_search_input(ctrl_key('c'));

        assert!(matches!(action, Some(Action::CopyToClipboard(s)) if s == "test query"));
    }

    #[test]
    fn test_validation_triggers_on_input_change() {
        let mut app = App::new(None, ConnectionContext::default());

        assert!(!app.spl_validation_pending);

        app.handle_search_input(key('a'));

        assert!(app.spl_validation_pending);
        assert!(app.last_input_change.is_some());
    }

    // Tests for extracted helper functions

    #[test]
    fn test_try_handle_search_copy_with_results() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_results = vec![serde_json::json!({"key": "value"})];
        app.search_scroll_offset = 0;

        let result = app.try_handle_search_copy(ctrl_key('c'));

        assert!(result.is_some());
        let action = result.unwrap();
        assert!(action.is_some());
        assert!(matches!(action, Some(Action::CopyToClipboard(s)) if s.contains("key")));
    }

    #[test]
    fn test_try_handle_search_copy_without_results() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_input.set_value("test query");
        app.search_results.clear();

        let result = app.try_handle_search_copy(ctrl_key('c'));

        assert!(result.is_some());
        let action = result.unwrap();
        assert!(matches!(action, Some(Action::CopyToClipboard(s)) if s == "test query"));
    }

    #[test]
    fn test_try_handle_search_copy_with_empty_content() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_input.set_value("   ");
        app.search_results.clear();

        let result = app.try_handle_search_copy(ctrl_key('c'));

        assert!(result.is_some());
        let action = result.unwrap();
        assert!(action.is_none()); // Returns None when empty
        assert_eq!(app.toasts.len(), 1);
    }

    #[test]
    fn test_try_handle_search_copy_not_copy_key() {
        let mut app = App::new(None, ConnectionContext::default());

        let result = app.try_handle_search_copy(key('a'));

        assert!(result.is_none()); // Not handled
    }

    #[test]
    fn test_handle_search_esc_from_results_focused() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_input_mode = SearchInputMode::ResultsFocused;

        let action = app.handle_search_esc();

        assert!(action.is_none());
        assert!(matches!(
            app.search_input_mode,
            SearchInputMode::QueryFocused
        ));
    }

    #[test]
    fn test_handle_search_esc_from_query_focused() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_input_mode = SearchInputMode::QueryFocused;

        let action = app.handle_search_esc();

        assert!(action.is_none());
        assert!(matches!(
            app.search_input_mode,
            SearchInputMode::QueryFocused
        ));
    }

    #[test]
    fn test_execute_search_with_input() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_input.set_value("index=main");

        let action = app.execute_search();

        assert!(matches!(action, Some(Action::RunSearch { .. })));
        assert!(matches!(
            app.search_input_mode,
            SearchInputMode::ResultsFocused
        ));
        assert_eq!(app.search_history.len(), 1);
    }

    #[test]
    fn test_execute_search_empty_input() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_input.set_value("");

        let action = app.execute_search();

        assert!(action.is_none());
    }

    #[test]
    fn test_toggle_search_mode_normal_to_realtime() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_mode = SearchMode::Normal;

        let action = app.toggle_search_mode();

        assert!(action.is_none());
        assert!(matches!(app.search_mode, SearchMode::Realtime));
        assert_eq!(app.toasts.len(), 1);
    }

    #[test]
    fn test_toggle_search_mode_realtime_to_normal() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_mode = SearchMode::Realtime;

        let action = app.toggle_search_mode();

        assert!(action.is_none());
        assert!(matches!(app.search_mode, SearchMode::Normal));
        assert_eq!(app.toasts.len(), 1);
    }

    #[test]
    fn test_navigate_search_history_backward_from_none() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_input.set_value("current");
        app.search_history = vec!["old1".to_string(), "old2".to_string()];

        let action = app.navigate_search_history_backward();

        assert!(action.is_none());
        assert_eq!(app.history_index, Some(0));
        assert_eq!(app.search_input.value(), "old1");
        assert_eq!(app.saved_search_input.value(), "current");
    }

    #[test]
    fn test_navigate_search_history_backward_already_navigating() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_history = vec!["old1".to_string(), "old2".to_string()];
        app.history_index = Some(0);

        let action = app.navigate_search_history_backward();

        assert!(action.is_none());
        assert_eq!(app.history_index, Some(1));
        assert_eq!(app.search_input.value(), "old2");
    }

    #[test]
    fn test_navigate_search_history_backward_at_end() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_history = vec!["old1".to_string(), "old2".to_string()];
        app.history_index = Some(1);

        let action = app.navigate_search_history_backward();

        assert!(action.is_none());
        assert_eq!(app.history_index, Some(1)); // Stays at end
        assert_eq!(app.search_input.value(), "old2");
    }

    #[test]
    fn test_navigate_search_history_backward_empty_history() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_history.clear();

        let action = app.navigate_search_history_backward();

        assert!(action.is_none());
        assert_eq!(app.history_index, None);
    }

    #[test]
    fn test_navigate_search_history_forward_from_middle() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_history = vec!["old1".to_string(), "old2".to_string()];
        app.history_index = Some(1);
        app.search_input.set_value("old2");

        let action = app.navigate_search_history_forward();

        assert!(action.is_none());
        assert_eq!(app.history_index, Some(0));
        assert_eq!(app.search_input.value(), "old1");
    }

    #[test]
    fn test_navigate_search_history_forward_to_original() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_input.set_value("original");
        app.saved_search_input.set_value("original");
        app.search_history = vec!["old1".to_string()];
        app.history_index = Some(0);
        app.search_input.set_value("old1");

        let action = app.navigate_search_history_forward();

        assert!(action.is_none());
        assert_eq!(app.history_index, None);
        assert_eq!(app.search_input.value(), "original");
    }

    #[test]
    fn test_navigate_search_history_forward_from_none() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_history = vec!["old1".to_string()];

        let action = app.navigate_search_history_forward();

        assert!(action.is_none());
        assert_eq!(app.history_index, None);
    }

    #[test]
    fn test_begin_search_export_with_results() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_results = vec![serde_json::json!({"key": "value"})];

        let action = app.begin_search_export();

        assert!(action.is_none());
        // begin_export is called but doesn't return an action directly
    }

    #[test]
    fn test_begin_search_export_without_results() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_results.clear();

        let action = app.begin_search_export();

        assert!(action.is_none());
    }

    #[test]
    fn test_handle_search_query_focused_enter() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_input.set_value("index=main");

        let action =
            app.handle_search_query_focused(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

        assert!(matches!(action, Some(Action::RunSearch { .. })));
    }

    #[test]
    fn test_handle_search_query_focused_ctrl_r() {
        let mut app = App::new(None, ConnectionContext::default());

        let action = app.handle_search_query_focused(ctrl_key('r'));

        assert!(action.is_none());
        assert!(matches!(app.search_mode, SearchMode::Realtime));
    }

    #[test]
    fn test_handle_search_query_focused_down() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_history = vec!["old1".to_string()];
        app.history_index = Some(0);

        let action =
            app.handle_search_query_focused(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));

        assert!(action.is_none());
    }

    #[test]
    fn test_handle_search_query_focused_up() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_history = vec!["old1".to_string()];

        let action =
            app.handle_search_query_focused(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));

        assert!(action.is_none());
    }

    #[test]
    fn test_handle_search_query_focused_ctrl_e() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_results = vec![serde_json::json!({})];

        let action = app.handle_search_query_focused(ctrl_key('e'));

        assert!(action.is_none());
    }

    #[test]
    fn test_handle_search_results_focused_returns_none() {
        let mut app = App::new(None, ConnectionContext::default());

        let action = app.handle_search_results_focused(key('a'));

        assert!(action.is_none());
    }
}
