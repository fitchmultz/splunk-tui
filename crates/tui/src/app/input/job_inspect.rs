//! Job inspect screen input handler.
//!
//! Responsibilities:
//! - Handle Ctrl+C copy of the inspected job's SID
//!
//! Non-responsibilities:
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
        // Ctrl+C: copy SID of the currently selected job (inspect view)
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
            if let Some(job) = self.get_selected_job() {
                return Some(Action::CopyToClipboard(job.sid.clone()));
            }
            self.toasts.push(Toast::info("Nothing to copy"));
            return None;
        }

        None
    }
}
