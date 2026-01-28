//! Mouse event handling for the TUI app.
//!
//! Responsibilities:
//! - Handle mouse scroll events
//! - Handle footer button clicks
//! - Handle content area clicks for selection
//!
//! Non-responsibilities:
//! - Does NOT handle keyboard input
//! - Does NOT render the UI

use crate::action::Action;
use crate::app::App;
use crate::app::footer_layout::FooterLayout;
use crate::app::state::{CurrentScreen, HEADER_HEIGHT};
use crossterm::event::{MouseEvent, MouseEventKind};

impl App {
    /// Handle mouse input - returns Action if one should be dispatched.
    pub fn handle_mouse(&mut self, mouse: MouseEvent) -> Option<Action> {
        if self.popup.is_some() {
            return None;
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
        let layout = FooterLayout::calculate(
            self.loading,
            f64::from(self.progress),
            self.current_screen,
            self.last_area.width,
        );

        if layout.is_quit_clicked(col) {
            Some(Action::Quit)
        } else {
            None
        }
    }

    /// Handle clicks in the main content area.
    fn handle_content_click(&mut self, row: u16, _col: u16) -> Option<Action> {
        match self.current_screen {
            CurrentScreen::Jobs => {
                // If filtering is active, the table area is pushed down by 3 rows
                let filter_offset = if self.is_filtering || self.search_filter.is_some() {
                    3
                } else {
                    0
                };

                // Jobs table has a header row at content start + 1
                // Data starts at content start + 2
                let data_start = HEADER_HEIGHT + filter_offset + 2;
                if row >= data_start {
                    let relative_row = (row - data_start) as usize;
                    let offset = self.jobs_state.offset();
                    let index = offset + relative_row;

                    if index < self.filtered_jobs_len() {
                        let already_selected = self.jobs_state.selected() == Some(index);
                        self.jobs_state.select(Some(index));
                        if already_selected {
                            return Some(Action::InspectJob);
                        }
                    }
                }
            }
            CurrentScreen::Indexes => {
                // Indexes list starts at HEADER_HEIGHT + 1 (no table header)
                let data_start = HEADER_HEIGHT + 1;
                if row >= data_start {
                    let relative_row = (row - data_start) as usize;
                    let offset = self.indexes_state.offset();
                    let index = offset + relative_row;

                    if let Some(indexes) = &self.indexes
                        && index < indexes.len()
                    {
                        self.indexes_state.select(Some(index));
                    }
                }
            }
            _ => {}
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConnectionContext;
    use crate::app::state::CurrentScreen;
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
}
