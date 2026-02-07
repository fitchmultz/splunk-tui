//! Health screen rendering.
//!
//! Renders comprehensive Splunk environment health metrics including server info,
//! splunkd health, license usage, KVStore status, and log parsing health.

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use splunk_client::models::HealthCheckOutput;
use splunk_config::Theme;

use crate::ui::theme::ThemeExt;
use crate::ui::widgets::{render_empty_state, render_loading_state};

/// Configuration for rendering the health screen.
pub struct HealthRenderConfig<'a> {
    /// Whether data is currently loading
    pub loading: bool,
    /// The health information to display
    pub health_info: Option<&'a HealthCheckOutput>,
    /// Theme for consistent styling.
    pub theme: &'a Theme,
    /// Current spinner frame for loading animation.
    pub spinner_frame: u8,
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
        theme,
        spinner_frame,
    } = config;

    if loading && health_info.is_none() {
        render_loading_state(
            f,
            area,
            "Health Check",
            "Loading health info...",
            spinner_frame,
            theme,
        );
        return;
    }

    let info = match health_info {
        Some(i) => i,
        None => {
            render_empty_state(f, area, "Health Check", "health info");
            return;
        }
    };

    let lines = build_health_lines(info, theme);
    let health_widget = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Health Check")
                .border_style(theme.border())
                .title_style(theme.title()),
        )
        .alignment(Alignment::Left);
    f.render_widget(health_widget, area);
}

