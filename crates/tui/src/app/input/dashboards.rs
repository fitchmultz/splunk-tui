//! Input handler for the Dashboards screen.
//!
//! Responsibilities:
//! - Handle keyboard input for the dashboards list screen.
//!
//! Does NOT handle:
//! - Does NOT render UI (handled by screens::dashboards)
//! - Does NOT manage state directly (returns Actions for that)

use crossterm::event::{KeyCode, KeyEvent};

use crate::action::Action;
use crate::app::App;

impl App {
    /// Handle input for the Dashboards screen.
    pub fn handle_dashboards_input(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Char('r') => {
                // Reset pagination and reload
                self.dashboards_pagination.reset();
                Some(Action::LoadDashboards {
                    count: self.dashboards_pagination.page_size,
                    offset: 0,
                })
            }
            _ => None,
        }
    }
}
