//! Multi-instance dashboard input handler.

use crate::action::Action;
use crate::app::App;
use crate::app::export::ExportTarget;
use crate::ui::Toast;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl App {
    /// Handle input for the multi-instance dashboard screen.
    pub fn handle_multi_instance_input(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            // Navigation: Up/Down to select instance
            KeyCode::Up | KeyCode::Char('k') => {
                if self.multi_instance_selected_index > 0 {
                    self.multi_instance_selected_index -= 1;
                }
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(ref data) = self.multi_instance_data
                    && self.multi_instance_selected_index < data.instances.len().saturating_sub(1)
                {
                    self.multi_instance_selected_index += 1;
                }
                None
            }
            // Refresh data: 'R' (global) or 'r' (per-instance)
            KeyCode::Char('R') => Some(Action::LoadMultiInstanceOverview),
            KeyCode::Char('r') => {
                let profile_name = self.multi_instance_data.as_ref().and_then(|data| {
                    data.instances
                        .get(self.multi_instance_selected_index)
                        .map(|i| i.profile_name.clone())
                });

                if let Some(name) = profile_name {
                    // Mark as loading in UI immediately
                    if let Some(ref mut data) = self.multi_instance_data {
                        if let Some(instance) =
                            data.instances.iter_mut().find(|i| i.profile_name == name)
                        {
                            instance.status = crate::action::InstanceStatus::Loading;
                        }
                    }

                    return Some(Action::RetryInstance(name));
                }
                Some(Action::LoadMultiInstanceOverview)
            }

            // Ctrl+C or 'y': copy instance summary (vim-style)
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let content = self.multi_instance_data.as_ref().map(|d| {
                    d.instances
                        .iter()
                        .map(|i| {
                            format!(
                                "{} ({}): Health={}, Jobs={}",
                                i.profile_name, i.base_url, i.health_status, i.job_count
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                });

                if let Some(content) = content {
                    return Some(Action::CopyToClipboard(content));
                }
                self.toasts.push(Toast::info("Nothing to copy"));
                None
            }
            KeyCode::Char('y') if key.modifiers.is_empty() => {
                let content = self.multi_instance_data.as_ref().map(|d| {
                    d.instances
                        .iter()
                        .map(|i| {
                            format!(
                                "{} ({}): Health={}, Jobs={}",
                                i.profile_name, i.base_url, i.health_status, i.job_count
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                });

                if let Some(content) = content {
                    return Some(Action::CopyToClipboard(content));
                }
                self.toasts.push(Toast::info("Nothing to copy"));
                None
            }
            // Ctrl+E: export data
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if self.multi_instance_data.is_some() {
                    self.begin_export(ExportTarget::MultiInstance);
                } else {
                    self.toasts.push(Toast::info("No data to export"));
                }
                None
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{InstanceOverview, MultiInstanceOverviewData, OverviewResource};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn create_test_multi_instance_data() -> MultiInstanceOverviewData {
        use crate::action::InstanceStatus;
        MultiInstanceOverviewData {
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            instances: vec![
                InstanceOverview {
                    profile_name: "prod".to_string(),
                    base_url: "https://splunk.prod.example.com".to_string(),
                    resources: vec![OverviewResource {
                        resource_type: "indexes".to_string(),
                        count: 42,
                        status: "ok".to_string(),
                        error: None,
                    }],
                    error: None,
                    health_status: "green".to_string(),
                    job_count: 5,
                    status: InstanceStatus::Healthy,
                    last_success_at: Some("2024-01-01T00:00:00Z".to_string()),
                },
                InstanceOverview {
                    profile_name: "dev".to_string(),
                    base_url: "https://splunk.dev.example.com".to_string(),
                    resources: vec![OverviewResource {
                        resource_type: "indexes".to_string(),
                        count: 10,
                        status: "ok".to_string(),
                        error: None,
                    }],
                    error: None,
                    health_status: "green".to_string(),
                    job_count: 1,
                    status: InstanceStatus::Healthy,
                    last_success_at: Some("2024-01-01T00:00:00Z".to_string()),
                },
            ],
        }
    }

    #[test]
    fn test_down_navigation() {
        let mut app = crate::app::App {
            multi_instance_data: Some(create_test_multi_instance_data()),
            multi_instance_selected_index: 0,
            ..Default::default()
        };

        let key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        app.handle_multi_instance_input(key);
        assert_eq!(app.multi_instance_selected_index, 1);

        // Should not go past last item
        app.handle_multi_instance_input(key);
        assert_eq!(app.multi_instance_selected_index, 1);
    }

    #[test]
    fn test_up_navigation() {
        let mut app = crate::app::App {
            multi_instance_data: Some(create_test_multi_instance_data()),
            multi_instance_selected_index: 1,
            ..Default::default()
        };

        let key = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        app.handle_multi_instance_input(key);
        assert_eq!(app.multi_instance_selected_index, 0);

        // Should not go below 0
        app.handle_multi_instance_input(key);
        assert_eq!(app.multi_instance_selected_index, 0);
    }

    #[test]
    fn test_j_k_navigation() {
        let mut app = crate::app::App {
            multi_instance_data: Some(create_test_multi_instance_data()),
            multi_instance_selected_index: 0,
            ..Default::default()
        };

        // Test 'j' for down
        let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        app.handle_multi_instance_input(key);
        assert_eq!(app.multi_instance_selected_index, 1);

        // Test 'k' for up
        let key = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        app.handle_multi_instance_input(key);
        assert_eq!(app.multi_instance_selected_index, 0);
    }

    #[test]
    fn test_refresh() {
        let mut app = crate::app::App::default();

        let key = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE);
        let action = app.handle_multi_instance_input(key);

        assert!(matches!(action, Some(Action::LoadMultiInstanceOverview)));
    }

    #[test]
    fn test_ctrl_c_copies_summary() {
        let mut app = crate::app::App {
            multi_instance_data: Some(create_test_multi_instance_data()),
            ..Default::default()
        };

        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let action = app.handle_multi_instance_input(key);

        assert!(matches!(action, Some(Action::CopyToClipboard(_))));
    }

    #[test]
    fn test_ctrl_c_shows_toast_when_empty() {
        let mut app = crate::app::App {
            multi_instance_data: None,
            ..Default::default()
        };

        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let action = app.handle_multi_instance_input(key);

        assert!(action.is_none());
        assert!(!app.toasts.is_empty());
    }

    #[test]
    fn test_ctrl_e_opens_export() {
        let mut app = crate::app::App {
            multi_instance_data: Some(create_test_multi_instance_data()),
            ..Default::default()
        };

        let key = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL);
        app.handle_multi_instance_input(key);

        assert_eq!(app.export_target, Some(ExportTarget::MultiInstance));
    }
}
