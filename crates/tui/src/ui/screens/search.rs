//! Search screen rendering.
//!
//! Renders the search input, status, and results for running Splunk searches.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Gauge, Paragraph},
};

use crate::ui::syntax::highlight_spl;

/// Configuration for rendering the search screen.
pub struct SearchRenderConfig<'a> {
    /// The current search input text
    pub search_input: &'a str,
    /// The current search status message
    pub search_status: &'a str,
    /// Whether a search is currently running
    pub loading: bool,
    /// Progress of the current search (0.0 to 1.0)
    pub progress: f32,
    /// The search results to display (pre-formatted JSON strings)
    pub search_results: &'a [String],
    /// The scroll offset for displaying results
    pub search_scroll_offset: usize,
}

/// Render the search screen.
///
/// # Arguments
///
/// * `f` - The frame to render to
/// * `area` - The area to render within
/// * `config` - Configuration for rendering
pub fn render_search(f: &mut Frame, area: Rect, config: SearchRenderConfig) {
    let SearchRenderConfig {
        search_input,
        search_status,
        loading,
        progress,
        search_results,
        search_scroll_offset,
    } = config;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(3), // Search input
                Constraint::Length(3), // Status
                Constraint::Min(0),    // Results
            ]
            .as_ref(),
        )
        .split(area);

    // Search input
    let input = Paragraph::new(highlight_spl(search_input))
        .block(Block::default().borders(Borders::ALL).title("Search Query"));
    f.render_widget(input, chunks[0]);

    // Status
    if loading {
        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("Status"))
            .gauge_style(
                Style::default()
                    .fg(Color::Cyan)
                    .bg(Color::Black)
                    .add_modifier(Modifier::ITALIC),
            )
            .ratio(progress.clamp(0.0, 1.0) as f64)
            .label(format!("{} ({:.0}%)", search_status, progress * 100.0));
        f.render_widget(gauge, chunks[1]);
    } else {
        let status = Paragraph::new(search_status)
            .block(Block::default().borders(Borders::ALL).title("Status"));
        f.render_widget(status, chunks[1]);
    }

    // Results
    if search_results.is_empty() {
        let placeholder = Paragraph::new("No results. Enter a search query and press Enter.")
            .block(Block::default().borders(Borders::ALL).title("Results"))
            .alignment(Alignment::Center);
        f.render_widget(placeholder, chunks[2]);
    } else {
        let results_text: Vec<ratatui::text::Line> = search_results
            .iter()
            .skip(search_scroll_offset)
            .map(|s| ratatui::text::Line::from(s.as_str()))
            .collect();

        let results = Paragraph::new(results_text)
            .block(Block::default().borders(Borders::ALL).title("Results"));
        f.render_widget(results, chunks[2]);
    }
}
