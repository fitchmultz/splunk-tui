//! Inputs screen rendering.
//!
//! Renders the list of Splunk data inputs with their types and status.

use crate::theme::Theme;
use crate::ui::theme::ThemeExt;
use crate::ui::widgets::render_screen_state;
use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Row, Table, TableState},
};
use splunk_client::models::Input;

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

    let Some(inputs) = render_screen_state(
        f,
        area,
        loading,
        inputs,
        "Data Inputs",
        "Loading inputs...",
        "inputs",
        spinner_frame,
        theme,
    ) else {
        return;
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
        Cell::from("Name").style(theme.table_header()),
        Cell::from("Type").style(theme.table_header()),
        Cell::from("Host").style(theme.table_header()),
        Cell::from("Source").style(theme.table_header()),
        Cell::from("Sourcetype").style(theme.table_header()),
        Cell::from("Status").style(theme.table_header()),
    ]);

    // Rows
    let rows: Vec<Row> = inputs
        .iter()
        .map(|input| {
            let status = if input.disabled {
                Span::styled("Disabled", theme.error())
            } else {
                Span::styled("Enabled", theme.success())
            };

            Row::new(vec![
                Cell::from(input.name.as_str()),
                Cell::from(input.input_type.to_string()),
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
            .border_style(theme.border())
            .title_style(theme.title()),
    )
    .row_highlight_style(theme.highlight());

    f.render_stateful_widget(table, area, state);
}
