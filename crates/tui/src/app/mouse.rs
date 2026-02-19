//! Mouse event handling for the TUI app.
//!
//! Responsibilities:
//! - Handle mouse scroll events
//! - Handle footer button clicks
//! - Handle content area clicks for selection
//! - Handle popup dialog interactions (confirm/cancel via click)
//!
//! Does NOT handle:
//! - Does NOT handle keyboard input
//! - Does NOT render the UI

use crate::action::Action;
use crate::app::App;
use crate::app::footer_layout::FooterLayout;
use crate::app::state::{CurrentScreen, HEADER_HEIGHT};
use crate::ui::popup::{POPUP_HEIGHT_PERCENT, POPUP_WIDTH_PERCENT};
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Constraint, Direction, Layout, Rect};

impl App {
    /// Handle mouse input - returns Action if one should be dispatched.
    pub fn handle_mouse(&mut self, mouse: MouseEvent) -> Option<Action> {
        // Route to popup handler when popup is active
        if self.popup.is_some() {
            return self.handle_popup_mouse(mouse);
        }
        match mouse.kind {
            MouseEventKind::ScrollUp => Some(Action::NavigateUp),
            MouseEventKind::ScrollDown => Some(Action::NavigateDown),
            MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                // Check for footer navigation
                if mouse.row >= self.last_area.height.saturating_sub(2)
                    && mouse.row < self.last_area.height.saturating_sub(1)
                {
                    return self.handle_footer_click(mouse.column);
                }

                // Check for content area clicks
                if mouse.row >= HEADER_HEIGHT
                    && mouse.row < self.last_area.height - crate::app::state::FOOTER_HEIGHT
                {
                    return self.handle_content_click(mouse.row, mouse.column);
                }
                None
            }
            _ => None,
        }
    }

    /// Handle clicks in the footer area.
    /// Currently only handles quit button clicks since navigation is now keyboard-only (Tab/Shift+Tab).
    fn handle_footer_click(&mut self, col: u16) -> Option<Action> {
        // Use the shared layout helper to ensure consistency with rendering
        // Pass search_input_mode for context-aware navigation width in Search screen
        let layout = FooterLayout::calculate_with_mode(
            self.loading,
            f64::from(self.progress),
            self.current_screen,
            self.last_area.width,
            Some(self.search_input_mode),
        );

        if layout.is_quit_clicked(col) {
            Some(Action::Quit)
        } else {
            None
        }
    }

    /// Handle clicks in the main content area.
    fn handle_content_click(&mut self, row: u16, _col: u16) -> Option<Action> {
        // Ignore content clicks during loading
        if self.loading {
            return None;
        }
        match self.current_screen {
            // PlainList screens (ListState, no header)
            CurrentScreen::Apps => {
                if let Some(index) = calculate_list_click_index(
                    row,
                    HEADER_HEIGHT + 1,
                    self.apps_state.offset(),
                    self.apps.as_deref().map(|v| v.len()).unwrap_or(0),
                ) {
                    self.apps_state.select(Some(index));
                }
                None
            }
            CurrentScreen::Users => {
                if let Some(index) = calculate_list_click_index(
                    row,
                    HEADER_HEIGHT + 1,
                    self.users_state.offset(),
                    self.users.as_deref().map(|v| v.len()).unwrap_or(0),
                ) {
                    self.users_state.select(Some(index));
                }
                None
            }
            CurrentScreen::Roles => {
                if let Some(index) = calculate_list_click_index(
                    row,
                    HEADER_HEIGHT + 1,
                    self.roles_state.offset(),
                    self.roles.as_deref().map(|v| v.len()).unwrap_or(0),
                ) {
                    self.roles_state.select(Some(index));
                }
                None
            }
            CurrentScreen::Indexes => {
                if let Some(index) = calculate_list_click_index(
                    row,
                    HEADER_HEIGHT + 1,
                    self.indexes_state.offset(),
                    self.indexes.as_deref().map(|v| v.len()).unwrap_or(0),
                ) {
                    self.indexes_state.select(Some(index));
                }
                None
            }
            CurrentScreen::SavedSearches => {
                if let Some(index) = calculate_list_click_index(
                    row,
                    HEADER_HEIGHT + 1,
                    self.saved_searches_state.offset(),
                    self.saved_searches.as_deref().map(|v| v.len()).unwrap_or(0),
                ) {
                    self.saved_searches_state.select(Some(index));
                }
                None
            }
            CurrentScreen::Macros => {
                if let Some(index) = calculate_list_click_index(
                    row,
                    HEADER_HEIGHT + 1,
                    self.macros_state.offset(),
                    self.macros.as_deref().map(|v| v.len()).unwrap_or(0),
                ) {
                    self.macros_state.select(Some(index));
                }
                None
            }
            CurrentScreen::FiredAlerts => {
                if let Some(index) = calculate_list_click_index(
                    row,
                    HEADER_HEIGHT + 1,
                    self.fired_alerts_state.offset(),
                    self.fired_alerts.as_deref().map(|v| v.len()).unwrap_or(0),
                ) {
                    self.fired_alerts_state.select(Some(index));
                }
                None
            }
            CurrentScreen::Dashboards => {
                if let Some(index) = calculate_list_click_index(
                    row,
                    HEADER_HEIGHT + 1,
                    self.dashboards_state.offset(),
                    self.dashboards.as_deref().map(|v| v.len()).unwrap_or(0),
                ) {
                    self.dashboards_state.select(Some(index));
                }
                None
            }
            CurrentScreen::DataModels => {
                if let Some(index) = calculate_list_click_index(
                    row,
                    HEADER_HEIGHT + 1,
                    self.data_models_state.offset(),
                    self.data_models.as_deref().map(|v| v.len()).unwrap_or(0),
                ) {
                    self.data_models_state.select(Some(index));
                }
                None
            }

            // TableWithHeader screens (TableState, has header row)
            CurrentScreen::Jobs => self.handle_jobs_click(row),
            CurrentScreen::Inputs => {
                if let Some(index) = calculate_table_click_index(
                    row,
                    HEADER_HEIGHT + 1,
                    self.inputs_state.offset(),
                    self.inputs.as_deref().map(|v| v.len()).unwrap_or(0),
                ) {
                    self.inputs_state.select(Some(index));
                }
                None
            }
            CurrentScreen::SearchPeers => {
                if let Some(index) = calculate_table_click_index(
                    row,
                    HEADER_HEIGHT + 1,
                    self.search_peers_state.offset(),
                    self.search_peers.as_deref().map(|v| v.len()).unwrap_or(0),
                ) {
                    self.search_peers_state.select(Some(index));
                }
                None
            }
            CurrentScreen::Forwarders => {
                if let Some(index) = calculate_table_click_index(
                    row,
                    HEADER_HEIGHT + 1,
                    self.forwarders_state.offset(),
                    self.forwarders.as_deref().map(|v| v.len()).unwrap_or(0),
                ) {
                    self.forwarders_state.select(Some(index));
                }
                None
            }
            CurrentScreen::Lookups => {
                if let Some(index) = calculate_table_click_index(
                    row,
                    HEADER_HEIGHT + 1,
                    self.lookups_state.offset(),
                    self.lookups.as_deref().map(|v| v.len()).unwrap_or(0),
                ) {
                    self.lookups_state.select(Some(index));
                }
                None
            }
            CurrentScreen::Audit => {
                if let Some(index) = calculate_table_click_index(
                    row,
                    HEADER_HEIGHT + 1,
                    self.audit_state.offset(),
                    self.audit_events.as_deref().map(|v| v.len()).unwrap_or(0),
                ) {
                    self.audit_state.select(Some(index));
                }
                None
            }
            CurrentScreen::InternalLogs => {
                if let Some(index) = calculate_table_click_index(
                    row,
                    HEADER_HEIGHT + 1,
                    self.internal_logs_state.offset(),
                    self.internal_logs.as_deref().map(|v| v.len()).unwrap_or(0),
                ) {
                    self.internal_logs_state.select(Some(index));
                }
                None
            }

            // Multi-view screens (need view_mode check)
            CurrentScreen::Cluster => {
                if self.cluster_view_mode == crate::app::state::ClusterViewMode::Peers {
                    if let Some(index) = calculate_table_click_index(
                        row,
                        HEADER_HEIGHT + 1,
                        self.cluster_peers_state.offset(),
                        self.cluster_peers.as_deref().map(|v| v.len()).unwrap_or(0),
                    ) {
                        self.cluster_peers_state.select(Some(index));
                    }
                }
                None
            }
            CurrentScreen::WorkloadManagement => {
                match self.workload_view_mode {
                    crate::app::state::WorkloadViewMode::Pools => {
                        if let Some(index) = calculate_table_click_index(
                            row,
                            HEADER_HEIGHT + 1,
                            self.workload_pools_state.offset(),
                            self.workload_pools.as_deref().map(|v| v.len()).unwrap_or(0),
                        ) {
                            self.workload_pools_state.select(Some(index));
                        }
                    }
                    crate::app::state::WorkloadViewMode::Rules => {
                        if let Some(index) = calculate_table_click_index(
                            row,
                            HEADER_HEIGHT + 1,
                            self.workload_rules_state.offset(),
                            self.workload_rules.as_deref().map(|v| v.len()).unwrap_or(0),
                        ) {
                            self.workload_rules_state.select(Some(index));
                        }
                    }
                }
                None
            }
            CurrentScreen::Shc => {
                if self.shc_view_mode == crate::app::state::ShcViewMode::Members {
                    if let Some(index) = calculate_table_click_index(
                        row,
                        HEADER_HEIGHT + 1,
                        self.shc_members_state.offset(),
                        self.shc_members.as_deref().map(|v| v.len()).unwrap_or(0),
                    ) {
                        self.shc_members_state.select(Some(index));
                    }
                }
                None
            }
            CurrentScreen::Configs => {
                match self.config_view_mode {
                    crate::ui::screens::configs::ConfigViewMode::FileList => {
                        if let Some(index) = calculate_table_click_index(
                            row,
                            HEADER_HEIGHT + 1,
                            self.config_files_state.offset(),
                            self.config_files.as_deref().map(|v| v.len()).unwrap_or(0),
                        ) {
                            self.config_files_state.select(Some(index));
                        }
                    }
                    crate::ui::screens::configs::ConfigViewMode::StanzaList
                    | crate::ui::screens::configs::ConfigViewMode::StanzaDetail => {
                        let (offset, total) = if self.config_search_mode {
                            (0, self.filtered_stanza_indices.len())
                        } else {
                            (
                                self.config_stanzas_state.offset(),
                                self.config_stanzas.as_deref().map(|v| v.len()).unwrap_or(0),
                            )
                        };
                        if let Some(index) =
                            calculate_table_click_index(row, HEADER_HEIGHT + 1, offset, total)
                        {
                            self.config_stanzas_state.select(Some(index));
                        }
                    }
                }
                None
            }

            // Non-list screens (no mouse selection)
            _ => None,
        }
    }

    /// Special handler for Jobs screen (has filter offset).
    fn handle_jobs_click(&mut self, row: u16) -> Option<Action> {
        let filter_offset = if self.is_filtering || self.search_filter.is_some() {
            3
        } else {
            0
        };
        let header_row = HEADER_HEIGHT + filter_offset + 1;
        let offset = self.jobs_state.offset();
        let total = self.filtered_jobs_len();

        if let Some(index) = calculate_table_click_index(row, header_row, offset, total) {
            let already_selected = self.jobs_state.selected() == Some(index);
            self.jobs_state.select(Some(index));
            if already_selected {
                return Some(Action::InspectJob);
            }
        }
        None
    }
}

