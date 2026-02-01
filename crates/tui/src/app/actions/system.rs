//! System action handlers for the TUI app.
//!
//! Responsibilities:
//! - Handle loading/progress state updates
//! - Handle notifications and toast messages
//! - Handle clipboard operations
//! - Handle terminal resize events
//! - Handle search/filter mode transitions
//! - Handle theme cycling
//! - Handle error detail popups
//! - Handle SPL validation results

use crate::action::Action;
use crate::app::App;
use crate::app::clipboard;
use crate::ui::Toast;

impl App {
    /// Handle system/miscellaneous actions.
    pub fn handle_system_action(&mut self, action: Action) {
        match action {
            Action::Loading(is_loading) => {
                self.loading = is_loading;
                if is_loading {
                    self.progress = 0.0;
                }
            }
            Action::Progress(p) => {
                self.progress = p;
            }
            Action::Notify(level, message) => {
                self.toasts.push(Toast::new(message, level));
            }
            Action::Tick => {
                // Prune expired toasts
                self.toasts.retain(|t| !t.is_expired());
            }
            Action::CopyToClipboard(content) => {
                self.handle_copy_to_clipboard(content);
            }
            Action::Resize(width, height) => {
                // Update last_area to reflect new terminal dimensions
                self.last_area = ratatui::layout::Rect::new(0, 0, width, height);
            }
            Action::EnterSearchMode => {
                self.enter_search_mode();
            }
            Action::SearchInput(c) => {
                self.filter_input.push(c);
            }
            Action::ClearSearch => {
                self.search_filter = None;
                self.rebuild_filtered_indices();
            }
            Action::CycleSortColumn => {
                self.sort_state.cycle();
                self.rebuild_filtered_indices();
            }
            Action::ToggleSortDirection => {
                self.sort_state.toggle_direction();
                self.rebuild_filtered_indices();
            }
            Action::CycleTheme => {
                self.cycle_theme();
            }
            Action::SplValidationResult {
                valid,
                errors,
                warnings,
            } => {
                self.handle_spl_validation_result(valid, errors, warnings);
            }
            Action::ShowErrorDetails(details) => {
                self.show_error_details(details);
            }
            Action::ShowErrorDetailsFromCurrent => {
                self.show_error_details_from_current();
            }
            Action::ClearErrorDetails => {
                self.current_error = None;
                self.popup = None;
            }
            Action::JobOperationComplete(msg) => {
                self.selected_jobs.clear();
                self.search_status = msg;
                self.loading = false;
            }
            Action::OpenCreateIndexDialog => {
                self.open_create_index_dialog();
            }
            // Cluster management result actions
            Action::MaintenanceModeSet { result } => {
                self.loading = false;
                match result {
                    Ok(()) => {
                        self.toasts.push(Toast::info("Maintenance mode updated"));
                        // Refresh cluster info to show updated state
                        self.trigger_load_cluster_info();
                    }
                    Err(e) => {
                        self.toasts.push(Toast::error(format!(
                            "Failed to set maintenance mode: {}",
                            e
                        )));
                    }
                }
            }
            Action::ClusterRebalanced { result } => {
                self.loading = false;
                match result {
                    Ok(()) => {
                        self.toasts.push(Toast::info("Cluster rebalance initiated"));
                    }
                    Err(e) => {
                        self.toasts
                            .push(Toast::error(format!("Failed to rebalance cluster: {}", e)));
                    }
                }
            }
            Action::PeerDecommissioned { result } => {
                self.loading = false;
                match result {
                    Ok(()) => {
                        self.toasts.push(Toast::info("Peer decommission initiated"));
                        // Refresh peers list
                        self.trigger_load_cluster_peers();
                    }
                    Err(e) => {
                        self.toasts
                            .push(Toast::error(format!("Failed to decommission peer: {}", e)));
                    }
                }
            }
            Action::PeerRemoved { result } => {
                self.loading = false;
                match result {
                    Ok(()) => {
                        self.toasts.push(Toast::info("Peer removed from cluster"));
                        // Refresh peers list
                        self.trigger_load_cluster_peers();
                    }
                    Err(e) => {
                        self.toasts
                            .push(Toast::error(format!("Failed to remove peer: {}", e)));
                    }
                }
            }
            _ => {}
        }
    }

