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
                search: self.search_input.clone(),
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
                        KeyCode::Backspace => {
                            self.history_index = None;
                            if self.search_cursor_position > 0 {
                                // Remove character before cursor using safe char-based indexing
                                let char_count = self.search_input.chars().count();
                                if self.search_cursor_position <= char_count {
                                    let byte_pos = self
                                        .search_input
                                        .char_indices()
                                        .nth(self.search_cursor_position)
                                        .map(|(i, _)| i)
                                        .unwrap_or(self.search_input.len());
                                    let prev_char_start = self.search_input[..byte_pos]
                                        .char_indices()
                                        .next_back()
                                        .map(|(i, _)| i)
                                        .unwrap_or(0);
                                    self.search_input.remove(prev_char_start);
                                    self.search_cursor_position -= 1;
                                }
                            }
                            self.trigger_validation();
                            None
                        }
                        KeyCode::Delete => {
                            self.history_index = None;
                            let char_count = self.search_input.chars().count();
                            if self.search_cursor_position < char_count {
                                // Remove character at cursor using safe char-based indexing
                                if let Some((byte_pos, _)) = self
                                    .search_input
                                    .char_indices()
                                    .nth(self.search_cursor_position)
                                {
                                    self.search_input.remove(byte_pos);
                                }
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
                            let char_count = self.search_input.chars().count();
                            if self.search_cursor_position < char_count {
                                self.search_cursor_position += 1;
                            }
                            None
                        }
                        KeyCode::Home => {
                            self.search_cursor_position = 0;
                            None
                        }
                        KeyCode::End => {
                            self.search_cursor_position = self.search_input.chars().count();
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
                            // Move cursor to end of new text (using char count for consistency)
                            self.search_cursor_position = self.search_input.chars().count();
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
                            // Move cursor to end of new text (using char count for consistency)
                            self.search_cursor_position = self.search_input.chars().count();
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
                            // Insert character at cursor position using safe char-based indexing
                            let char_count = self.search_input.chars().count();
                            if self.search_cursor_position >= char_count {
                                // Append at end
                                self.search_input.push(c);
                            } else if let Some((byte_pos, _)) = self
                                .search_input
                                .char_indices()
                                .nth(self.search_cursor_position)
                            {
                                self.search_input.insert(byte_pos, c);
                            } else {
                                // Fallback to append
                                self.search_input.push(c);
                            }
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
