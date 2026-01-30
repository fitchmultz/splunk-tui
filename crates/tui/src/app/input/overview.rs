//! Overview screen input handler.

use crate::action::Action;
use crate::app::App;
use crate::app::export::ExportTarget;
use crate::ui::Toast;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl App {
    /// Handle input for the overview screen.
    pub fn handle_overview_input(&mut self, key: KeyEvent) -> Option<Action> {
        // Ctrl+C: copy resource summary
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
            let content = self.overview_data.as_ref().map(|d| {
                d.resources
                    .iter()
                    .map(|r| format!("{}: {} ({})", r.resource_type, r.count, r.status))
                    .collect::<Vec<_>>()
                    .join("\n")
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
                    && self.overview_data.is_some() =>
            {
                self.begin_export(ExportTarget::Overview);
                None
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{OverviewData, OverviewResource};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn ctrl_key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    #[test]
    fn test_ctrl_c_copies_summary() {
        let mut app = crate::app::App {
            overview_data: Some(OverviewData {
                resources: vec![OverviewResource {
                    resource_type: "indexes".to_string(),
                    count: 5,
                    status: "ok".to_string(),
                    error: None,
                }],
            }),
            ..Default::default()
        };

        let action = app.handle_overview_input(ctrl_key('c'));
        assert!(matches!(action, Some(Action::CopyToClipboard(_))));
    }

    #[test]
    fn test_ctrl_c_shows_toast_when_empty() {
        let mut app = crate::app::App {
            overview_data: None,
            ..Default::default()
        };

        let action = app.handle_overview_input(ctrl_key('c'));
        assert!(action.is_none());
        assert!(!app.toasts.is_empty());
    }

    #[test]
    fn test_ctrl_e_opens_export() {
        let mut app = crate::app::App {
            overview_data: Some(OverviewData { resources: vec![] }),
            ..Default::default()
        };

        app.handle_overview_input(ctrl_key('e'));
        assert_eq!(app.export_target, Some(ExportTarget::Overview));
    }
}