    fn trigger_load_cluster_info(&mut self) {
        // This is handled by the main loop, just set a flag or send action
        // For now, we'll refresh on next tick or user action
    }

    fn trigger_load_cluster_peers(&mut self) {
        // This is handled by the main loop, just set a flag or send action
        // For now, we'll refresh on next tick or user action
    }

    fn handle_copy_to_clipboard(&mut self, content: String) {
        match clipboard::copy_to_clipboard(content.clone()) {
            Ok(()) => {
                let preview = Self::clipboard_preview(&content);
                self.toasts.push(Toast::info(format!("Copied: {preview}")));
            }
            Err(e) => {
                self.toasts
                    .push(Toast::error(format!("Clipboard error: {e}")));
            }
        }
    }

    fn enter_search_mode(&mut self) {
        self.is_filtering = true;
        // Save current filter for potential cancel
        self.filter_before_edit = self.search_filter.clone();
        // Pre-populate filter_input with existing filter for editing
        self.filter_input = self.search_filter.clone().unwrap_or_default();
    }

    fn cycle_theme(&mut self) {
        self.color_theme = self.color_theme.cycle_next();
        self.theme = splunk_config::Theme::from(self.color_theme);
        self.toasts
            .push(Toast::info(format!("Theme: {}", self.color_theme)));
    }

    fn handle_spl_validation_result(
        &mut self,
        valid: bool,
        errors: Vec<String>,
        warnings: Vec<String>,
    ) {
        self.spl_validation_state = crate::app::SplValidationState {
            valid: Some(valid),
            errors,
            warnings,
            last_validated: Some(std::time::Instant::now()),
        };
        self.spl_validation_pending = false;
    }

    fn show_error_details(&mut self, details: crate::error_details::ErrorDetails) {
        use crate::ui::popup::{Popup, PopupType};
        self.current_error = Some(details);
        self.popup = Some(Popup::builder(PopupType::ErrorDetails).build());
    }

    fn show_error_details_from_current(&mut self) {
        use crate::ui::popup::{Popup, PopupType};
        if self.current_error.is_some() {
            self.popup = Some(Popup::builder(PopupType::ErrorDetails).build());
        }
    }

