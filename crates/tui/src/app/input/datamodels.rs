//! Input handler for the Data Models screen.
//!
//! Responsibilities:
//! - Handle keyboard input for the data models list screen.
//!
//! Does NOT handle:
//! - Does NOT render UI (handled by screens::datamodels)
//! - Does NOT manage state directly (returns Actions for that)

use crossterm::event::{KeyCode, KeyEvent};

use crate::action::Action;
use crate::app::App;

impl App {
    /// Handle input for the Data Models screen.
    pub fn handle_datamodels_input(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Char('r') => {
                // Reset pagination and reload
                self.data_models_pagination.reset();
                Some(Action::LoadDataModels {
                    count: self.data_models_pagination.page_size,
                    offset: 0,
                })
            }
            _ => None,
        }
    }
}
