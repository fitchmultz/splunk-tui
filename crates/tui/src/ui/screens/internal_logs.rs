//! Internal logs screen.

use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Borders, Cell, Row, Table, TableState},
};
use splunk_client::models::{LogEntry, LogLevel};
use splunk_config::Theme;

use crate::ui::theme::ThemeExt;
use crate::ui::widgets::{render_empty_state, render_loading_state};

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
        .border_style(theme.border())
        .title_style(theme.title());

    if config.loading && config.logs.is_none() {
        render_loading_state(
            f,
            area,
            title,
            "Loading internal logs...",
            config.spinner_frame,
            theme,
        );
        return;
    }

    let logs = match config.logs {
        Some(l) => l,
        None => {
            render_empty_state(f, area, title, "logs");
            return;
        }
    };

    let header_cells = ["Time", "Level", "Component", "Message"]
        .iter()
        .map(|h| Cell::from(*h).style(theme.table_header()));
    let header = Row::new(header_cells).height(1);

    let rows = logs.iter().map(|log| {
        let level_style = match log.level {
            LogLevel::Error | LogLevel::Fatal => theme.error(),
            LogLevel::Warn => theme.warning(),
            LogLevel::Info => theme.info(),
            LogLevel::Debug => theme.text_dim(),
            LogLevel::Unknown => theme.text(),
        };

        let cells = vec![
            Cell::from(log.time.as_str()),
            Cell::from(log.level.to_string()).style(level_style),
            Cell::from(log.component.as_str()).style(theme.text_dim()),
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
    .row_highlight_style(theme.highlight())
    .highlight_symbol(">> ");

    f.render_stateful_widget(table, area, config.state);
}
