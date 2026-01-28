//! Cluster screen rendering.
//!
//! Renders the cluster information including ID, mode, replication factors,
//! and cluster peers list. Supports toggling between summary and peers views.
//!
//! Responsibilities:
//! - Render cluster summary information (ID, mode, label, replication factors)
//! - Render cluster peers as a table with status indicators
//! - Handle view mode switching (Summary vs Peers)
//!
//! Non-responsibilities:
//! - Does NOT fetch data (handled by async tasks in main.rs)
//! - Does NOT handle user input (handled by input module)

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, List, ListItem, Row, Table, TableState},
};
use splunk_client::models::{ClusterInfo, ClusterPeer};
use splunk_config::Theme;

use crate::app::state::ClusterViewMode;

/// Configuration for rendering the cluster screen.
pub struct ClusterRenderConfig<'a> {
    /// Whether data is currently loading
    pub loading: bool,
    /// The cluster information to display
    pub cluster_info: Option<&'a ClusterInfo>,
    /// The cluster peers to display
    pub cluster_peers: Option<&'a [ClusterPeer]>,
    /// Current view mode
    pub view_mode: ClusterViewMode,
    /// Table state for peers view
    pub peers_state: &'a mut TableState,
    /// Theme for consistent styling
    pub theme: &'a Theme,
}

/// Render the cluster screen.
///
/// # Arguments
///
/// * `f` - The frame to render to
/// * `area` - The area to render within
/// * `config` - Configuration for rendering
pub fn render_cluster(f: &mut Frame, area: Rect, config: ClusterRenderConfig) {
    let ClusterRenderConfig {
        loading,
        cluster_info,
        cluster_peers,
        view_mode,
        peers_state,
        theme,
    } = config;

    if loading && cluster_info.is_none() {
        let loading_widget = ratatui::widgets::Paragraph::new("Loading cluster info...")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Cluster Information"),
            )
            .alignment(Alignment::Center);
        f.render_widget(loading_widget, area);
        return;
    }

    let info = match cluster_info {
        Some(i) => i,
        None => {
            let placeholder =
                ratatui::widgets::Paragraph::new("No cluster info loaded. Press 'r' to refresh.")
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Cluster Information"),
                    )
                    .alignment(Alignment::Center);
            f.render_widget(placeholder, area);
            return;
        }
    };

    match view_mode {
        ClusterViewMode::Summary => {
            render_summary(f, area, info, theme);
        }
        ClusterViewMode::Peers => {
            render_peers(f, area, info, cluster_peers, peers_state, loading, theme);
        }
    }
}

/// Render the cluster summary view.
fn render_summary(f: &mut Frame, area: Rect, info: &ClusterInfo, theme: &Theme) {
    let items: Vec<ListItem> = vec![
        ListItem::new(format!("ID: {}", info.id)),
        ListItem::new(format!("Mode: {}", info.mode)),
        ListItem::new(format!("Label: {:?}", info.label)),
        ListItem::new(format!("Manager URI: {:?}", info.manager_uri)),
        ListItem::new(format!("Replication Factor: {:?}", info.replication_factor)),
        ListItem::new(format!("Search Factor: {:?}", info.search_factor)),
        ListItem::new(format!("Status: {:?}", info.status)),
    ];

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title("Cluster Information (Summary) - Press 'p' for peers")
            .border_style(Style::default().fg(theme.border))
            .title_style(Style::default().fg(theme.title)),
    );
    f.render_widget(list, area);
}

/// Render the cluster peers view.
fn render_peers(
    f: &mut Frame,
    area: Rect,
    _info: &ClusterInfo,
    peers: Option<&[ClusterPeer]>,
    state: &mut TableState,
    loading: bool,
    theme: &Theme,
) {
    let title = if loading {
        "Cluster Peers (Loading...)"
    } else {
        "Cluster Peers - Press 'p' for summary"
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(theme.border))
        .title_style(Style::default().fg(theme.title));

    let peers = match peers {
        Some(p) => p,
        None => {
            // Show loading or empty state
            let paragraph = ratatui::widgets::Paragraph::new(if loading {
                "Loading peers..."
            } else {
                "No peers loaded. Press 'r' to refresh."
            })
            .block(block)
            .alignment(Alignment::Center);
            f.render_widget(paragraph, area);
            return;
        }
    };

    if peers.is_empty() {
        let paragraph = ratatui::widgets::Paragraph::new("No cluster peers found.")
            .block(block)
            .alignment(Alignment::Center);
        f.render_widget(paragraph, area);
        return;
    }

    // Define table headers
    let headers = [
        "Host",
        "Status",
        "State",
        "Site",
        "Port",
        "Rep Count",
        "Rep Status",
    ];

    // Create header row with styling
    let header_cells: Vec<Cell> = headers
        .iter()
        .map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(theme.table_header_fg)
                    .bg(theme.table_header_bg)
                    .add_modifier(Modifier::BOLD),
            )
        })
        .collect();
    let header = Row::new(header_cells).height(1);

    // Create rows for each peer
    let rows: Vec<Row> = peers
        .iter()
        .map(|peer| {
            let host_text = if peer.is_captain == Some(true) {
                format!("{} [C]", peer.host)
            } else {
                peer.host.clone()
            };

            let status_style = peer_status_style(&peer.status, theme);

            let cells = vec![
                Cell::from(host_text),
                Cell::from(peer.status.clone()).style(status_style),
                Cell::from(peer.peer_state.clone()),
                Cell::from(peer.site.clone().unwrap_or_default()),
                Cell::from(peer.port.to_string()),
                Cell::from(
                    peer.replication_count
                        .map(|c| c.to_string())
                        .unwrap_or_default(),
                ),
                Cell::from(peer.replication_status.clone().unwrap_or_default()),
            ];
            Row::new(cells).height(1)
        })
        .collect();

    // Column constraints
    let constraints = [
        Constraint::Min(20),    // Host (with captain indicator)
        Constraint::Length(12), // Status
        Constraint::Length(15), // State
        Constraint::Length(10), // Site
        Constraint::Length(6),  // Port
        Constraint::Length(10), // Rep Count
        Constraint::Length(12), // Rep Status
    ];

    let table = Table::new(rows, constraints)
        .header(header)
        .block(block)
        .row_highlight_style(Style::default().bg(theme.highlight_bg));

    f.render_stateful_widget(table, area, state);
}

/// Get the style for a peer status.
fn peer_status_style(status: &str, theme: &Theme) -> Style {
    match status.to_lowercase().as_str() {
        "up" => Style::default().fg(theme.success),
        "down" => Style::default().fg(theme.error),
        "pending" => Style::default().fg(theme.warning),
        _ => Style::default().fg(theme.text),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_status_style_up() {
        let theme = Theme::default();
        let style = peer_status_style("Up", &theme);
        // Should return success color
        assert_eq!(style.fg, Some(theme.success));
    }

    #[test]
    fn test_peer_status_style_down() {
        let theme = Theme::default();
        let style = peer_status_style("Down", &theme);
        // Should return error color
        assert_eq!(style.fg, Some(theme.error));
    }

    #[test]
    fn test_peer_status_style_pending() {
        let theme = Theme::default();
        let style = peer_status_style("Pending", &theme);
        // Should return warning color
        assert_eq!(style.fg, Some(theme.warning));
    }

    #[test]
    fn test_peer_status_style_unknown() {
        let theme = Theme::default();
        let style = peer_status_style("Unknown", &theme);
        // Should return default text color
        assert_eq!(style.fg, Some(theme.text));
    }
}
