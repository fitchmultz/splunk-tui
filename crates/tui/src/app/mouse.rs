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
        // Content in the footer block starts at column 1 (due to border)
        // Footer layout without loading: " Tab:Next Screen | Shift+Tab:Previous Screen | q:Quit "
        // Footer layout with loading: " Loading... 100% | Tab:Next Screen | Shift+Tab:Previous Screen | q:Quit "

        // Calculate quit button position based on footer layout
        let nav_text_len = 45; // " Tab:Next Screen | Shift+Tab:Previous Screen " (including leading space)
        let sep_len = 1; // "|"
        let quit_text_len = 8; // " q:Quit " (including spaces)

        let quit_start = if self.loading {
            // With loading: " Loading... 100% |" (18 chars) + nav_text + "|" + quit
            18 + nav_text_len + sep_len
        } else {
            // Without loading: nav_text + "|" + quit
            nav_text_len + sep_len
        };
        let quit_end = quit_start + quit_text_len;

        if col > quit_start && col <= quit_end {
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
    use crate::app::state::CurrentScreen;
    use crossterm::event::{KeyModifiers, MouseButton, MouseEventKind};

    #[test]
    fn test_handle_mouse_scroll() {
        let mut app = App::new(None);

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
        let mut app = App::new(None);
        app.last_area = ratatui::layout::Rect::new(0, 0, 80, 24);

        // Footer navigation clicks are no longer supported (navigation is keyboard-only via Tab/Shift+Tab)
        // Only quit button clicks work now
        // Footer layout: " Tab:Next Screen | Shift+Tab:Previous Screen | q:Quit "
        // Quit button starts at column 46, ends at 54
        let event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 50, // middle of " q:Quit "
            row: 22,    // Middle line of footer (24-2)
            modifiers: KeyModifiers::empty(),
        };

        let action = app.handle_mouse(event);
        assert!(matches!(action, Some(Action::Quit)));
    }

    #[test]
    fn test_handle_mouse_content_click_jobs() {
        use splunk_client::models::SearchJobStatus;

        let mut app = App::new(None);
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
        // Header (3) + Table Header (1) + first row (1) = Row 5
        let event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 10,
            row: 6, // Second row of data
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
}
