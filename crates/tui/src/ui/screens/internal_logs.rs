//! Internal logs screen.

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
};
use splunk_client::models::LogEntry;
use splunk_config::Theme;

use crate::ui::theme::spinner_char;

/// Configuration for rendering the internal logs screen.
pub struct InternalLogsRenderConfig<'a> {
    pub loading: bool,
    pub logs: Option<&'a [LogEntry]>,
    pub state: &'a mut TableState,
    pub auto_refresh: bool,
    pub theme: &'a Theme,
    /// Current spinner frame for loading animation
    pub spinner_frame: u8,
}

/// Render the internal logs screen.
pub fn render_internal_logs(f: &mut Frame, area: Rect, config: InternalLogsRenderConfig) {
    let theme = config.theme;

    let title = if config.auto_refresh {
        "Internal Logs (_internal) [AUTO]"
    } else {
        "Internal Logs (_internal)"
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(Style::default().fg(theme.border))
        .title_style(Style::default().fg(theme.title));

    if config.loading && config.logs.is_none() {
        let spinner = spinner_char(config.spinner_frame);
        let loading =
            ratatui::widgets::Paragraph::new(format!("{} Loading internal logs...", spinner))
                .block(block)
                .alignment(Alignment::Center);
        f.render_widget(loading, area);
        return;
    }

    let logs = match config.logs {
        Some(l) => l,
        None => {
            let placeholder =
                ratatui::widgets::Paragraph::new("No logs loaded. Press 'r' to refresh.")
                    .block(block)
                    .alignment(Alignment::Center);
            f.render_widget(placeholder, area);
            return;
        }
    };

    let header_cells = ["Time", "Level", "Component", "Message"].iter().map(|h| {
        Cell::from(*h).style(
            Style::default()
                .fg(theme.table_header_fg)
                .add_modifier(Modifier::BOLD),
        )
    });
    let header = Row::new(header_cells)
        .style(Style::default().bg(theme.table_header_bg))
        .height(1);

    let rows = logs.iter().map(|log| {
        let level_color = match log.level.to_uppercase().as_str() {
            "ERROR" | "FATAL" => theme.log_error,
            "WARN" | "WARNING" => theme.log_warn,
            "INFO" => theme.log_info,
            "DEBUG" => theme.log_debug,
            _ => theme.text,
        };

        let cells = vec![
            Cell::from(log.time.as_str()),
            Cell::from(log.level.as_str()).style(Style::default().fg(level_color)),
            Cell::from(log.component.as_str()).style(Style::default().fg(theme.log_component)),
            Cell::from(log.message.as_str()),
        ];
        Row::new(cells)
    });

    let table = Table::new(
        rows,
        [
            ratatui::layout::Constraint::Length(25), // Time
            ratatui::layout::Constraint::Length(10), // Level
            ratatui::layout::Constraint::Length(20), // Component
            ratatui::layout::Constraint::Min(40),    // Message
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
