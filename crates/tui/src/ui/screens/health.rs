//! Health screen rendering.
//!
//! Renders comprehensive Splunk environment health metrics including server info,
//! splunkd health, license usage, KVStore status, and log parsing health.

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    widgets::{Block, Borders, Paragraph},
};
use splunk_client::models::HealthCheckOutput;

/// Configuration for rendering the health screen.
pub struct HealthRenderConfig<'a> {
    /// Whether data is currently loading
    pub loading: bool,
    /// The health information to display
    pub health_info: Option<&'a HealthCheckOutput>,
}

/// Render the health screen.
///
/// # Arguments
///
/// * `f` - The frame to render to
/// * `area` - The area to render within
/// * `config` - Configuration for rendering
pub fn render_health(f: &mut Frame, area: Rect, config: HealthRenderConfig) {
    let HealthRenderConfig {
        loading,
        health_info,
    } = config;

    if loading && health_info.is_none() {
        let loading_widget = Paragraph::new("Loading health info...")
            .block(Block::default().borders(Borders::ALL).title("Health Check"))
            .alignment(Alignment::Center);
        f.render_widget(loading_widget, area);
        return;
    }

    let info = match health_info {
        Some(i) => i,
        None => {
            let placeholder = Paragraph::new("No health info loaded. Press 'r' to refresh.")
                .block(Block::default().borders(Borders::ALL).title("Health Check"))
                .alignment(Alignment::Center);
            f.render_widget(placeholder, area);
            return;
        }
    };

    let text = build_health_text(info);
    let health_widget = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("Health Check"))
        .alignment(Alignment::Left);
    f.render_widget(health_widget, area);
}

/// Build text content from health check output.
fn build_health_text(health: &HealthCheckOutput) -> String {
    let mut text = String::new();

    if let Some(server_info) = &health.server_info {
        push_section_text(&mut text, "Server Information");
        text.push_str(&format!("Name: {}\n", server_info.server_name));
        text.push_str(&format!("Version: {}\n", server_info.version));
        text.push_str(&format!("Build: {}\n", server_info.build));
        text.push_str(&format!(
            "OS: {}\n",
            server_info.os_name.as_deref().unwrap_or("unknown")
        ));
        text.push_str(&format!(
            "Mode: {}\n",
            server_info.mode.as_deref().unwrap_or("unknown")
        ));
    }

    if let Some(splunkd_health) = &health.splunkd_health {
        push_section_text(&mut text, "Splunkd Health");
        text.push_str(&format!("Overall: {}\n", splunkd_health.health));
        for (feature_name, feature) in &splunkd_health.features {
            text.push_str(&format!(
                "  {}: {} ({})\n",
                feature_name, feature.health, feature.status
            ));
        }
    }

    if let Some(license_usage) = &health.license_usage {
        push_section_text(&mut text, "License Usage");
        for (i, usage) in license_usage.iter().enumerate() {
            let percentage = if usage.quota > 0 {
                (usage.used_bytes as f64 / usage.quota as f64) * 100.0
            } else {
                0.0
            };
            text.push_str(&format!(
                "Pool {}: {}\n",
                i + 1,
                percentage_text(percentage)
            ));
            text.push_str(&format!("  Used: {}\n", format_bytes(usage.used_bytes)));
            text.push_str(&format!("  Quota: {}\n", format_bytes(usage.quota)));
        }
    }

    if let Some(kvstore_status) = &health.kvstore_status {
        push_section_text(&mut text, "KVStore Status");
        text.push_str(&format!(
            "Status: {}\n",
            kvstore_status.current_member.status
        ));
        text.push_str(&format!(
            "Host: {}:{}\n",
            kvstore_status.current_member.host, kvstore_status.current_member.port
        ));
        text.push_str(&format!(
            "Replica Set: {}\n",
            kvstore_status.current_member.replica_set
        ));
        text.push_str(&format!(
            "Oplog Used: {:.2} / {} MB\n",
            kvstore_status.replication_status.oplog_used,
            kvstore_status.replication_status.oplog_size
        ));
    }

    if let Some(log_parsing) = &health.log_parsing_health {
        push_section_text(&mut text, "Log Parsing Health");
        text.push_str(&format!(
            "Status: {}\n",
            if log_parsing.is_healthy {
                "Healthy"
            } else {
                "Unhealthy"
            }
        ));
        text.push_str(&format!("Total Errors: {}\n", log_parsing.total_errors));
        text.push_str(&format!("Time Window: {}\n", log_parsing.time_window));
        if !log_parsing.errors.is_empty() {
            text.push_str("Recent Errors:\n");
            for error in log_parsing.errors.iter().take(5) {
                text.push_str(&format!("  • {}: {}\n", error.source, error.message));
            }
            if log_parsing.errors.len() > 5 {
                text.push_str(&format!(
                    "  ... and {} more\n",
                    log_parsing.errors.len() - 5
                ));
            }
        }
    }

    if text.is_empty() {
        text = "No health data available.".to_string();
    }

    text
}

