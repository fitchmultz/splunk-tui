//! Job inspect screen input handler.
//!
//! Responsibilities:
//! - Handle Ctrl+C or 'y' copy of the inspected job's SID (vim-style)
//!
//! Does NOT handle:
//! - Does NOT handle global navigation (handled by keymap)
//! - Does NOT render the UI (handled by render module)
//! - Does NOT fetch job details (handled by actions)

use crate::action::Action;
use crate::app::App;
use crate::ui::Toast;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl App {
    /// Handle input for the job inspect screen.
    pub fn handle_job_inspect_input(&mut self, key: KeyEvent) -> Option<Action> {
        // Ctrl+C or 'y': copy SID of the currently selected job (inspect view) (vim-style)
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

        None
    }
}
