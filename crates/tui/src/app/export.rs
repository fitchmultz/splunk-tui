//! Export functionality for the TUI app.
//!
//! Responsibilities:
//! - Define export targets and formats
//! - Collect data for export from various screens
//! - Manage export popup state
//!
//! Non-responsibilities:
//! - Does NOT perform actual file I/O (handled by Action::ExportData)
//! - Does NOT render UI (handled by popup module)

use crate::action::ExportFormat;
use crate::app::App;
use crate::ui::popup::{Popup, PopupType};

/// Identifies which screen's data should be exported when the export popup is confirmed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportTarget {
    SearchResults,
    Indexes,
    Users,
    Apps,
    SavedSearches,
    ClusterInfo,
    Jobs,
    Health,
    InternalLogs,
}

impl ExportTarget {
    pub fn title(self) -> &'static str {
        match self {
            ExportTarget::SearchResults => "Export Search Results",
            ExportTarget::Indexes => "Export Indexes",
            ExportTarget::Users => "Export Users",
            ExportTarget::Apps => "Export Apps",
            ExportTarget::SavedSearches => "Export Saved Searches",
            ExportTarget::ClusterInfo => "Export Cluster Info",
            ExportTarget::Jobs => "Export Jobs",
            ExportTarget::Health => "Export Health",
            ExportTarget::InternalLogs => "Export Internal Logs",
        }
    }

    pub fn default_filename(self, format: ExportFormat) -> String {
        let base = match self {
            ExportTarget::SearchResults => "results",
            ExportTarget::Indexes => "indexes",
            ExportTarget::Users => "users",
            ExportTarget::Apps => "apps",
            ExportTarget::SavedSearches => "saved-searches",
            ExportTarget::ClusterInfo => "cluster-info",
            ExportTarget::Jobs => "jobs",
            ExportTarget::Health => "health",
            ExportTarget::InternalLogs => "internal-logs",
        };

        let ext = match format {
            ExportFormat::Json => "json",
            ExportFormat::Csv => "csv",
        };

        format!("{base}.{ext}")
    }
}

impl App {
    /// Begin an export flow for a specific screen's dataset.
    pub fn begin_export(&mut self, target: ExportTarget) {
        self.export_target = Some(target);
        self.export_input = target.default_filename(self.export_format);
        self.popup = Some(Popup::builder(PopupType::ExportSearch).build());
        self.update_export_popup();
    }

    /// Collect the dataset to export, pre-serialized as `serde_json::Value`.
    pub fn collect_export_data(&self) -> Option<serde_json::Value> {
        let target = self.export_target.unwrap_or(ExportTarget::SearchResults);

        match target {
            ExportTarget::SearchResults => {
                Some(serde_json::Value::Array(self.search_results.clone()))
            }
            ExportTarget::Indexes => self
                .indexes
                .as_ref()
                .and_then(|v| serde_json::to_value(v).ok()),
            ExportTarget::Users => self
                .users
                .as_ref()
                .and_then(|v| serde_json::to_value(v).ok()),
            ExportTarget::Apps => self
                .apps
                .as_ref()
                .and_then(|v| serde_json::to_value(v).ok()),
            ExportTarget::SavedSearches => self
                .saved_searches
                .as_ref()
                .and_then(|v| serde_json::to_value(v).ok()),
            ExportTarget::ClusterInfo => self
                .cluster_info
                .as_ref()
                .and_then(|v| serde_json::to_value(v).ok()),
            ExportTarget::Jobs => self
                .jobs
                .as_ref()
                .and_then(|v| serde_json::to_value(v).ok()),
            ExportTarget::Health => self
                .health_info
                .as_ref()
                .and_then(|v| serde_json::to_value(v).ok()),
            ExportTarget::InternalLogs => self
                .internal_logs
                .as_ref()
                .and_then(|v| serde_json::to_value(v).ok()),
        }
    }

    /// Refresh the export popup content based on current input, format, and target.
    pub fn update_export_popup(&mut self) {
        if let Some(Popup {
            kind: PopupType::ExportSearch,
            ..
        }) = &mut self.popup
        {
            let target = self.export_target.unwrap_or(ExportTarget::SearchResults);
            let format_str = match self.export_format {
                ExportFormat::Json => "JSON",
                ExportFormat::Csv => "CSV",
            };

            let popup = Popup::builder(PopupType::ExportSearch)
                .title(target.title())
                .content(format!(
                    "File: {}\nFormat: {} (Tab to toggle)\n\nPress Enter to export, Esc to cancel",
                    self.export_input, format_str
                ))
                .build();
            self.popup = Some(popup);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_target_title() {
        assert_eq!(ExportTarget::SearchResults.title(), "Export Search Results");
        assert_eq!(ExportTarget::Indexes.title(), "Export Indexes");
        assert_eq!(ExportTarget::Jobs.title(), "Export Jobs");
        assert_eq!(ExportTarget::Health.title(), "Export Health");
    }

    #[test]
    fn test_export_target_default_filename() {
        assert_eq!(
            ExportTarget::SearchResults.default_filename(ExportFormat::Json),
            "results.json"
        );
        assert_eq!(
            ExportTarget::SearchResults.default_filename(ExportFormat::Csv),
            "results.csv"
        );
        assert_eq!(
            ExportTarget::Indexes.default_filename(ExportFormat::Json),
            "indexes.json"
        );
        assert_eq!(
            ExportTarget::Jobs.default_filename(ExportFormat::Csv),
            "jobs.csv"
        );
    }
}
