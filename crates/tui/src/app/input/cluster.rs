//! Cluster screen input handler.
//!
//! Responsibilities:
//! - Handle Ctrl+C copy of cluster ID
//! - Handle Ctrl+E export of cluster info
//!
//! Non-responsibilities:
//! - Does NOT handle global navigation (handled by keymap)
//! - Does NOT render the UI (handled by render module)
//! - Does NOT fetch cluster data (handled by actions)

use crate::action::Action;
use crate::app::App;
use crate::app::export::ExportTarget;
use crate::ui::Toast;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl App {
    /// Handle input for the cluster screen.
    pub fn handle_cluster_input(&mut self, key: KeyEvent) -> Option<Action> {
        // Ctrl+C: copy cluster ID
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
            if let Some(info) = &self.cluster_info {
                return Some(Action::CopyToClipboard(info.id.clone()));
            }
            self.toasts.push(Toast::info("Nothing to copy"));
            return None;
        }

        match key.code {
            KeyCode::Char('e')
                if key.modifiers.contains(KeyModifiers::CONTROL) && self.cluster_info.is_some() =>
            {
                self.begin_export(ExportTarget::ClusterInfo);
                None
            }
            _ => None,
        }
    }
}
