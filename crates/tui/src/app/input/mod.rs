//! Per-screen input handlers for the TUI app.
//!
//! Responsibilities:
//! - Dispatch keyboard input to the appropriate screen handler
//! - Re-export all screen-specific input handlers
//!
//! Non-responsibilities:
//! - Does NOT define handler implementations (see submodules)
//! - Does NOT handle global keybindings (handled by keymap module)
//! - Does NOT handle popup input (handled by popups module)

pub mod apps;
pub mod audit;
pub mod cluster;
pub mod configs;
pub mod fired_alerts;
pub mod forwarders;
pub mod health;
pub mod helpers;
pub mod indexes;
pub mod inputs;
pub mod internal_logs;
pub mod job_inspect;
pub mod jobs;
pub mod kvstore;
pub mod license;
pub mod lookups;
pub mod macros;
pub mod multi_instance;
pub mod overview;
pub mod roles;
pub mod saved_searches;
pub mod search;
pub mod search_peers;
pub mod settings;
pub mod users;

use crate::action::Action;
use crate::app::App;
use crate::app::state::CurrentScreen;
use crossterm::event::KeyEvent;

impl App {
    /// Dispatch input to the appropriate screen handler.
    pub fn dispatch_screen_input(&mut self, key: KeyEvent) -> Option<Action> {
        match self.current_screen {
            CurrentScreen::Search => self.handle_search_input(key),
            CurrentScreen::Jobs => self.handle_jobs_input(key),
            CurrentScreen::Indexes => self.handle_indexes_input(key),
            CurrentScreen::Cluster => self.handle_cluster_input(key),
            CurrentScreen::JobInspect => self.handle_job_inspect_input(key),
            CurrentScreen::Health => self.handle_health_input(key),
            CurrentScreen::License => self.handle_license_input(key),
            CurrentScreen::Kvstore => self.handle_kvstore_input(key),
            CurrentScreen::SavedSearches => self.handle_saved_searches_input(key),
            CurrentScreen::Macros => self.handle_macros_input(key),
            CurrentScreen::InternalLogs => self.handle_internal_logs_input(key),
            CurrentScreen::Apps => self.handle_apps_input(key),
            CurrentScreen::Users => self.handle_users_input(key),
            CurrentScreen::Roles => self.handle_roles_input(key),
            CurrentScreen::SearchPeers => self.handle_search_peers_input(key),
            CurrentScreen::Inputs => self.handle_inputs_input(key),
            CurrentScreen::Configs => self.handle_configs_input(key),
            CurrentScreen::Settings => self.handle_settings_input(key),
            CurrentScreen::Overview => self.handle_overview_input(key),
            CurrentScreen::MultiInstance => self.handle_multi_instance_input(key),
            CurrentScreen::FiredAlerts => self.handle_fired_alerts_input(key),
            CurrentScreen::Forwarders => self.handle_forwarders_input(key),
            CurrentScreen::Lookups => self.handle_lookups_input(key),
            CurrentScreen::Audit => self.handle_audit_input(key),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::export::ExportTarget;
    use crate::ui::popup::PopupType;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn ctrl_key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn test_export_key_opens_export_popup_from_all_supported_screens() {
        let mut app = App::default();

        // Indexes
        app.popup = None;
        app.indexes = Some(vec![]); // Empty but present
        app.handle_indexes_input(ctrl_key('e'));
        assert_eq!(app.export_target, None); // Should NOT open for empty list

        app.indexes = Some(vec![
            serde_json::from_value(serde_json::json!({
                "name": "test",
                "currentDBSizeMB": 100,
                "totalEventCount": 1000
            }))
            .unwrap(),
        ]);
        app.handle_indexes_input(ctrl_key('e'));
        assert_eq!(app.export_target, Some(ExportTarget::Indexes));
        assert!(matches!(
            app.popup.as_ref().map(|p| &p.kind),
            Some(PopupType::ExportSearch)
        ));

        // Users
        app.popup = None;
        app.users = Some(vec![
            serde_json::from_value(serde_json::json!({
                "name": "test",
                "roles": ["admin"]
            }))
            .unwrap(),
        ]);
        app.handle_users_input(ctrl_key('e'));
        assert_eq!(app.export_target, Some(ExportTarget::Users));

        // Apps
        app.popup = None;
        app.apps = Some(vec![
            serde_json::from_value(serde_json::json!({
                "name": "test",
                "disabled": false
            }))
            .unwrap(),
        ]);
        app.handle_apps_input(ctrl_key('e'));
        assert_eq!(app.export_target, Some(ExportTarget::Apps));

        // Saved Searches
        app.popup = None;
        app.saved_searches = Some(vec![
            serde_json::from_value(serde_json::json!({
                "name": "test",
                "search": "index=_internal",
                "disabled": false
            }))
            .unwrap(),
        ]);
        app.handle_saved_searches_input(ctrl_key('e'));
        assert_eq!(app.export_target, Some(ExportTarget::SavedSearches));

        // Cluster
        app.popup = None;
        app.cluster_info = Some(
            serde_json::from_value(serde_json::json!({
                "id": "123",
                "mode": "master"
            }))
            .unwrap(),
        );
        app.handle_cluster_input(ctrl_key('e'));
        assert_eq!(app.export_target, Some(ExportTarget::ClusterInfo));

        // Jobs
        app.popup = None;
        app.jobs = Some(vec![
            serde_json::from_value(serde_json::json!({
                "sid": "123",
                "isDone": true,
                "runDuration": 0.0,
                "scanCount": 0,
                "eventCount": 0,
                "resultCount": 0,
                "diskUsage": 0
            }))
            .unwrap(),
        ]);
        app.handle_jobs_input(ctrl_key('e'));
        assert_eq!(app.export_target, Some(ExportTarget::Jobs));

        // Health
        app.popup = None;
        app.health_info = Some(serde_json::from_value(serde_json::json!({})).unwrap());
        app.handle_health_input(ctrl_key('e'));
        assert_eq!(app.export_target, Some(ExportTarget::Health));

        // Internal Logs
        app.popup = None;
        app.internal_logs = Some(vec![
            serde_json::from_value(serde_json::json!({
                "_time": "2025-01-01T00:00:00Z",
                "log_level": "INFO",
                "_raw": "test message"
            }))
            .unwrap(),
        ]);
        app.handle_internal_logs_input(ctrl_key('e'));
        assert_eq!(app.export_target, Some(ExportTarget::InternalLogs));
    }
}
