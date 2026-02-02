//! Export popup handler.
//!
//! Responsibilities:
//! - Handle export dialog input and format toggling
//! - Manage export filename input with automatic extension updates
//!
//! Non-responsibilities:
//! - Does NOT render popups (handled by ui::popup module)
//! - Does NOT perform the actual export (just returns Action::ExportData)

use crate::action::{Action, ExportFormat};
use crate::app::App;
use crossterm::event::{KeyCode, KeyEvent};

impl App {
    /// Handle export search popup.
    pub fn handle_export_popup(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Esc => {
                self.popup = None;
                self.export_target = None;
                None
            }
            KeyCode::Enter => {
                if self.export_input.is_empty() {
                    return None;
                }

                if let Some(data) = self.collect_export_data() {
                    let path = std::path::PathBuf::from(&self.export_input);
                    let format = self.export_format;
                    self.popup = None;
                    self.export_target = None;
                    Some(Action::ExportData(data, path, format))
                } else {
                    None
                }
            }
            KeyCode::Tab => {
                self.export_format = match self.export_format {
                    ExportFormat::Json => ExportFormat::Csv,
                    ExportFormat::Csv => ExportFormat::Json,
                };
                // Automatically update extension if it matches the previous format
                match self.export_format {
                    ExportFormat::Json => {
                        if self.export_input.ends_with(".csv") {
                            self.export_input.truncate(self.export_input.len() - 4);
                            self.export_input.push_str(".json");
                        }
                    }
                    ExportFormat::Csv => {
                        if self.export_input.ends_with(".json") {
                            self.export_input.truncate(self.export_input.len() - 5);
                            self.export_input.push_str(".csv");
                        }
                    }
                }
                self.update_export_popup();
                None
            }
            KeyCode::Backspace => {
                self.export_input.pop();
                self.update_export_popup();
                None
            }
            KeyCode::Char(c) => {
                self.export_input.push(c);
                self.update_export_popup();
                None
            }
            _ => None,
        }
    }
}
