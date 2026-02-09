//! Export functionality for the TUI app.
//!
//! Responsibilities:
//! - Define export targets and formats
//! - Collect data for export from various screens
//! - Manage export popup state
//!
//! Does NOT handle:
//! - Does NOT perform actual file I/O (handled by Action::ExportData)
//! - Does NOT render UI (handled by popup module)

use crate::action::ExportFormat;
use crate::app::App;
use crate::app::input::components::SingleLineInput;
use crate::ui::popup::{Popup, PopupType};

/// Identifies which screen's data should be exported when the export popup is confirmed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportTarget {
    SearchResults,
    Indexes,
    Users,
    Roles,
    Apps,
    SavedSearches,
    Macros,
    ClusterInfo,
    Jobs,
    Health,
    License,
    Kvstore,
    InternalLogs,
    Overview,
    SearchPeers,
    FiredAlerts,
    Forwarders,
    Lookups,
    MultiInstance,
    AuditEvents,
    Workload,
    ShcStatus,
}

impl ExportTarget {
    pub fn title(self) -> &'static str {
        match self {
            ExportTarget::SearchResults => "Export Search Results",
            ExportTarget::Indexes => "Export Indexes",
            ExportTarget::Users => "Export Users",
            ExportTarget::Roles => "Export Roles",
            ExportTarget::Apps => "Export Apps",
            ExportTarget::SavedSearches => "Export Saved Searches",
            ExportTarget::Macros => "Export Macros",
            ExportTarget::ClusterInfo => "Export Cluster Info",
            ExportTarget::Jobs => "Export Jobs",
            ExportTarget::Health => "Export Health",
            ExportTarget::License => "Export License",
            ExportTarget::Kvstore => "Export KVStore",
            ExportTarget::InternalLogs => "Export Internal Logs",
            ExportTarget::Overview => "Export Overview",
            ExportTarget::SearchPeers => "Export Search Peers",
            ExportTarget::FiredAlerts => "Export Fired Alerts",
            ExportTarget::Forwarders => "Export Forwarders",
            ExportTarget::Lookups => "Export Lookups",
            ExportTarget::MultiInstance => "Export Multi-Instance Dashboard",
            ExportTarget::AuditEvents => "Export Audit Events",
            ExportTarget::Workload => "Export Workload Management",
            ExportTarget::ShcStatus => "Export SHC Status",
        }
    }

    pub fn default_filename(self, format: ExportFormat) -> String {
        let base = match self {
            ExportTarget::SearchResults => "results",
            ExportTarget::Indexes => "indexes",
            ExportTarget::Users => "users",
            ExportTarget::Roles => "roles",
            ExportTarget::Apps => "apps",
            ExportTarget::SavedSearches => "saved-searches",
            ExportTarget::Macros => "macros",
            ExportTarget::ClusterInfo => "cluster-info",
            ExportTarget::Jobs => "jobs",
            ExportTarget::Health => "health",
            ExportTarget::License => "license",
            ExportTarget::Kvstore => "kvstore",
            ExportTarget::InternalLogs => "internal-logs",
            ExportTarget::Overview => "overview",
            ExportTarget::SearchPeers => "search-peers",
            ExportTarget::FiredAlerts => "fired-alerts",
            ExportTarget::Forwarders => "forwarders",
            ExportTarget::Lookups => "lookups",
            ExportTarget::MultiInstance => "multi-instance",
            ExportTarget::AuditEvents => "audit-events",
            ExportTarget::Workload => "workload",
            ExportTarget::ShcStatus => "shc-status",
        };

        let ext = match format {
            ExportFormat::Json => "json",
            ExportFormat::Csv => "csv",
            ExportFormat::Ndjson => "ndjson",
            ExportFormat::Yaml => "yaml",
            ExportFormat::Markdown => "md",
        };

        format!("{base}.{ext}")
    }
}

impl App {
    /// Begin an export flow for a specific screen's dataset.
    pub fn begin_export(&mut self, target: ExportTarget) {
        self.export_target = Some(target);
        self.export_input =
            SingleLineInput::with_value(target.default_filename(self.export_format));
        self.popup = Some(Popup::builder(PopupType::ExportSearch).build());
        self.update_export_popup();
    }

