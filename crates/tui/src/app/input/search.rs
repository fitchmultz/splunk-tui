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
                search: self.search_input.clone(),
            });
        }

        None
    }

    /// Handle input for the search screen.
    pub fn handle_search_input(&mut self, key: KeyEvent) -> Option<Action> {
        // Handle Ctrl+* shortcuts while in input
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
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
                self.search_input.clone()
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
                                let query = self.search_input.clone();
                                self.add_to_history(query.clone());
                                self.search_status = format!("Running: {}", query);
                                // Switch to ResultsFocused after running search
                                self.search_input_mode = SearchInputMode::ResultsFocused;
                                Some(Action::RunSearch {
                                    query,
                                    search_defaults: self.search_defaults.clone(),
                                })
                            } else {
                                None
                            }
                        }
                        KeyCode::Backspace => {
                            self.history_index = None;
                            if self.search_cursor_position > 0 {
                                // Remove character before cursor
                                let pos = self.search_cursor_position;
                                self.search_input.remove(pos - 1);
                                self.search_cursor_position -= 1;
                            }
                            self.trigger_validation();
                            None
                        }
                        KeyCode::Delete => {
                            self.history_index = None;
                            if self.search_cursor_position < self.search_input.len() {
                                // Remove character at cursor
                                let pos = self.search_cursor_position;
                                self.search_input.remove(pos);
                            }
                            self.trigger_validation();
                            None
                        }
                        KeyCode::Left => {
                            if self.search_cursor_position > 0 {
                                self.search_cursor_position -= 1;
                            }
                            None
                        }
                        KeyCode::Right => {
                            if self.search_cursor_position < self.search_input.len() {
                                self.search_cursor_position += 1;
                            }
                            None
                        }
                        KeyCode::Home => {
                            self.search_cursor_position = 0;
                            None
                        }
                        KeyCode::End => {
                            self.search_cursor_position = self.search_input.len();
                            None
                        }
                        KeyCode::Down => {
                            if let Some(curr) = self.history_index {
                                if curr > 0 {
                                    self.history_index = Some(curr - 1);
                                    self.search_input = self.search_history[curr - 1].clone();
                                } else {
                                    self.history_index = None;
                                    self.search_input = self.saved_search_input.clone();
                                }
                            }
                            // Move cursor to end of new text
                            self.search_cursor_position = self.search_input.len();
                            self.trigger_validation();
                            None
                        }
                        KeyCode::Up => {
                            if self.search_history.is_empty() {
                                return None;
                            }

                            if let Some(curr) = self.history_index {
                                if curr < self.search_history.len().saturating_sub(1) {
                                    self.history_index = Some(curr + 1);
                                }
                            } else {
                                self.saved_search_input = self.search_input.clone();
                                self.history_index = Some(0);
                            }

                            if let Some(idx) = self.history_index {
                                self.search_input = self.search_history[idx].clone();
                            }
                            // Move cursor to end of new text
                            self.search_cursor_position = self.search_input.len();
                            self.trigger_validation();
                            None
                        }
                        KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                            if !self.search_results.is_empty() {
                                self.begin_export(ExportTarget::SearchResults);
                            }
                            None
                        }
                        KeyCode::Char(c) => {
                            self.history_index = None;
                            // Insert character at cursor position
                            self.search_input.insert(self.search_cursor_position, c);
                            self.search_cursor_position += 1;
                            self.trigger_validation();
                            None
                        }
                        _ => None,
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
