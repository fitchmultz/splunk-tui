//! Jobs screen input handler.
//!
//! Responsibilities:
//! - Handle job selection and multi-selection (space key)
//! - Handle job cancel (c key) and delete (d key)
//! - Handle auto-refresh toggle (a key)
//! - Handle Ctrl+C copy of selected job SID
//! - Handle filter input mode
//!
//! Non-responsibilities:
//! - Does NOT handle global navigation (handled by keymap)
//! - Does NOT render the UI (handled by render module)
//! - Does NOT execute the actual cancel/delete operations (handled by actions)

use crate::action::Action;
use crate::app::App;
use crate::app::export::ExportTarget;
use crate::ui::Toast;
use crate::ui::popup::{Popup, PopupType};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl App {
    /// Handle input for the jobs screen.
    pub fn handle_jobs_input(&mut self, key: KeyEvent) -> Option<Action> {
        // Ctrl+C or 'y': copy selected job SID (vim-style)
        let is_copy = (key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('c')))
            || (key.modifiers.is_empty() && matches!(key.code, KeyCode::Char('y')));
        if is_copy {
            if let Some(job) = self.get_selected_job() {
                return Some(Action::CopyToClipboard(job.sid.clone()));
            }
            self.toasts.push(Toast::info("Nothing to copy"));
            return None;
        }

        // Normal jobs screen input
        match key.code {
            KeyCode::Char('e')
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && self.jobs.as_ref().map(|v| !v.is_empty()).unwrap_or(false) =>
            {
                self.begin_export(ExportTarget::Jobs);
                None
            }
            KeyCode::Char('a') => {
                self.auto_refresh = !self.auto_refresh;
                None
            }
            KeyCode::Char('c') => {
                if !self.selected_jobs.is_empty() {
                    self.popup = Some(
                        Popup::builder(PopupType::ConfirmCancelBatch(
                            self.selected_jobs.iter().cloned().collect(),
                        ))
                        .build(),
                    );
                } else if let Some(job) = self.get_selected_job() {
                    self.popup =
                        Some(Popup::builder(PopupType::ConfirmCancel(job.sid.clone())).build());
                }
                None
            }
            KeyCode::Char('d') => {
                if !self.selected_jobs.is_empty() {
                    self.popup = Some(
                        Popup::builder(PopupType::ConfirmDeleteBatch(
                            self.selected_jobs.iter().cloned().collect(),
                        ))
                        .build(),
                    );
                } else if let Some(job) = self.get_selected_job() {
                    self.popup =
                        Some(Popup::builder(PopupType::ConfirmDelete(job.sid.clone())).build());
                }
                None
            }
            KeyCode::Char(' ') => {
                if let Some(job) = self.get_selected_job() {
                    let sid = job.sid.clone();
                    if self.selected_jobs.contains(&sid) {
                        self.selected_jobs.remove(&sid);
                    } else {
                        self.selected_jobs.insert(sid);
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Handle input when in jobs filter mode.
    pub(crate) fn handle_jobs_filter_input(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Esc => {
                self.is_filtering = false;
                // If we have a saved filter value, restore it (cancel edit)
                // Otherwise clear the filter (no previous filter to restore)
                if let Some(saved) = self.filter_before_edit.take() {
                    self.search_filter = Some(saved);
                    self.filter_input.clear();
                    self.rebuild_filtered_indices();
                    None
                } else {
                    self.filter_input.clear();
                    Some(Action::ClearSearch)
                }
            }
            KeyCode::Enter => {
                self.is_filtering = false;
                self.filter_before_edit = None; // Commit the edit, clear saved state
                if !self.filter_input.is_empty() {
                    self.search_filter = Some(self.filter_input.clone());
                    self.filter_input.clear();
                    self.rebuild_filtered_indices();
                    None
                } else {
                    Some(Action::ClearSearch)
                }
            }
            KeyCode::Backspace => {
                self.filter_input.pop();
                None
            }
            KeyCode::Char(c) => {
                self.filter_input.push(c);
                None
            }
            _ => None,
        }
    }
}
