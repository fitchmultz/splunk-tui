//! KVStore screen input handler.
//!
//! Responsibilities:
//! - Handle Ctrl+C or 'y' copy of KVStore status (vim-style)
//! - Handle Ctrl+E export of KVStore status
//! - Handle 'r' key to refresh KVStore data
//!
//! Does NOT handle:
//! - Does NOT handle global navigation (handled by keymap)
//! - Does NOT render the UI (handled by render module)
//! - Does NOT fetch KVStore data (handled by actions)

use crate::action::Action;
use crate::app::App;
use crate::app::export::ExportTarget;
use crate::ui::Toast;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl App {
    /// Handle input for the KVStore screen.
    pub fn handle_kvstore_input(&mut self, key: KeyEvent) -> Option<Action> {
        // Ctrl+C or 'y': copy KVStore status summary (vim-style)
        let is_copy = (key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('c')))
            || (key.modifiers.is_empty() && matches!(key.code, KeyCode::Char('y')));
        if is_copy {
            let content = self.kvstore_status.as_ref().map(|status| {
                // Create a summary of KVStore status
                let member = &status.current_member;
                let replication = &status.replication_status;

                let usage_pct = if replication.oplog_size > 0 {
                    (replication.oplog_used / replication.oplog_size as f64) * 100.0
                } else {
                    0.0
                };

                format!(
                    "KVStore: {}@{}:{} ({}), Status: {}, Oplog: {:.1}% used",
                    member.guid,
                    member.host,
                    member.port,
                    member.replica_set,
                    member.status,
                    usage_pct
                )
            });

            if let Some(content) = content {
                return Some(Action::CopyToClipboard(content));
            }
            self.toasts.push(Toast::info("Nothing to copy"));
            return None;
        }

        match key.code {
            KeyCode::Char('e')
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && self.kvstore_status.is_some() =>
            {
                self.begin_export(ExportTarget::Kvstore);
                None
            }
            _ => None,
        }
    }
}
