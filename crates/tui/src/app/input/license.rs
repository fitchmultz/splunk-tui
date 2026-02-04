//! License screen input handler.
//!
//! Responsibilities:
//! - Handle Ctrl+C or 'y' copy of license info (vim-style)
//! - Handle Ctrl+E export of license info
//! - Handle 'r' key to refresh license data
//!
//! Non-responsibilities:
//! - Does NOT handle global navigation (handled by keymap)
//! - Does NOT render the UI (handled by render module)
//! - Does NOT fetch license data (handled by actions)

use crate::action::Action;
use crate::app::App;
use crate::app::export::ExportTarget;
use crate::ui::Toast;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl App {
    /// Handle input for the license screen.
    pub fn handle_license_input(&mut self, key: KeyEvent) -> Option<Action> {
        // Ctrl+C or 'y': copy license info summary (vim-style)
        let is_copy = (key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('c')))
            || (key.modifiers.is_empty() && matches!(key.code, KeyCode::Char('y')));
        if is_copy {
            let content = self.license_info.as_ref().map(|info| {
                // Create a summary of license usage
                let mut parts = Vec::new();
                for usage in &info.usage {
                    let used = usage.effective_used_bytes();
                    let pct = if usage.quota > 0 {
                        (used as f64 / usage.quota as f64) * 100.0
                    } else {
                        0.0
                    };
                    parts.push(format!(
                        "{}: {:.1}% ({} / {})",
                        usage.name,
                        pct,
                        format_bytes(used),
                        format_bytes(usage.quota)
                    ));
                }
                if parts.is_empty() {
                    "No license usage data".to_string()
                } else {
                    parts.join(", ")
                }
            });

            if let Some(content) = content {
                return Some(Action::CopyToClipboard(content));
            }
            self.toasts.push(Toast::info("Nothing to copy"));
            return None;
        }

        match key.code {
            KeyCode::Char('e')
                if key.modifiers.contains(KeyModifiers::CONTROL) && self.license_info.is_some() =>
            {
                self.begin_export(ExportTarget::License);
                None
            }
            _ => None,
        }
    }
}

/// Format byte count with appropriate units.
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
