//! Inputs screen rendering.
//!
//! Renders the list of Splunk data inputs with their types and status.

use crate::ui::theme::spinner_char;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
};
use splunk_client::models::Input;
use splunk_config::Theme;

/// Configuration for rendering the inputs screen.
pub struct InputsRenderConfig<'a> {
    /// Whether data is currently loading
    pub loading: bool,
    /// The list of inputs to display
    pub inputs: Option<&'a [Input]>,
    /// The current table selection state
    pub state: &'a mut TableState,
    /// Theme for consistent styling.
    pub theme: &'a Theme,
    /// Current spinner frame for loading animation
    pub spinner_frame: u8,
}

/// Render the inputs screen.
///
/// # Arguments
///
/// * `f` - The frame to render to
/// * `area` - The area to render within
/// * `config` - Configuration for rendering
pub fn render_inputs(f: &mut Frame, area: Rect, config: InputsRenderConfig) {
    let InputsRenderConfig {
        loading,
        inputs,
        state,
        theme,
        spinner_frame,
    } = config;

    if loading && inputs.is_none() {
        let spinner = spinner_char(spinner_frame);
        let loading_widget =
            ratatui::widgets::Paragraph::new(format!("{} Loading inputs...", spinner))
                .block(Block::default().borders(Borders::ALL).title("Data Inputs"))
                .alignment(Alignment::Center);
        f.render_widget(loading_widget, area);
        return;
    }

    let inputs = match inputs {
        Some(i) => i,
        None => {
            let placeholder =
                ratatui::widgets::Paragraph::new("No inputs loaded. Press 'r' to refresh.")
                    .block(Block::default().borders(Borders::ALL).title("Data Inputs"))
                    .alignment(Alignment::Center);
            f.render_widget(placeholder, area);
            return;
        }
    };

    if inputs.is_empty() {
        let placeholder = ratatui::widgets::Paragraph::new("No inputs found.")
            .block(Block::default().borders(Borders::ALL).title("Data Inputs"))
            .alignment(Alignment::Center);
        f.render_widget(placeholder, area);
        return;
    }

    // Header
    let header = Row::new(vec![
        Cell::from("Name").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Type").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Host").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Source").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Sourcetype").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Status").style(Style::default().add_modifier(Modifier::BOLD)),
    ]);

    // Rows
    let rows: Vec<Row> = inputs
        .iter()
        .map(|input| {
            let status = if input.disabled {
                Span::styled("Disabled", Style::default().fg(theme.error))
            } else {
                Span::styled("Enabled", Style::default().fg(theme.success))
            };

            Row::new(vec![
                Cell::from(input.name.as_str()),
                Cell::from(input.input_type.as_str()),
                Cell::from(input.host.as_deref().unwrap_or("-")),
                Cell::from(input.source.as_deref().unwrap_or("-")),
                Cell::from(input.sourcetype.as_deref().unwrap_or("-")),
                Cell::from(Line::from(vec![status])),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            ratatui::layout::Constraint::Percentage(20),
            ratatui::layout::Constraint::Percentage(15),
            ratatui::layout::Constraint::Percentage(15),
            ratatui::layout::Constraint::Percentage(20),
            ratatui::layout::Constraint::Percentage(20),
            ratatui::layout::Constraint::Percentage(10),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Data Inputs")
            .border_style(Style::default().fg(theme.border))
            .title_style(Style::default().fg(theme.title)),
    )
    .row_highlight_style(
        Style::default()
            .fg(theme.highlight_fg)
            .bg(theme.highlight_bg)
            .add_modifier(Modifier::BOLD),
    );

    f.render_stateful_widget(table, area, state);
}
