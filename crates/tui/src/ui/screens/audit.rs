//! Audit events screen rendering.
//!
//! Renders the list of Splunk audit events with filtering capabilities.

use crate::ui::theme::ThemeExt;
use crate::ui::widgets::{render_empty_state, render_loading_state};
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
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
    /// Current spinner frame for loading animation
    pub spinner_frame: u8,
}

/// Render the audit events screen.
pub fn render_audit(f: &mut Frame, area: Rect, config: AuditRenderConfig) {
    let theme = config.theme;

    let block = Block::default()
        .borders(Borders::ALL)
        .title("Audit Events")
        .border_style(theme.border())
        .title_style(theme.title());

    if config.loading && config.events.is_none() {
        render_loading_state(
            f,
            area,
            "Audit Events",
            "Loading audit events...",
            config.spinner_frame,
            theme,
        );
        return;
    }

    let events = match config.events {
        Some(e) => e,
        None => {
            render_empty_state(f, area, "Audit Events", "audit events");
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
        .map(|h| Cell::from(*h).style(theme.table_header()));
    let header = Row::new(header_cells).height(1);

    let rows = events.iter().map(|event| {
        let result_style = match event.result.to_string().as_str() {
            "success" => theme.success(),
            "failure" | "error" => theme.error(),
            _ => theme.text(),
        };

        let cells = vec![
            Cell::from(event.time.as_str()),
            Cell::from(event.user.as_str()),
            Cell::from(event.action.to_string()),
            Cell::from(event.target.as_str()),
            Cell::from(event.result.to_string()).style(result_style),
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
    .row_highlight_style(theme.highlight())
    .highlight_symbol(">> ");

    f.render_stateful_widget(table, area, config.state);
}