/// Build text content from health check output.
fn build_health_lines(health: &HealthCheckOutput, theme: &Theme) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    if let Some(server_info) = &health.server_info {
        push_section_lines(&mut lines, "Server Information", theme);
        lines.push(Line::from(format!("Name: {}", server_info.server_name)));
        lines.push(Line::from(format!("Version: {}", server_info.version)));
        lines.push(Line::from(format!("Build: {}", server_info.build)));
        lines.push(Line::from(format!(
            "OS: {}",
            server_info.os_name.as_deref().unwrap_or("unknown")
        )));
        lines.push(Line::from(format!(
            "Mode: {}",
            server_info.mode.as_deref().unwrap_or("unknown")
        )));
    }

    if let Some(splunkd_health) = &health.splunkd_health {
        push_section_lines(&mut lines, "Splunkd Health", theme);

        let overall_color = match splunkd_health.health.to_lowercase().as_str() {
            "green" => theme.success,
            "red" => theme.error,
            _ => theme.warning,
        };

        lines.push(Line::from(vec![
            Span::raw("Overall: "),
            Span::styled(
                splunkd_health.health.clone(),
                Style::default()
                    .fg(overall_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        for (feature_name, feature) in &splunkd_health.features {
            lines.push(Line::from(format!(
                "  {}: {} ({})",
                feature_name, feature.health, feature.status
            )));
        }
    }

    if let Some(license_usage) = &health.license_usage {
        push_section_lines(&mut lines, "License Usage", theme);
        for (i, usage) in license_usage.iter().enumerate() {
            let used_bytes = usage.effective_used_bytes();
            let percentage = if usage.quota > 0 {
                (used_bytes as f64 / usage.quota as f64) * 100.0
            } else {
                0.0
            };

            let (pct_text, pct_color) = percentage_span(percentage, theme);
            lines.push(Line::from(vec![
                Span::raw(format!("Pool {}: ", i + 1)),
                Span::styled(pct_text, Style::default().fg(pct_color)),
            ]));
            lines.push(Line::from(format!("  Used: {}", format_bytes(used_bytes))));
            lines.push(Line::from(format!(
                "  Quota: {}",
                format_bytes(usage.quota)
            )));
        }
    }

    if let Some(kvstore_status) = &health.kvstore_status {
        push_section_lines(&mut lines, "KVStore Status", theme);
        lines.push(Line::from(format!(
            "Status: {}",
            kvstore_status.current_member.status
        )));
        lines.push(Line::from(format!(
            "Host: {}:{}",
            kvstore_status.current_member.host, kvstore_status.current_member.port
        )));
        lines.push(Line::from(format!(
            "Replica Set: {}",
            kvstore_status.current_member.replica_set
        )));
        lines.push(Line::from(format!(
            "Oplog Used: {:.2} / {} MB",
            kvstore_status.replication_status.oplog_used,
            kvstore_status.replication_status.oplog_size
        )));
    }

    if let Some(log_parsing) = &health.log_parsing_health {
        push_section_lines(&mut lines, "Log Parsing Health", theme);

        let status_color = if log_parsing.is_healthy {
            theme.success
        } else {
            theme.error
        };

        lines.push(Line::from(vec![
            Span::raw("Status: "),
            Span::styled(
                if log_parsing.is_healthy {
                    "Healthy".to_string()
                } else {
                    "Unhealthy".to_string()
                },
                Style::default()
                    .fg(status_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        lines.push(Line::from(format!(
            "Total Errors: {}",
            log_parsing.total_errors
        )));
        lines.push(Line::from(format!(
            "Time Window: {}",
            log_parsing.time_window
        )));

        if !log_parsing.errors.is_empty() {
            lines.push(Line::from(Span::styled("Recent Errors:", theme.title())));
            for err in log_parsing.errors.iter().take(5) {
                lines.push(Line::from(format!("  • {}: {}", err.source, err.message)));
            }
            if log_parsing.errors.len() > 5 {
                lines.push(Line::from(format!(
                    "  ... and {} more",
                    log_parsing.errors.len() - 5
                )));
            }
        }
    }

    if lines.is_empty() {
        lines.push(Line::from("No health data available."));
    }

    lines
}

/// Add a section header to the text.
fn push_section_lines(lines: &mut Vec<Line<'static>>, title: &str, theme: &Theme) {
    if !lines.is_empty() {
        lines.push(Line::from(""));
    }
    lines.push(Line::from(Span::styled(title.to_string(), theme.title())));
}

/// Format license usage percentage and choose a semantic color.
fn percentage_span(percentage: f64, theme: &Theme) -> (String, ratatui::style::Color) {
    let color = if percentage < 70.0 {
        theme.success
    } else if percentage < 90.0 {
        theme.warning
    } else {
        theme.error
    };
    (format!("{:.1}%", percentage), color)
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

    fn flatten_lines(lines: Vec<Line>) -> String {
        lines
            .into_iter()
            .map(|l| {
                l.spans
                    .into_iter()
                    .map(|s| s.content.to_string())
                    .collect::<String>()
            })
            .collect::<Vec<String>>()
            .join("\n")
    }

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
    fn test_build_health_text_empty() {
        let health = HealthCheckOutput {
            server_info: None,
            splunkd_health: None,
            license_usage: None,
            kvstore_status: None,
            log_parsing_health: None,
        };
        let lines = build_health_lines(&health, &Theme::default());
        assert_eq!(lines.len(), 1);
        assert!(
            lines[0].spans[0]
                .content
                .contains("No health data available.")
        );
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
        let lines = build_health_lines(&health, &Theme::default());
        let text = flatten_lines(lines);
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
                quota: 1024 * 1024 * 1024,           // 1 GB
                used_bytes: Some(512 * 1024 * 1024), // 512 MB
                slaves_usage_bytes: None,
                stack_id: None,
            }]),
            kvstore_status: None,
            log_parsing_health: None,
        };
        let lines = build_health_lines(&health, &Theme::default());
        let text = flatten_lines(lines);
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
        let lines = build_health_lines(&health, &Theme::default());
        let text = flatten_lines(lines);
        assert!(text.contains("Log Parsing Health"));
        assert!(text.contains("Total Errors: 3"));
        assert!(text.contains("Recent Errors:"));
        assert!(text.contains("Failed to parse timestamp"));
        assert!(text.contains("Invalid timestamp format"));
        assert!(text.contains("Timestamp out of range"));
    }
}
