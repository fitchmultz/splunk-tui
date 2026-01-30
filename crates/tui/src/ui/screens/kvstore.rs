//! KVStore screen rendering.
//!
//! Renders KVStore status information including current member details
//! and replication status.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};
use splunk_client::models::KvStoreStatus;
use splunk_config::Theme;

/// Configuration for rendering the KVStore screen.
pub struct KvstoreRenderConfig<'a> {
    /// Whether data is currently loading
    pub loading: bool,
    /// The KVStore status to display
    pub kvstore_status: Option<&'a KvStoreStatus>,
    /// Theme for consistent styling.
    pub theme: &'a Theme,
}

/// Render the KVStore screen.
///
/// # Arguments
///
/// * `f` - The frame to render to
/// * `area` - The area to render within
/// * `config` - Configuration for rendering
pub fn render_kvstore(f: &mut Frame, area: Rect, config: KvstoreRenderConfig) {
    let KvstoreRenderConfig {
        loading,
        kvstore_status,
        theme,
    } = config;

    if loading && kvstore_status.is_none() {
        let loading_widget = Paragraph::new("Loading KVStore status...")
            .block(Block::default().borders(Borders::ALL).title("KVStore"))
            .alignment(Alignment::Center);
        f.render_widget(loading_widget, area);
        return;
    }

    let status = match kvstore_status {
        Some(s) => s,
        None => {
            let placeholder = Paragraph::new("No KVStore status loaded. Press 'r' to refresh.")
                .block(Block::default().borders(Borders::ALL).title("KVStore"))
                .alignment(Alignment::Center);
            f.render_widget(placeholder, area);
            return;
        }
    };

    // Create layout with two sections: member info and replication status
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50), // Member info section
            Constraint::Percentage(50), // Replication status section
        ])
        .split(area);

    render_member_section(f, chunks[0], status, theme);
    render_replication_section(f, chunks[1], status, theme);
}

/// Render the current member information section.
fn render_member_section(f: &mut Frame, area: Rect, status: &KvStoreStatus, theme: &Theme) {
    let member = &status.current_member;

    let rows = vec![
        Row::new(vec![Cell::from("GUID"), Cell::from(member.guid.clone())]),
        Row::new(vec![Cell::from("Host"), Cell::from(member.host.clone())]),
        Row::new(vec![
            Cell::from("Port"),
            Cell::from(member.port.to_string()),
        ]),
        Row::new(vec![
            Cell::from("Replica Set"),
            Cell::from(member.replica_set.clone()),
        ]),
        Row::new(vec![
            Cell::from("Status"),
            Cell::from(member.status.clone())
                .style(Style::default().fg(status_color(&member.status, theme))),
        ]),
    ];

    let widths = [Constraint::Percentage(30), Constraint::Percentage(70)];

    let table = Table::new(rows, widths)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Current Member")
                .border_style(Style::default().fg(theme.border))
                .title_style(Style::default().fg(theme.title)),
        )
        .column_spacing(1);

    f.render_widget(table, area);
}

/// Render the replication status section.
fn render_replication_section(f: &mut Frame, area: Rect, status: &KvStoreStatus, theme: &Theme) {
    let replication = &status.replication_status;

    // Calculate oplog usage percentage
    let usage_pct = if replication.oplog_size > 0 {
        (replication.oplog_used / replication.oplog_size as f64) * 100.0
    } else {
        0.0
    };

    let usage_color = if usage_pct < 70.0 {
        theme.success
    } else if usage_pct < 90.0 {
        theme.warning
    } else {
        theme.error
    };

    let rows = vec![
        Row::new(vec![
            Cell::from("Oplog Size"),
            Cell::from(format_bytes(replication.oplog_size)),
        ]),
        Row::new(vec![
            Cell::from("Oplog Used"),
            Cell::from(format!("{:.2}%", usage_pct)).style(Style::default().fg(usage_color)),
        ]),
        Row::new(vec![
            Cell::from("Oplog Used (bytes)"),
            Cell::from(format_bytes(
                (replication.oplog_used * replication.oplog_size as f64 / 100.0) as u64,
            )),
        ]),
    ];

    let widths = [Constraint::Percentage(30), Constraint::Percentage(70)];

    let table = Table::new(rows, widths)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Replication Status")
                .border_style(Style::default().fg(theme.border))
                .title_style(Style::default().fg(theme.title)),
        )
        .column_spacing(1);

    f.render_widget(table, area);
}

/// Get color based on status string.
fn status_color(status: &str, theme: &Theme) -> ratatui::style::Color {
    match status.to_lowercase().as_str() {
        "running" | "ok" | "healthy" | "ready" => theme.success,
        "stopped" | "error" | "unhealthy" | "failed" => theme.error,
        "starting" | "stopping" | "degraded" => theme.warning,
        _ => theme.text,
    }
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
    use splunk_client::models::{KvStoreMember, KvStoreReplicationStatus};

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn test_status_color() {
        let theme = Theme::default();

        assert_eq!(status_color("running", &theme), theme.success);
        assert_eq!(status_color("stopped", &theme), theme.error);
        assert_eq!(status_color("starting", &theme), theme.warning);
        assert_eq!(status_color("unknown", &theme), theme.text);
    }

    #[test]
    fn test_kvstore_render_config() {
        let theme = Theme::default();
        let config = KvstoreRenderConfig {
            loading: false,
            kvstore_status: None,
            theme: &theme,
        };

        assert!(!config.loading);
        assert!(config.kvstore_status.is_none());
    }

    #[test]
    fn test_kvstore_render_config_with_status() {
        let theme = Theme::default();
        let status = KvStoreStatus {
            current_member: KvStoreMember {
                guid: "test-guid".to_string(),
                host: "localhost".to_string(),
                port: 8191,
                replica_set: "rs0".to_string(),
                status: "running".to_string(),
            },
            replication_status: KvStoreReplicationStatus {
                oplog_size: 1024 * 1024 * 1024,
                oplog_used: 50.0,
            },
        };

        let config = KvstoreRenderConfig {
            loading: false,
            kvstore_status: Some(&status),
            theme: &theme,
        };

        assert!(!config.loading);
        assert!(config.kvstore_status.is_some());
        assert_eq!(
            config.kvstore_status.unwrap().current_member.guid,
            "test-guid"
        );
    }
}