/// Calculate the data index from a click row for a plain list (no table header).
/// Returns None if the click is outside the data area.
fn calculate_list_click_index(
    click_row: u16,
    data_start_row: u16,
    offset: usize,
    total_items: usize,
) -> Option<usize> {
    if click_row < data_start_row {
        return None;
    }
    let relative_row = (click_row - data_start_row) as usize;
    let index = offset + relative_row;
    if index < total_items {
        Some(index)
    } else {
        None
    }
}

/// Calculate the data index from a click row for a table with a header row.
/// Data starts one row below the header.
fn calculate_table_click_index(
    click_row: u16,
    header_row: u16,
    offset: usize,
    total_items: usize,
) -> Option<usize> {
    let data_start = header_row + 1;
    if click_row < data_start {
        return None;
    }
    let relative_row = (click_row - data_start) as usize;
    let index = offset + relative_row;
    if index < total_items {
        Some(index)
    } else {
        None
    }
}

// ============================================================================
// Popup Mouse Handling
// ============================================================================

impl App {
    /// Handle mouse input when a popup is active.
    fn handle_popup_mouse(&mut self, mouse: MouseEvent) -> Option<Action> {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.handle_popup_click(mouse.column, mouse.row)
            }
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                // Scroll events in popups - currently ignored
                // Could be implemented for scrollable popups (Help, ErrorDetails, etc.)
                None
            }
            _ => None,
        }
    }

    /// Calculate the popup area on screen.
    /// Uses the same calculation as render.rs:centered_rect().
    fn get_popup_area(&self) -> Option<Rect> {
        let popup = self.popup.as_ref()?;
        if !popup.kind.is_confirmation() {
            return None;
        }
        let screen = self.last_area;

        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - POPUP_HEIGHT_PERCENT) / 2),
                Constraint::Percentage(POPUP_HEIGHT_PERCENT),
                Constraint::Percentage((100 - POPUP_HEIGHT_PERCENT) / 2),
            ])
            .split(screen);

        Some(
            Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage((100 - POPUP_WIDTH_PERCENT) / 2),
                    Constraint::Percentage(POPUP_WIDTH_PERCENT),
                    Constraint::Percentage((100 - POPUP_WIDTH_PERCENT) / 2),
                ])
                .split(popup_layout[1])[1],
        )
    }

    /// Handle mouse clicks when a popup is active.
    fn handle_popup_click(&mut self, col: u16, row: u16) -> Option<Action> {
        let popup_area = self.get_popup_area()?;

        if col < popup_area.x
            || col >= popup_area.x + popup_area.width
            || row < popup_area.y
            || row >= popup_area.y + popup_area.height
        {
            self.popup = None;
            return None;
        }

        let popup = self.popup.take()?;
        if popup.kind.is_confirmation() {
            self.execute_confirmation_action(popup.kind)
        } else {
            self.popup = Some(popup);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConnectionContext;
    use crate::app::state::{CurrentScreen, HEADER_HEIGHT};
    use crossterm::event::{KeyModifiers, MouseButton, MouseEventKind};

    #[test]
    fn test_handle_mouse_scroll() {
        let mut app = App::new(None, ConnectionContext::default());

        // Scroll Down
        let event_down = MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::empty(),
        };
        let action_down = app.handle_mouse(event_down);
        assert!(matches!(action_down, Some(Action::NavigateDown)));

        // Scroll Up
        let event_up = MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::empty(),
        };
        let action_up = app.handle_mouse(event_up);
        assert!(matches!(action_up, Some(Action::NavigateUp)));
    }

    #[test]
    fn test_handle_mouse_footer_click() {
        let mut app = App::new(None, ConnectionContext::default());
        app.last_area = ratatui::layout::Rect::new(0, 0, 80, 24);

        // Footer navigation clicks are no longer supported (navigation is keyboard-only via Tab/Shift+Tab)
        // Only quit button clicks work now
        // Use FooterLayout to get the correct quit button position
        let layout = FooterLayout::calculate(false, 0.0, app.current_screen, app.last_area.width);

        // Click in the middle of the quit button (accounting for border)
        let click_col = layout.quit_start + 1 + 3; // +1 for border, +3 for middle of quit button
        let event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: click_col,
            row: 22, // Middle line of footer (24-2)
            modifiers: KeyModifiers::empty(),
        };

        let action = app.handle_mouse(event);
        assert!(
            matches!(action, Some(Action::Quit)),
            "Clicking quit button should return Quit action (col={})",
            click_col
        );
    }

    #[test]
    fn test_handle_mouse_content_click_jobs() {
        use splunk_client::models::SearchJobStatus;

        let mut app = App::new(None, ConnectionContext::default());
        app.last_area = ratatui::layout::Rect::new(0, 0, 80, 24);
        app.current_screen = CurrentScreen::Jobs;
        app.jobs = Some(vec![
            SearchJobStatus {
                sid: "job1".to_string(),
                is_done: true,
                is_finalized: true,
                done_progress: 1.0,
                run_duration: 1.0,
                cursor_time: None,
                scan_count: 0,
                event_count: 0,
                result_count: 0,
                disk_usage: 0,
                priority: None,
                label: None,
            },
            SearchJobStatus {
                sid: "job2".to_string(),
                is_done: true,
                is_finalized: true,
                done_progress: 1.0,
                run_duration: 1.0,
                cursor_time: None,
                scan_count: 0,
                event_count: 0,
                result_count: 0,
                disk_usage: 0,
                priority: None,
                label: None,
            },
        ]);
        app.rebuild_filtered_indices();

        // Click second job
        // Header (4) + Table Header (1) + first row (1) = Row 6
        let event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 10,
            row: 7, // Second row of data
            modifiers: KeyModifiers::empty(),
        };

        let action = app.handle_mouse(event);
        // First click should just select
        assert!(action.is_none());
        assert_eq!(app.jobs_state.selected(), Some(1));

        // Second click on same row should Inspect
        let action2 = app.handle_mouse(event);
        assert!(matches!(action2, Some(Action::InspectJob)));
    }

    #[test]
    fn test_handle_mouse_content_click_jobs_with_filter_offset() {
        use splunk_client::models::SearchJobStatus;

        let mut app = App::new(None, ConnectionContext::default());
        app.last_area = ratatui::layout::Rect::new(0, 0, 80, 24);
        app.current_screen = CurrentScreen::Jobs;
        app.jobs = Some(vec![
            SearchJobStatus {
                sid: "job1".to_string(),
                is_done: true,
                is_finalized: true,
                done_progress: 1.0,
                run_duration: 1.0,
                cursor_time: None,
                scan_count: 0,
                event_count: 0,
                result_count: 0,
                disk_usage: 0,
                priority: None,
                label: None,
            },
            SearchJobStatus {
                sid: "job2".to_string(),
                is_done: true,
                is_finalized: true,
                done_progress: 1.0,
                run_duration: 1.0,
                cursor_time: None,
                scan_count: 0,
                event_count: 0,
                result_count: 0,
                disk_usage: 0,
                priority: None,
                label: None,
            },
        ]);
        app.rebuild_filtered_indices();

        app.is_filtering = true;
        app.search_filter = Some("foo".to_string());

        let filter_offset = 3;
        let data_start = HEADER_HEIGHT + filter_offset + 2;
        let event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 10,
            row: data_start + 1, // Click second row (index 1) after filter offset
            modifiers: KeyModifiers::empty(),
        };

        // First click selects the filtered row
        let action = app.handle_mouse(event);
        assert!(action.is_none());
        assert_eq!(app.jobs_state.selected(), Some(1));

        // Second click on the same row should inspect
        let action2 = app.handle_mouse(event);
        assert!(matches!(action2, Some(Action::InspectJob)));
    }

    #[test]
    fn test_footer_click_quit_at_0_percent() {
        let mut app = App::new(None, ConnectionContext::default());
        app.last_area = ratatui::layout::Rect::new(0, 0, 100, 24);
        app.loading = true;
        app.progress = 0.0;

        // Calculate expected quit position using FooterLayout
        let layout = FooterLayout::calculate(true, 0.0, app.current_screen, app.last_area.width);

        // Click in the middle of the quit button (accounting for border)
        let click_col = layout.quit_start + 1 + 3; // +1 for border, +3 for middle of quit button
        let event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: click_col,
            row: 22,
            modifiers: KeyModifiers::empty(),
        };

        let action = app.handle_mouse(event);
        assert!(
            matches!(action, Some(Action::Quit)),
            "Clicking quit at 0% progress should return Quit action (col={})",
            click_col
        );
    }

    #[test]
    fn test_footer_click_quit_at_9_percent() {
        let mut app = App::new(None, ConnectionContext::default());
        app.last_area = ratatui::layout::Rect::new(0, 0, 100, 24);
        app.loading = true;
        app.progress = 0.09;

        let layout = FooterLayout::calculate(true, 0.09, app.current_screen, app.last_area.width);

        let click_col = layout.quit_start + 1 + 3;
        let event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: click_col,
            row: 22,
            modifiers: KeyModifiers::empty(),
        };

        let action = app.handle_mouse(event);
        assert!(
            matches!(action, Some(Action::Quit)),
            "Clicking quit at 9% progress should return Quit action (col={})",
            click_col
        );
    }

    #[test]
    fn test_footer_click_quit_at_10_percent() {
        let mut app = App::new(None, ConnectionContext::default());
        app.last_area = ratatui::layout::Rect::new(0, 0, 100, 24);
        app.loading = true;
        app.progress = 0.10;

        let layout = FooterLayout::calculate(true, 0.10, app.current_screen, app.last_area.width);

        let click_col = layout.quit_start + 1 + 3;
        let event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: click_col,
            row: 22,
            modifiers: KeyModifiers::empty(),
        };

        let action = app.handle_mouse(event);
        assert!(
            matches!(action, Some(Action::Quit)),
            "Clicking quit at 10% progress should return Quit action (col={})",
            click_col
        );
    }

    #[test]
    fn test_footer_click_quit_at_100_percent() {
        let mut app = App::new(None, ConnectionContext::default());
        app.last_area = ratatui::layout::Rect::new(0, 0, 100, 24);
        app.loading = true;
        app.progress = 1.0;

        let layout = FooterLayout::calculate(true, 1.0, app.current_screen, app.last_area.width);

        let click_col = layout.quit_start + 1 + 3;
        let event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: click_col,
            row: 22,
            modifiers: KeyModifiers::empty(),
        };

        let action = app.handle_mouse(event);
        assert!(
            matches!(action, Some(Action::Quit)),
            "Clicking quit at 100% progress should return Quit action (col={})",
            click_col
        );
    }

    #[test]
    fn test_footer_click_quit_narrow_terminal() {
        let mut app = App::new(None, ConnectionContext::default());
        // Narrow terminal (width < 60)
        app.last_area = ratatui::layout::Rect::new(0, 0, 50, 24);
        app.loading = false;

        let layout = FooterLayout::calculate(false, 0.0, app.current_screen, app.last_area.width);

        // Quit should still be visible at width 50
        assert!(layout.quit_visible, "Quit should be visible at width 50");

        let click_col = layout.quit_start + 1 + 3;
        let event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: click_col,
            row: 22,
            modifiers: KeyModifiers::empty(),
        };

        let action = app.handle_mouse(event);
        assert!(
            matches!(action, Some(Action::Quit)),
            "Clicking quit on narrow terminal should return Quit action (col={})",
            click_col
        );
    }

    #[test]
    fn test_footer_click_quit_very_narrow() {
        let mut app = App::new(None, ConnectionContext::default());
        // Very narrow terminal (width < 40)
        app.last_area = ratatui::layout::Rect::new(0, 0, 35, 24);
        app.loading = false;

        let layout = FooterLayout::calculate(false, 0.0, app.current_screen, app.last_area.width);

        // Quit should NOT be visible at width 35
        assert!(!layout.quit_visible, "Quit should be hidden at width 35");

        // Clicking where quit would be should not return Quit action
        let event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 30,
            row: 22,
            modifiers: KeyModifiers::empty(),
        };

        let action = app.handle_mouse(event);
        assert!(
            !matches!(action, Some(Action::Quit)),
            "Clicking quit area on very narrow terminal should NOT return Quit action"
        );
    }

    #[test]
    fn test_footer_click_quit_jobs_screen() {
        let mut app = App::new(None, ConnectionContext::default());
        app.last_area = ratatui::layout::Rect::new(0, 0, 100, 24);
        app.current_screen = CurrentScreen::Jobs;
        app.loading = false;

        let layout = FooterLayout::calculate(false, 0.0, app.current_screen, app.last_area.width);

        let click_col = layout.quit_start + 1 + 3;
        let event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: click_col,
            row: 22,
            modifiers: KeyModifiers::empty(),
        };

        let action = app.handle_mouse(event);
        assert!(
            matches!(action, Some(Action::Quit)),
            "Clicking quit on Jobs screen should return Quit action"
        );
    }

    #[test]
    fn test_footer_click_quit_search_screen() {
        let mut app = App::new(None, ConnectionContext::default());
        app.last_area = ratatui::layout::Rect::new(0, 0, 100, 24);
        app.current_screen = CurrentScreen::Search;
        app.loading = false;

        let layout = FooterLayout::calculate(false, 0.0, app.current_screen, app.last_area.width);

        let click_col = layout.quit_start + 1 + 3;
        let event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: click_col,
            row: 22,
            modifiers: KeyModifiers::empty(),
        };

        let action = app.handle_mouse(event);
        assert!(
            matches!(action, Some(Action::Quit)),
            "Clicking quit on Search screen should return Quit action"
        );
    }

    #[test]
    fn test_footer_click_outside_quit_does_nothing() {
        let mut app = App::new(None, ConnectionContext::default());
        app.last_area = ratatui::layout::Rect::new(0, 0, 100, 24);
        app.loading = false;

        // Click in the nav area (left side of footer)
        let event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 10, // In the nav text area
            row: 22,
            modifiers: KeyModifiers::empty(),
        };

        let action = app.handle_mouse(event);
        assert!(
            action.is_none(),
            "Clicking in nav area should not return any action"
        );
    }

    #[test]
    fn test_footer_click_with_loading_progress_variations() {
        // Test that different progress values result in correct quit button positions
        let test_cases = [
            (0.0, 16u16), // 0% = 16 chars
            (0.05, 16),   // 5% = 16 chars
            (0.09, 16),   // 9% = 16 chars
            (0.10, 17),   // 10% = 17 chars
            (0.50, 17),   // 50% = 17 chars
            (0.99, 17),   // 99% = 17 chars
            (1.0, 18),    // 100% = 18 chars
        ];

        for (progress, expected_loading_width) in test_cases {
            let layout = FooterLayout::calculate(true, progress, CurrentScreen::Jobs, 100);

            assert_eq!(
                layout.loading_width, expected_loading_width,
                "Progress {} should have loading width {}",
                progress, expected_loading_width
            );
        }
    }

    #[test]
    fn test_calculate_list_click_index_in_bounds() {
        assert_eq!(calculate_list_click_index(5, 5, 0, 10), Some(0));
        assert_eq!(calculate_list_click_index(7, 5, 0, 10), Some(2));
        assert_eq!(calculate_list_click_index(5, 5, 5, 10), Some(5));
        assert_eq!(calculate_list_click_index(14, 5, 0, 10), Some(9));
    }

    #[test]
    fn test_calculate_list_click_index_out_of_bounds() {
        assert_eq!(calculate_list_click_index(4, 5, 0, 10), None);
        assert_eq!(calculate_list_click_index(20, 5, 0, 10), None);
        assert_eq!(calculate_list_click_index(5, 5, 10, 10), None);
    }

    #[test]
    fn test_calculate_table_click_index_in_bounds() {
        assert_eq!(calculate_table_click_index(6, 5, 0, 10), Some(0));
        assert_eq!(calculate_table_click_index(8, 5, 0, 10), Some(2));
        assert_eq!(calculate_table_click_index(6, 5, 5, 10), Some(5));
        assert_eq!(calculate_table_click_index(15, 5, 0, 10), Some(9));
    }

    #[test]
    fn test_calculate_table_click_index_header_click() {
        assert_eq!(calculate_table_click_index(5, 5, 0, 10), None);
        assert_eq!(calculate_table_click_index(4, 5, 0, 10), None);
    }

    #[test]
    fn test_handle_mouse_content_click_apps() {
        use splunk_client::models::App as SplunkApp;

        let mut app = App::new(None, ConnectionContext::default());
        app.last_area = ratatui::layout::Rect::new(0, 0, 80, 24);
        app.current_screen = CurrentScreen::Apps;
        app.apps = Some(vec![
            SplunkApp {
                name: "app1".to_string(),
                label: Some("App One".to_string()),
                version: Some("1.0".to_string()),
                is_configured: None,
                is_visible: None,
                disabled: false,
                description: None,
                author: None,
            },
            SplunkApp {
                name: "app2".to_string(),
                label: Some("App Two".to_string()),
                version: Some("2.0".to_string()),
                is_configured: None,
                is_visible: None,
                disabled: false,
                description: None,
                author: None,
            },
        ]);

        let event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 10,
            row: HEADER_HEIGHT + 2,
            modifiers: KeyModifiers::empty(),
        };

        let action = app.handle_mouse(event);
        assert!(action.is_none());
        assert_eq!(app.apps_state.selected(), Some(1));
    }

    #[test]
    fn test_handle_mouse_content_click_inputs() {
        use splunk_client::models::{Input, InputType};

        let mut app = App::new(None, ConnectionContext::default());
        app.last_area = ratatui::layout::Rect::new(0, 0, 80, 24);
        app.current_screen = CurrentScreen::Inputs;
        app.inputs = Some(vec![
            Input {
                name: "input1".to_string(),
                input_type: InputType::Monitor,
                disabled: false,
                host: None,
                source: None,
                sourcetype: None,
                connection_host: None,
                port: None,
                path: None,
                blacklist: None,
                whitelist: None,
                recursive: None,
                command: None,
                interval: None,
            },
            Input {
                name: "input2".to_string(),
                input_type: InputType::Udp,
                disabled: false,
                host: None,
                source: None,
                sourcetype: None,
                connection_host: None,
                port: None,
                path: None,
                blacklist: None,
                whitelist: None,
                recursive: None,
                command: None,
                interval: None,
            },
        ]);

        let event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 10,
            row: HEADER_HEIGHT + 3,
            modifiers: KeyModifiers::empty(),
        };

        let action = app.handle_mouse(event);
        assert!(action.is_none());
        assert_eq!(app.inputs_state.selected(), Some(1));
    }

    #[test]
    fn test_handle_mouse_content_click_cluster_peers_view() {
        use crate::app::state::ClusterViewMode;
        use splunk_client::models::{ClusterPeer, PeerState, PeerStatus, ReplicationStatus};

        let mut app = App::new(None, ConnectionContext::default());
        app.last_area = ratatui::layout::Rect::new(0, 0, 80, 24);
        app.current_screen = CurrentScreen::Cluster;
        app.cluster_view_mode = ClusterViewMode::Peers;
        app.cluster_peers = Some(vec![
            ClusterPeer {
                id: "peer1".to_string(),
                label: Some("Peer 1".to_string()),
                status: PeerStatus::Up,
                peer_state: PeerState::Searchable,
                site: Some("site1".to_string()),
                guid: "guid-1".to_string(),
                host: "host-1".to_string(),
                port: 8080,
                replication_count: Some(0),
                replication_status: Some(ReplicationStatus::Complete),
                bundle_replication_count: None,
                is_captain: Some(false),
            },
            ClusterPeer {
                id: "peer2".to_string(),
                label: Some("Peer 2".to_string()),
                status: PeerStatus::Up,
                peer_state: PeerState::Searchable,
                site: Some("site1".to_string()),
                guid: "guid-2".to_string(),
                host: "host-2".to_string(),
                port: 8080,
                replication_count: Some(0),
                replication_status: Some(ReplicationStatus::Complete),
                bundle_replication_count: None,
                is_captain: Some(false),
            },
        ]);

        let event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 10,
            row: HEADER_HEIGHT + 3,
            modifiers: KeyModifiers::empty(),
        };

        let action = app.handle_mouse(event);
        assert!(action.is_none());
        assert_eq!(app.cluster_peers_state.selected(), Some(1));
    }

    #[test]
    fn test_handle_mouse_content_click_cluster_summary_view() {
        use crate::app::state::ClusterViewMode;

        let mut app = App::new(None, ConnectionContext::default());
        app.last_area = ratatui::layout::Rect::new(0, 0, 80, 24);
        app.current_screen = CurrentScreen::Cluster;
        app.cluster_view_mode = ClusterViewMode::Summary;
        app.cluster_peers = Some(vec![]);
        app.cluster_peers_state.select(None);

        let event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 10,
            row: HEADER_HEIGHT + 2,
            modifiers: KeyModifiers::empty(),
        };

        let action = app.handle_mouse(event);
        assert!(action.is_none());
        assert!(app.cluster_peers_state.selected().is_none());
    }

    #[test]
    fn test_handle_mouse_content_click_workload_pools() {
        use crate::app::state::WorkloadViewMode;
        use splunk_client::models::WorkloadPool;

        let mut app = App::new(None, ConnectionContext::default());
        app.last_area = ratatui::layout::Rect::new(0, 0, 80, 24);
        app.current_screen = CurrentScreen::WorkloadManagement;
        app.workload_view_mode = WorkloadViewMode::Pools;
        app.workload_pools = Some(vec![
            WorkloadPool {
                name: "pool1".to_string(),
                cpu_weight: None,
                mem_weight: None,
                default_pool: None,
                enabled: None,
                search_concurrency: None,
                search_time_range: None,
                admission_rules_enabled: None,
                cpu_cores: None,
                mem_limit: None,
            },
            WorkloadPool {
                name: "pool2".to_string(),
                cpu_weight: None,
                mem_weight: None,
                default_pool: None,
                enabled: None,
                search_concurrency: None,
                search_time_range: None,
                admission_rules_enabled: None,
                cpu_cores: None,
                mem_limit: None,
            },
        ]);

        let event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 10,
            row: HEADER_HEIGHT + 3,
            modifiers: KeyModifiers::empty(),
        };

        let action = app.handle_mouse(event);
        assert!(action.is_none());
        assert_eq!(app.workload_pools_state.selected(), Some(1));
    }
}