    /// Collect the dataset to export, pre-serialized as `serde_json::Value`.
    pub fn collect_export_data(&self) -> Result<Option<serde_json::Value>, String> {
        let target = self.export_target.unwrap_or(ExportTarget::SearchResults);

        match target {
            ExportTarget::SearchResults => {
                Ok(Some(serde_json::Value::Array(self.search_results.clone())))
            }
            ExportTarget::Indexes => self
                .indexes
                .as_ref()
                .map(|v| {
                    serde_json::to_value(v)
                        .map_err(|e| format!("Failed to serialize indexes: {}", e))
                })
                .transpose(),
            ExportTarget::Users => self
                .users
                .as_ref()
                .map(|v| {
                    serde_json::to_value(v).map_err(|e| format!("Failed to serialize users: {}", e))
                })
                .transpose(),
            ExportTarget::Roles => self
                .roles
                .as_ref()
                .map(|v| {
                    serde_json::to_value(v).map_err(|e| format!("Failed to serialize roles: {}", e))
                })
                .transpose(),
            ExportTarget::Apps => self
                .apps
                .as_ref()
                .map(|v| {
                    serde_json::to_value(v).map_err(|e| format!("Failed to serialize apps: {}", e))
                })
                .transpose(),
            ExportTarget::SavedSearches => self
                .saved_searches
                .as_ref()
                .map(|v| {
                    serde_json::to_value(v)
                        .map_err(|e| format!("Failed to serialize saved searches: {}", e))
                })
                .transpose(),
            ExportTarget::Macros => self
                .macros
                .as_ref()
                .map(|v| {
                    serde_json::to_value(v)
                        .map_err(|e| format!("Failed to serialize macros: {}", e))
                })
                .transpose(),
            ExportTarget::ClusterInfo => self
                .cluster_info
                .as_ref()
                .map(|v| {
                    serde_json::to_value(v)
                        .map_err(|e| format!("Failed to serialize cluster info: {}", e))
                })
                .transpose(),
            ExportTarget::Jobs => self
                .jobs
                .as_ref()
                .map(|v| {
                    serde_json::to_value(v).map_err(|e| format!("Failed to serialize jobs: {}", e))
                })
                .transpose(),
            ExportTarget::Health => self
                .health_info
                .as_ref()
                .map(|v| {
                    serde_json::to_value(v)
                        .map_err(|e| format!("Failed to serialize health: {}", e))
                })
                .transpose(),
            ExportTarget::License => self
                .license_info
                .as_ref()
                .map(|v| {
                    serde_json::to_value(v)
                        .map_err(|e| format!("Failed to serialize license: {}", e))
                })
                .transpose(),
            ExportTarget::Kvstore => self
                .kvstore_status
                .as_ref()
                .map(|v| {
                    serde_json::to_value(v)
                        .map_err(|e| format!("Failed to serialize kvstore: {}", e))
                })
                .transpose(),
            ExportTarget::InternalLogs => self
                .internal_logs
                .as_ref()
                .map(|v| {
                    serde_json::to_value(v)
                        .map_err(|e| format!("Failed to serialize internal logs: {}", e))
                })
                .transpose(),
            ExportTarget::Overview => self
                .overview_data
                .as_ref()
                .map(|v| {
                    serde_json::to_value(v)
                        .map_err(|e| format!("Failed to serialize overview: {}", e))
                })
                .transpose(),
            ExportTarget::SearchPeers => self
                .search_peers
                .as_ref()
                .map(|v| {
                    serde_json::to_value(v)
                        .map_err(|e| format!("Failed to serialize search peers: {}", e))
                })
                .transpose(),
            ExportTarget::FiredAlerts => self
                .fired_alerts
                .as_ref()
                .map(|v| {
                    serde_json::to_value(v)
                        .map_err(|e| format!("Failed to serialize fired alerts: {}", e))
                })
                .transpose(),
            ExportTarget::Forwarders => self
                .forwarders
                .as_ref()
                .map(|v| {
                    serde_json::to_value(v)
                        .map_err(|e| format!("Failed to serialize forwarders: {}", e))
                })
                .transpose(),
            ExportTarget::Lookups => self
                .lookups
                .as_ref()
                .map(|v| {
                    serde_json::to_value(v)
                        .map_err(|e| format!("Failed to serialize lookups: {}", e))
                })
                .transpose(),
            ExportTarget::MultiInstance => self
                .multi_instance_data
                .as_ref()
                .map(|v| {
                    serde_json::to_value(v)
                        .map_err(|e| format!("Failed to serialize multi-instance data: {}", e))
                })
                .transpose(),
            ExportTarget::AuditEvents => self
                .audit_events
                .as_ref()
                .map(|v| {
                    serde_json::to_value(v)
                        .map_err(|e| format!("Failed to serialize audit events: {}", e))
                })
                .transpose(),
            ExportTarget::Workload => {
                // Export both pools and rules as a combined object
                let pools = self.workload_pools.clone().unwrap_or_default();
                let rules = self.workload_rules.clone().unwrap_or_default();
                let combined = serde_json::json!({
                    "pools": pools,
                    "rules": rules,
                });
                Ok(Some(combined))
            }
            ExportTarget::ShcStatus => self
                .shc_status
                .as_ref()
                .map(|v| {
                    serde_json::to_value(v)
                        .map_err(|e| format!("Failed to serialize shc status: {}", e))
                })
                .transpose(),
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
                ExportFormat::Ndjson => "NDJSON",
                ExportFormat::Yaml => "YAML",
                ExportFormat::Markdown => "Markdown",
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
