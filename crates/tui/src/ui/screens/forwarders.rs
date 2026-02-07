//! Forwarders screen for the TUI.
//!
//! Responsibilities:
//! - Render the forwarders table with status indicators
//! - Display loading state and empty state
//!
//! Does NOT handle:
//! - Data fetching (handled by side effects)
//! - User input (handled by input handlers)

use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
};
use splunk_client::models::Forwarder;

use splunk_config::Theme;

use crate::ui::theme::spinner_char;

/// Configuration for rendering the forwarders screen.
pub struct ForwardersRenderConfig<'a> {
    /// Whether data is currently loading.
    pub loading: bool,
    /// The list of forwarders to display.
    pub forwarders: Option<&'a [Forwarder]>,
    /// The table state for selection.
    pub forwarders_state: &'a mut TableState,
    /// The theme to use for styling.
    pub theme: &'a Theme,
    /// Current spinner frame for loading animation
    pub spinner_frame: u8,
}

/// Render the forwarders screen.
pub fn render_forwarders(f: &mut Frame, area: Rect, config: ForwardersRenderConfig) {
    let ForwardersRenderConfig {
        loading,
        forwarders,
        forwarders_state,
        theme,
        spinner_frame,
    } = config;

    if loading && forwarders.is_none() {
        let spinner = spinner_char(spinner_frame);
        let loading_widget =
            ratatui::widgets::Paragraph::new(format!("{} Loading forwarders...", spinner))
                .block(Block::default().borders(Borders::ALL).title("Forwarders"))
                .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(loading_widget, area);
        return;
    }

    let forwarders = match forwarders {
        Some(f) => f,
        None => {
            let placeholder =
                ratatui::widgets::Paragraph::new("No forwarders loaded. Press 'r' to refresh.")
                    .block(Block::default().borders(Borders::ALL).title("Forwarders"))
                    .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(placeholder, area);
            return;
        }
    };

    if forwarders.is_empty() {
        let empty = ratatui::widgets::Paragraph::new("No forwarders found.")
            .block(Block::default().borders(Borders::ALL).title("Forwarders"))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(empty, area);
        return;
    }

    // Build table rows
    let header_cells = [
        "Name",
        "Hostname",
        "IP Address",
        "Version",
        "Last Phone Home",
    ]
    .iter()
    .map(|h| {
        Cell::from(*h).style(
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
    });
    let header = Row::new(header_cells).height(1);

    let rows: Vec<Row> = forwarders
        .iter()
        .map(|forwarder| {
            let cells = vec![
                Cell::from(forwarder.name.clone()),
                Cell::from(forwarder.hostname.clone().unwrap_or_default()),
                Cell::from(forwarder.ip_address.clone().unwrap_or_default()),
                Cell::from(forwarder.version.clone().unwrap_or_default()),
                Cell::from(forwarder.last_phone.clone().unwrap_or_default()),
            ];
            Row::new(cells).height(1)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(20),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
        ],
    )
    .header(header)
    .block(Block::default().borders(Borders::ALL).title("Forwarders"))
    .row_highlight_style(Style::default().bg(theme.accent).fg(theme.background))
    .highlight_symbol("> ");

    f.render_stateful_widget(table, area, forwarders_state);
}
