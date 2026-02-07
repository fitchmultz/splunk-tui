//! Search peers screen for the TUI.
//!
//! Responsibilities:
//! - Render the search peers table with status indicators
//! - Display loading state and empty state
//!
//! Does NOT handle:
//! - Data fetching (handled by side effects)
//! - User input (handled by input handlers)

use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
};
use splunk_client::models::SearchPeer;

use splunk_config::Theme;

use crate::ui::theme::ThemeExt;
use crate::ui::widgets::{render_empty_state, render_loading_state};

/// Configuration for rendering the search peers screen.
pub struct SearchPeersRenderConfig<'a> {
    /// Whether data is currently loading.
    pub loading: bool,
    /// The list of search peers to display.
    pub search_peers: Option<&'a [SearchPeer]>,
    /// The table state for selection.
    pub peers_state: &'a mut TableState,
    /// The theme to use for styling.
    pub theme: &'a Theme,
    /// Current spinner frame for loading animation
    pub spinner_frame: u8,
}

/// Render the search peers screen.
pub fn render_search_peers(f: &mut Frame, area: Rect, config: SearchPeersRenderConfig) {
    let SearchPeersRenderConfig {
        loading,
        search_peers,
        peers_state,
        theme,
        spinner_frame,
    } = config;

    if loading && search_peers.is_none() {
        render_loading_state(
            f,
            area,
            "Search Peers",
            "Loading search peers...",
            spinner_frame,
            theme,
        );
        return;
    }

    let peers = match search_peers {
        Some(p) => p,
        None => {
            render_empty_state(f, area, "Search Peers", "search peers");
            return;
        }
    };

    if peers.is_empty() {
        let empty = ratatui::widgets::Paragraph::new("No search peers found.")
            .block(Block::default().borders(Borders::ALL).title("Search Peers"))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(empty, area);
        return;
    }

    // Build table rows
    let header_cells = ["Name", "Host", "Port", "Status", "Version"]
        .iter()
        .map(|h| Cell::from(*h).style(theme.table_header()));
    let header = Row::new(header_cells).height(1);

    let rows: Vec<Row> = peers
        .iter()
        .map(|peer| {
            let status_style = match peer.status.as_str() {
                "Up" => theme.success(),
                "Down" => theme.error(),
                _ => theme.warning(),
            };

            let cells = vec![
                Cell::from(peer.name.clone()),
                Cell::from(peer.host.clone()),
                Cell::from(peer.port.to_string()),
                Cell::from(peer.status.clone()).style(status_style),
                Cell::from(peer.version.as_deref().unwrap_or("N/A")),
            ];
            Row::new(cells).height(1)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(20),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title("Search Peers"))
    .row_highlight_style(theme.highlight())
    .highlight_symbol("> ");

    f.render_stateful_widget(table, area, peers_state);
}
