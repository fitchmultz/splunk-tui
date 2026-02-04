//! Search screen input handler.
//!
//! Responsibilities:
//! - Handle query input and editing (QueryFocused mode)
//! - Handle result navigation (ResultsFocused mode)
//! - Handle search history navigation
//! - Handle Ctrl+C copy from results
//! - Trigger SPL validation on input changes (debounced)
//!
//! Non-responsibilities:
//! - Does NOT handle global navigation (handled by keymap)
//! - Does NOT render the UI (handled by render module)
//! - Does NOT perform actual validation (handled by side effects)

use crate::action::Action;
use crate::app::App;
use crate::app::export::ExportTarget;
use crate::app::state::SearchInputMode;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use splunk_client::SearchMode;
use std::time::Instant;

/// Debounce delay for SPL validation in milliseconds.
const VALIDATION_DEBOUNCE_MS: u64 = 500;

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
            return Some(Action::ValidateSpl {
                search: self.search_input.value().to_string(),
            });
        }

        None
    }

    /// Handle input for the search screen.
    pub fn handle_search_input(&mut self, key: KeyEvent) -> Option<Action> {
        // Handle Ctrl+C or 'y' copy shortcut while in input (vim-style)
        let is_copy = (key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('c')))
            || (key.modifiers.is_empty() && matches!(key.code, KeyCode::Char('y')));
        if is_copy {
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
                return None;
            }

            return Some(Action::CopyToClipboard(content));
        }

        // Handle mode switching and input based on current mode
        match key.code {
            // Tab toggles between QueryFocused and ResultsFocused modes
            KeyCode::Tab => {
                self.search_input_mode = self.search_input_mode.toggle();
                None
            }
            // Esc switches back to QueryFocused mode (or clears if already focused)
            KeyCode::Esc => {
                if matches!(self.search_input_mode, SearchInputMode::ResultsFocused) {
                    self.search_input_mode = SearchInputMode::QueryFocused;
                }
                None
            }
            _ => match self.search_input_mode {
                SearchInputMode::QueryFocused => {
                    // In QueryFocused mode, handle text input
                    match key.code {
                        KeyCode::Enter => {
                            if !self.search_input.is_empty() {
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
                            } else {
                                None
                            }
                        }
                        KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            // Toggle search mode between Normal and Realtime
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
                        KeyCode::Down => {
                            // Navigate forward in history (towards newer queries)
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
                        KeyCode::Up => {
                            // Navigate backward in history (towards older queries)
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
                        KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            if !self.search_results.is_empty() {
                                self.begin_export(ExportTarget::SearchResults);
                            }
                            None
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
                SearchInputMode::ResultsFocused => {
                    // In ResultsFocused mode, navigation keys are handled by global bindings
                    // We just return None here and let the global bindings handle navigation
                    None
                }
            },
        }
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
    fn test_tab_toggles_input_mode() {
        let mut app = App::new(None, ConnectionContext::default());
        assert!(matches!(
            app.search_input_mode,
            SearchInputMode::QueryFocused
        ));

        app.handle_search_input(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert!(matches!(
            app.search_input_mode,
            SearchInputMode::ResultsFocused
        ));

        app.handle_search_input(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert!(matches!(
            app.search_input_mode,
            SearchInputMode::QueryFocused
        ));
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
}