    fn open_create_index_dialog(&mut self) {
        use crate::ui::popup::{Popup, PopupType};
        self.popup = Some(
            Popup::builder(PopupType::CreateIndex {
                name_input: String::new(),
                max_data_size_mb: None,
                max_hot_buckets: None,
                max_warm_db_count: None,
                frozen_time_period_secs: None,
                home_path: None,
                cold_db_path: None,
                thawed_path: None,
                cold_to_frozen_dir: None,
            })
            .build(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConnectionContext;

    #[test]
    fn test_loading_sets_progress_to_zero() {
        let mut app = App::new(None, ConnectionContext::default());
        app.progress = 0.5;

        app.handle_system_action(Action::Loading(true));

        assert!(app.loading);
        assert_eq!(app.progress, 0.0);
    }

    #[test]
    fn test_progress_updates_value() {
        let mut app = App::new(None, ConnectionContext::default());

        app.handle_system_action(Action::Progress(0.75));

        assert_eq!(app.progress, 0.75);
    }

    #[test]
    fn test_notify_adds_toast() {
        let mut app = App::new(None, ConnectionContext::default());

        app.handle_system_action(Action::Notify(
            crate::ui::ToastLevel::Info,
            "Test message".to_string(),
        ));

        assert_eq!(app.toasts.len(), 1);
        assert_eq!(app.toasts[0].message, "Test message");
    }

    #[test]
    fn test_tick_prunes_expired_toasts() {
        let mut app = App::new(None, ConnectionContext::default());
        // Add a toast that's already expired (using created_at field)
        let mut expired_toast = Toast::info("Expired");
        expired_toast.created_at = std::time::Instant::now() - std::time::Duration::from_secs(100);
        app.toasts.push(expired_toast);

        // Add a fresh toast
        app.toasts.push(Toast::info("Fresh"));

        app.handle_system_action(Action::Tick);

        // Only the fresh toast should remain
        assert_eq!(app.toasts.len(), 1);
        assert_eq!(app.toasts[0].message, "Fresh");
    }

    #[test]
    fn test_enter_search_mode_saves_current_filter() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_filter = Some("existing filter".to_string());

        app.handle_system_action(Action::EnterSearchMode);

        assert!(app.is_filtering);
        assert_eq!(app.filter_before_edit, Some("existing filter".to_string()));
        assert_eq!(app.filter_input, "existing filter");
    }

    #[test]
    fn test_enter_search_mode_with_no_filter() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_filter = None;

        app.handle_system_action(Action::EnterSearchMode);

        assert!(app.is_filtering);
        assert!(app.filter_before_edit.is_none());
        assert!(app.filter_input.is_empty());
    }

    #[test]
    fn test_search_input_appends_character() {
        let mut app = App::new(None, ConnectionContext::default());
        app.filter_input = "hel".to_string();

        app.handle_system_action(Action::SearchInput('l'));
        app.handle_system_action(Action::SearchInput('o'));

        assert_eq!(app.filter_input, "hello");
    }

    #[test]
    fn test_clear_search_clears_filter() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_filter = Some("test".to_string());

        app.handle_system_action(Action::ClearSearch);

        assert!(app.search_filter.is_none());
    }

    #[test]
    fn test_cycle_theme_changes_theme() {
        let mut app = App::new(None, ConnectionContext::default());
        let initial_theme = app.color_theme;

        app.handle_system_action(Action::CycleTheme);

        assert_ne!(app.color_theme, initial_theme);
        assert_eq!(app.toasts.len(), 1);
        assert!(app.toasts[0].message.contains("Theme:"));
    }

    #[test]
    fn test_spl_validation_result_updates_state() {
        let mut app = App::new(None, ConnectionContext::default());

        app.handle_system_action(Action::SplValidationResult {
            valid: true,
            errors: vec![],
            warnings: vec!["Warning 1".to_string()],
        });

        assert_eq!(app.spl_validation_state.valid, Some(true));
        assert!(
            app.spl_validation_state
                .warnings
                .contains(&"Warning 1".to_string())
        );
        assert!(!app.spl_validation_pending);
    }

    #[test]
    fn test_show_error_details_from_current_with_no_error() {
        let mut app = App::new(None, ConnectionContext::default());
        app.current_error = None;

        app.handle_system_action(Action::ShowErrorDetailsFromCurrent);

        assert!(app.popup.is_none());
    }

    #[test]
    fn test_clear_error_details_clears_state() {
        let mut app = App::new(None, ConnectionContext::default());
        use crate::ui::popup::{Popup, PopupType};
        app.current_error = Some(crate::error_details::ErrorDetails::from_error_string(
            "Error",
        ));
        app.popup = Some(Popup::builder(PopupType::ErrorDetails).build());

        app.handle_system_action(Action::ClearErrorDetails);

        assert!(app.current_error.is_none());
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_job_operation_complete_clears_selection() {
        let mut app = App::new(None, ConnectionContext::default());
        app.selected_jobs.insert("job1".to_string());
        app.selected_jobs.insert("job2".to_string());

        app.handle_system_action(Action::JobOperationComplete("Jobs finalized".to_string()));

        assert!(app.selected_jobs.is_empty());
        assert_eq!(app.search_status, "Jobs finalized");
        assert!(!app.loading);
    }

    #[test]
    fn test_resize_updates_last_area() {
        let mut app = App::new(None, ConnectionContext::default());

        app.handle_system_action(Action::Resize(100, 50));

        assert_eq!(app.last_area.width, 100);
        assert_eq!(app.last_area.height, 50);
    }
}
