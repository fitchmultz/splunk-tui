//! KVStore screen rendering.
//!
//! Renders KVStore status information including current member details
//! and replication status.

use crate::ui::theme::ThemeExt;
use crate::ui::widgets::{render_empty_state, render_loading_state};
use crate::utils::format_bytes;
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Cell, Row, Table},
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
    /// Current spinner frame for loading animation
    pub spinner_frame: u8,
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
        spinner_frame,
    } = config;

    if loading && kvstore_status.is_none() {
        render_loading_state(
            f,
            area,
            "KVStore",
            "Loading KVStore status...",
            spinner_frame,
            theme,
        );
        return;
    }

    let status = match kvstore_status {
        Some(s) => s,
        None => {
            render_empty_state(f, area, "KVStore", "KVStore status");
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
            Cell::from(member.status.clone()).style(theme.status_style(&member.status)),
        ]),
    ];

    let widths = [Constraint::Percentage(30), Constraint::Percentage(70)];

    let table = Table::new(rows, widths)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Current Member")
                .border_style(theme.border())
                .title_style(theme.title()),
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

    let rows = vec![
        Row::new(vec![
            Cell::from("Oplog Size"),
            Cell::from(format_bytes(replication.oplog_size)),
        ]),
        Row::new(vec![
            Cell::from("Oplog Used"),
            Cell::from(format!("{:.2}%", usage_pct)).style(usage_style(usage_pct, theme)),
        ]),
        Row::new(vec![
            Cell::from("Oplog Used (bytes)"),
            Cell::from(format_bytes(
                (replication.oplog_used * replication.oplog_size as f64 / 100.0) as usize,
            )),
        ]),
    ];

    let widths = [Constraint::Percentage(30), Constraint::Percentage(70)];

    let table = Table::new(rows, widths)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Replication Status")
                .border_style(theme.border())
                .title_style(theme.title()),
        )
        .column_spacing(1);

    f.render_widget(table, area);
}

/// Get style based on usage percentage.
fn usage_style(usage_pct: f64, theme: &Theme) -> Style {
    if usage_pct < 70.0 {
        theme.success()
    } else if usage_pct < 90.0 {
        theme.warning()
    } else {
        theme.error()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use splunk_client::models::{KvStoreMember, KvStoreReplicationStatus};

    #[test]
    fn test_status_style() {
        let theme = Theme::default();

        assert_eq!(theme.status_style("running").fg, Some(theme.success));
        assert_eq!(theme.status_style("stopped").fg, Some(theme.error));
        assert_eq!(theme.status_style("starting").fg, Some(theme.warning));
        assert_eq!(theme.status_style("unknown").fg, Some(theme.text));
    }

    #[test]
    fn test_kvstore_render_config() {
        let theme = Theme::default();
        let config = KvstoreRenderConfig {
            loading: false,
            kvstore_status: None,
            theme: &theme,
            spinner_frame: 0,
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
            spinner_frame: 0,
        };

        assert!(!config.loading);
        assert!(config.kvstore_status.is_some());
        assert_eq!(
            config.kvstore_status.unwrap().current_member.guid,
            "test-guid"
        );
    }
}
