//! Per-screen input handlers for the TUI app.
//!
//! Responsibilities:
//! - Handle keyboard input for each screen
//! - Return Actions to be processed by the main loop
//!
//! Non-responsibilities:
//! - Does NOT mutate App state directly (returns Actions)
//! - Does NOT handle global keybindings (handled by keymap module)

use crate::action::Action;
use crate::app::App;
use crate::app::export::ExportTarget;
use crate::app::state::CurrentScreen;
use crate::ui::Toast;
use crate::ui::popup::{Popup, PopupType};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl App {
    fn handle_search_input(&mut self, key: KeyEvent) -> Option<Action> {
        // Handle Ctrl+* shortcuts while in input
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
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
                self.toasts.push(Toast::info("Nothing to copy"));
                return None;
            }

            return Some(Action::CopyToClipboard(content));
        }

        match key.code {
            KeyCode::Enter => {
                if !self.search_input.is_empty() {
                    let query = self.search_input.clone();
                    self.add_to_history(query.clone());
                    self.search_status = format!("Running: {}", query);
                    Some(Action::RunSearch(query))
                } else {
                    None
                }
            }
            KeyCode::Backspace => {
                self.history_index = None;
                self.search_input.pop();
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
                self.search_input.push(c);
                None
            }
            _ => None,
        }
    }

    fn handle_jobs_input(&mut self, key: KeyEvent) -> Option<Action> {
        // Ctrl+C: copy selected job SID
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
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

    pub(crate) fn handle_jobs_filter_input(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Esc => {
                self.is_filtering = false;
                self.filter_input.clear();
                Some(Action::ClearSearch)
            }
            KeyCode::Enter => {
                self.is_filtering = false;
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

    fn handle_indexes_input(&mut self, key: KeyEvent) -> Option<Action> {
        // Ctrl+C: copy selected index name
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
            let content = self
                .indexes
                .as_ref()
                .and_then(|indexes| self.indexes_state.selected().and_then(|i| indexes.get(i)))
                .map(|idx| idx.name.clone());

            if let Some(content) = content {
                return Some(Action::CopyToClipboard(content));
            }

            self.toasts.push(Toast::info("Nothing to copy"));
            return None;
        }

        match key.code {
            KeyCode::Char('e')
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && self
                        .indexes
                        .as_ref()
                        .map(|v| !v.is_empty())
                        .unwrap_or(false) =>
            {
                self.begin_export(ExportTarget::Indexes);
                None
            }
            _ => None,
        }
    }

    fn handle_cluster_input(&mut self, key: KeyEvent) -> Option<Action> {
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

    fn handle_job_inspect_input(&mut self, key: KeyEvent) -> Option<Action> {
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

    fn handle_health_input(&mut self, key: KeyEvent) -> Option<Action> {
        // Ctrl+C: copy health status
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
            let content = self.health_info.as_ref().and_then(|h| {
                h.splunkd_health
                    .as_ref()
                    .map(|sh| sh.health.clone())
                    .or_else(|| h.server_info.as_ref().map(|s| s.server_name.clone()))
            });

            if let Some(content) = content {
                return Some(Action::CopyToClipboard(content));
            }
            self.toasts.push(Toast::info("Nothing to copy"));
            return None;
        }

        match key.code {
            KeyCode::Char('e')
                if key.modifiers.contains(KeyModifiers::CONTROL) && self.health_info.is_some() =>
            {
                self.begin_export(ExportTarget::Health);
                None
            }
            _ => None,
        }
    }

    fn handle_saved_searches_input(&mut self, key: KeyEvent) -> Option<Action> {
        // Ctrl+C: copy selected saved search name
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
            let content = self.saved_searches.as_ref().and_then(|searches| {
                self.saved_searches_state
                    .selected()
                    .and_then(|i| searches.get(i))
                    .map(|s| s.name.clone())
            });

            if let Some(content) = content.filter(|s| !s.trim().is_empty()) {
                return Some(Action::CopyToClipboard(content));
            }

            self.toasts.push(Toast::info("Nothing to copy"));
            return None;
        }

        match key.code {
            KeyCode::Char('e')
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && self
                        .saved_searches
                        .as_ref()
                        .map(|v| !v.is_empty())
                        .unwrap_or(false) =>
            {
                self.begin_export(ExportTarget::SavedSearches);
                None
            }
            KeyCode::Enter => {
                let query = self.saved_searches.as_ref().and_then(|searches| {
                    self.saved_searches_state.selected().and_then(|selected| {
                        searches.get(selected).map(|search| search.search.clone())
                    })
                });

                if let Some(query) = query {
                    self.search_input = query.clone();
                    self.current_screen = CurrentScreen::Search;
                    self.add_to_history(query.clone());
                    self.search_status = format!("Running: {}", query);
                    return Some(Action::RunSearch(query));
                }
                None
            }
            _ => None,
        }
    }

    fn handle_internal_logs_input(&mut self, key: KeyEvent) -> Option<Action> {
        // Ctrl+C: copy selected log message
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
            let content = self.internal_logs.as_ref().and_then(|logs| {
                self.internal_logs_state
                    .selected()
                    .and_then(|i| logs.get(i))
                    .map(|l| l.message.clone())
            });

            if let Some(content) = content.filter(|s| !s.trim().is_empty()) {
                return Some(Action::CopyToClipboard(content));
            }

            self.toasts.push(Toast::info("Nothing to copy"));
            return None;
        }

        match key.code {
            KeyCode::Char('e')
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && self
                        .internal_logs
                        .as_ref()
                        .map(|v| !v.is_empty())
                        .unwrap_or(false) =>
            {
                self.begin_export(ExportTarget::InternalLogs);
                None
            }
            KeyCode::Char('a') => {
                self.auto_refresh = !self.auto_refresh;
                None
            }
            _ => None,
        }
    }

    fn handle_apps_input(&mut self, key: KeyEvent) -> Option<Action> {
        // Ctrl+C: copy selected app name
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
            let content = self.apps.as_ref().and_then(|apps| {
                self.apps_state
                    .selected()
                    .and_then(|i| apps.get(i))
                    .map(|a| a.name.clone())
            });

            if let Some(content) = content.filter(|s| !s.trim().is_empty()) {
                return Some(Action::CopyToClipboard(content));
            }

            self.toasts.push(Toast::info("Nothing to copy"));
            return None;
        }

        match key.code {
            KeyCode::Char('e')
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && self.apps.as_ref().map(|v| !v.is_empty()).unwrap_or(false) =>
            {
                self.begin_export(ExportTarget::Apps);
                None
            }
            _ => None,
        }
    }

    fn handle_users_input(&mut self, key: KeyEvent) -> Option<Action> {
        // Ctrl+C: copy selected username
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
            let content = self.users.as_ref().and_then(|users| {
                self.users_state
                    .selected()
                    .and_then(|i| users.get(i))
                    .map(|u| u.name.clone())
            });

            if let Some(content) = content.filter(|s| !s.trim().is_empty()) {
                return Some(Action::CopyToClipboard(content));
            }

            self.toasts.push(Toast::info("Nothing to copy"));
            return None;
        }

        match key.code {
            KeyCode::Char('e')
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && self.users.as_ref().map(|v| !v.is_empty()).unwrap_or(false) =>
            {
                self.begin_export(ExportTarget::Users);
                None
            }
            _ => None,
        }
    }

    fn handle_settings_input(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Char('a') => {
                self.auto_refresh = !self.auto_refresh;
                self.toasts.push(Toast::info(format!(
                    "Auto-refresh: {}",
                    if self.auto_refresh { "On" } else { "Off" }
                )));
                None
            }
            KeyCode::Char('s') => {
                self.sort_state.column = match self.sort_state.column {
                    crate::app::state::SortColumn::Sid => crate::app::state::SortColumn::Status,
                    crate::app::state::SortColumn::Status => {
                        crate::app::state::SortColumn::Duration
                    }
                    crate::app::state::SortColumn::Duration => {
                        crate::app::state::SortColumn::Results
                    }
                    crate::app::state::SortColumn::Results => crate::app::state::SortColumn::Events,
                    crate::app::state::SortColumn::Events => crate::app::state::SortColumn::Sid,
                };
                self.toasts.push(Toast::info(format!(
                    "Sort column: {}",
                    self.sort_state.column.as_str()
                )));
                None
            }
            KeyCode::Char('d') => {
                self.sort_state.direction = match self.sort_state.direction {
                    crate::app::state::SortDirection::Asc => crate::app::state::SortDirection::Desc,
                    crate::app::state::SortDirection::Desc => crate::app::state::SortDirection::Asc,
                };
                self.toasts.push(Toast::info(format!(
                    "Sort direction: {}",
                    self.sort_state.direction.as_str()
                )));
                None
            }
            KeyCode::Char('c') => {
                self.search_history.clear();
                self.toasts.push(Toast::info("Search history cleared"));
                None
            }
            _ => None,
        }
    }

    /// Dispatch input to the appropriate screen handler.
    pub fn dispatch_screen_input(&mut self, key: KeyEvent) -> Option<Action> {
        match self.current_screen {
            CurrentScreen::Search => self.handle_search_input(key),
            CurrentScreen::Jobs => self.handle_jobs_input(key),
            CurrentScreen::Indexes => self.handle_indexes_input(key),
            CurrentScreen::Cluster => self.handle_cluster_input(key),
            CurrentScreen::JobInspect => self.handle_job_inspect_input(key),
            CurrentScreen::Health => self.handle_health_input(key),
            CurrentScreen::SavedSearches => self.handle_saved_searches_input(key),
            CurrentScreen::InternalLogs => self.handle_internal_logs_input(key),
            CurrentScreen::Apps => self.handle_apps_input(key),
            CurrentScreen::Users => self.handle_users_input(key),
            CurrentScreen::Settings => self.handle_settings_input(key),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
