//! Audit events screen rendering.
//!
//! Renders the list of Splunk audit events with filtering capabilities.

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
};
use splunk_client::models::AuditEvent;
use splunk_config::Theme;

/// Configuration for rendering the audit events screen.
pub struct AuditRenderConfig<'a> {
    /// Whether data is currently loading
    pub loading: bool,
    /// The list of audit events to display
    pub events: Option<&'a [AuditEvent]>,
    /// The current table selection state
    pub state: &'a mut TableState,
    /// Theme for consistent styling
    pub theme: &'a Theme,
}

/// Render the audit events screen.
pub fn render_audit(f: &mut Frame, area: Rect, config: AuditRenderConfig) {
    let theme = config.theme;

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Audit Events")
        .border_style(Style::default().fg(theme.border))
        .title_style(Style::default().fg(theme.title));

    if config.loading && config.events.is_none() {
        let loading = ratatui::widgets::Paragraph::new("Loading audit events...")
            .block(block)
            .alignment(Alignment::Center);
        f.render_widget(loading, area);
        return;
    }

    let events = match config.events {
        Some(e) => e,
        None => {
            let placeholder =
                ratatui::widgets::Paragraph::new("No audit events loaded. Press 'r' to refresh.")
                    .block(block)
                    .alignment(Alignment::Center);
            f.render_widget(placeholder, area);
            return;
        }
    };

    if events.is_empty() {
        let placeholder =
            ratatui::widgets::Paragraph::new("No audit events found for the specified time range.")
                .block(block)
                .alignment(Alignment::Center);
        f.render_widget(placeholder, area);
        return;
    }

    let header_cells = ["Time", "User", "Action", "Target", "Result"]
        .iter()
        .map(|h| {
            Cell::from(*h).style(
                Style::default()
                    .fg(theme.table_header_fg)
                    .add_modifier(Modifier::BOLD),
            )
        });
    let header = Row::new(header_cells)
        .style(Style::default().bg(theme.table_header_bg))
        .height(1);

    let rows = events.iter().map(|event| {
        let result_color = match event.result.to_lowercase().as_str() {
            "success" => theme.log_info,
            "failure" | "error" => theme.log_error,
            _ => theme.text,
        };

        let cells = vec![
            Cell::from(event.time.as_str()),
            Cell::from(event.user.as_str()),
            Cell::from(event.action.as_str()),
            Cell::from(event.target.as_str()),
            Cell::from(event.result.as_str()).style(Style::default().fg(result_color)),
        ];
        Row::new(cells)
    });

    let table = Table::new(
        rows,
        [
            ratatui::layout::Constraint::Length(25), // Time
            ratatui::layout::Constraint::Length(15), // User
            ratatui::layout::Constraint::Length(20), // Action
            ratatui::layout::Constraint::Length(20), // Target
            ratatui::layout::Constraint::Length(10), // Result
        ],
    )
    .header(header)
    .block(block)
    .row_highlight_style(
        Style::default()
            .fg(theme.highlight_fg)
            .bg(theme.highlight_bg)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol(">> ");

    f.render_stateful_widget(table, area, config.state);
}