/// Add a section header to the text.
fn push_section_text(text: &mut String, title: &str) {
    if !text.is_empty() {
        text.push('\n');
    }
    text.push_str(title);
    text.push('\n');
}

/// Format license usage percentage with color indication.
fn percentage_text(percentage: f64) -> String {
    let color = if percentage < 70.0 {
        "green"
    } else if percentage < 90.0 {
        "yellow"
    } else {
        "red"
    };
    format!("{:.1}% ({})", percentage, color)
}

/// Format byte count with appropriate units.
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use splunk_client::models::{LicenseUsage, LogParsingError, LogParsingHealth, ServerInfo};

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
        assert_eq!(format_bytes(1024 * 1024 * 1024 * 1024), "1.00 TB");
    }

    #[test]
    fn test_percentage_text() {
        assert!(percentage_text(50.0).contains("50.0%"));
        assert!(percentage_text(50.0).contains("green"));
        assert!(percentage_text(80.0).contains("yellow"));
        assert!(percentage_text(95.0).contains("red"));
    }

    #[test]
    fn test_build_health_text_empty() {
        let health = HealthCheckOutput {
            server_info: None,
            splunkd_health: None,
            license_usage: None,
            kvstore_status: None,
            log_parsing_health: None,
        };
        let text = build_health_text(&health);
        assert_eq!(text, "No health data available.");
    }

    #[test]
    fn test_build_health_text_with_server_info() {
        let health = HealthCheckOutput {
            server_info: Some(ServerInfo {
                server_name: "splunk01".to_string(),
                version: "9.0.0".to_string(),
                build: "abc123".to_string(),
                mode: Some("standalone".to_string()),
                server_roles: vec![],
                os_name: Some("Linux".to_string()),
            }),
            splunkd_health: None,
            license_usage: None,
            kvstore_status: None,
            log_parsing_health: None,
        };
        let text = build_health_text(&health);
        assert!(text.contains("Server Information"));
        assert!(text.contains("Name: splunk01"));
        assert!(text.contains("Version: 9.0.0"));
    }

    #[test]
    fn test_build_health_text_with_license() {
        let health = HealthCheckOutput {
            server_info: None,
            splunkd_health: None,
            license_usage: Some(vec![LicenseUsage {
                name: "test_license".to_string(),
                quota: 1024 * 1024 * 1024,     // 1 GB
                used_bytes: 512 * 1024 * 1024, // 512 MB
                slaves_usage_bytes: None,
                stack_id: None,
            }]),
            kvstore_status: None,
            log_parsing_health: None,
        };
        let text = build_health_text(&health);
        assert!(text.contains("License Usage"));
        assert!(text.contains("50.0%"));
    }

    #[test]
    fn test_build_health_text_with_log_parsing() {
        let health = HealthCheckOutput {
            server_info: None,
            splunkd_health: None,
            license_usage: None,
            kvstore_status: None,
            log_parsing_health: Some(LogParsingHealth {
                is_healthy: false,
                total_errors: 3,
                errors: vec![
                    LogParsingError {
                        time: "2025-01-20T10:00:00".to_string(),
                        source: "/var/log/splunk/metrics.log".to_string(),
                        sourcetype: "splunkd".to_string(),
                        message: "Failed to parse timestamp".to_string(),
                        log_level: "ERROR".to_string(),
                        component: "DateParser".to_string(),
                    },
                    LogParsingError {
                        time: "2025-01-20T10:01:00".to_string(),
                        source: "/var/log/splunk/metrics.log".to_string(),
                        sourcetype: "splunkd".to_string(),
                        message: "Invalid timestamp format".to_string(),
                        log_level: "ERROR".to_string(),
                        component: "DateParser".to_string(),
                    },
                    LogParsingError {
                        time: "2025-01-20T10:02:00".to_string(),
                        source: "/var/log/splunk/metrics.log".to_string(),
                        sourcetype: "splunkd".to_string(),
                        message: "Timestamp out of range".to_string(),
                        log_level: "ERROR".to_string(),
                        component: "DateParser".to_string(),
                    },
                ],
                time_window: "-24h".to_string(),
            }),
        };
        let text = build_health_text(&health);
        assert!(text.contains("Log Parsing Health"));
        assert!(text.contains("Total Errors: 3"));
        assert!(text.contains("Recent Errors:"));
        assert!(text.contains("Failed to parse timestamp"));
        assert!(text.contains("Invalid timestamp format"));
        assert!(text.contains("Timestamp out of range"));
    }
}
