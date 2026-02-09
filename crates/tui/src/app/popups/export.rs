//! Export popup handler.
//!
//! Responsibilities:
//! - Handle export dialog input and format toggling
//! - Manage export filename input with automatic extension updates
//!
//! Does NOT handle:
//! - Does NOT render popups (handled by ui::popup module)
//! - Does NOT perform the actual export (just returns Action::ExportData)

use crate::action::{Action, ExportFormat};
use crate::app::App;
use crate::ui::ToastLevel;
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

                match self.collect_export_data() {
                    Ok(Some(data)) => {
                        let path = std::path::PathBuf::from(self.export_input.value());
                        let format = self.export_format;
                        self.popup = None;
                        self.export_target = None;
                        Some(Action::ExportData(data, path, format))
                    }
                    Ok(None) => {
                        // No data available (screen not loaded yet) - silently close
                        self.popup = None;
                        self.export_target = None;
                        None
                    }
                    Err(e) => {
                        // Serialization failed - show error toast and close popup
                        self.popup = None;
                        self.export_target = None;
                        Some(Action::Notify(
                            ToastLevel::Error,
                            format!("Export failed: {}", e),
                        ))
                    }
                }
            }
            KeyCode::Tab => {
                self.export_format = match self.export_format {
                    ExportFormat::Json => ExportFormat::Csv,
                    ExportFormat::Csv => ExportFormat::Ndjson,
                    ExportFormat::Ndjson => ExportFormat::Yaml,
                    ExportFormat::Yaml => ExportFormat::Markdown,
                    ExportFormat::Markdown => ExportFormat::Json,
                };
                // Automatically update extension if it matches the previous format
                let current_value = self.export_input.value().to_string();
                let new_value = match self.export_format {
                    ExportFormat::Json => {
                        if current_value.ends_with(".csv") {
                            current_value[..current_value.len() - 4].to_string() + ".json"
                        } else if current_value.ends_with(".ndjson") {
                            current_value[..current_value.len() - 7].to_string() + ".json"
                        } else if current_value.ends_with(".yaml") {
                            current_value[..current_value.len() - 5].to_string() + ".json"
                        } else if current_value.ends_with(".md") {
                            current_value[..current_value.len() - 3].to_string() + ".json"
                        } else {
                            current_value
                        }
                    }
                    ExportFormat::Csv => {
                        if current_value.ends_with(".json") {
                            current_value[..current_value.len() - 5].to_string() + ".csv"
                        } else if current_value.ends_with(".ndjson") {
                            current_value[..current_value.len() - 7].to_string() + ".csv"
                        } else if current_value.ends_with(".yaml") {
                            current_value[..current_value.len() - 5].to_string() + ".csv"
                        } else if current_value.ends_with(".md") {
                            current_value[..current_value.len() - 3].to_string() + ".csv"
                        } else {
                            current_value
                        }
                    }
                    ExportFormat::Ndjson => {
                        if current_value.ends_with(".json") {
                            current_value[..current_value.len() - 5].to_string() + ".ndjson"
                        } else if current_value.ends_with(".csv") {
                            current_value[..current_value.len() - 4].to_string() + ".ndjson"
                        } else if current_value.ends_with(".yaml") {
                            current_value[..current_value.len() - 5].to_string() + ".ndjson"
                        } else if current_value.ends_with(".md") {
                            current_value[..current_value.len() - 3].to_string() + ".ndjson"
                        } else {
                            current_value
                        }
                    }
                    ExportFormat::Yaml => {
                        if current_value.ends_with(".json") {
                            current_value[..current_value.len() - 5].to_string() + ".yaml"
                        } else if current_value.ends_with(".csv") {
                            current_value[..current_value.len() - 4].to_string() + ".yaml"
                        } else if current_value.ends_with(".ndjson") {
                            current_value[..current_value.len() - 7].to_string() + ".yaml"
                        } else if current_value.ends_with(".md") {
                            current_value[..current_value.len() - 3].to_string() + ".yaml"
                        } else {
                            current_value
                        }
                    }
                    ExportFormat::Markdown => {
                        if current_value.ends_with(".json") {
                            current_value[..current_value.len() - 5].to_string() + ".md"
                        } else if current_value.ends_with(".csv") {
                            current_value[..current_value.len() - 4].to_string() + ".md"
                        } else if current_value.ends_with(".ndjson") {
                            current_value[..current_value.len() - 7].to_string() + ".md"
                        } else if current_value.ends_with(".yaml") {
                            current_value[..current_value.len() - 5].to_string() + ".md"
                        } else {
                            current_value
                        }
                    }
                };
                self.export_input.set_value(new_value);
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
