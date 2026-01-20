//! Search screen rendering.
//!
//! Renders the search input, status, and results for running Splunk searches.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Paragraph},
};
use serde_json::Value;

/// Configuration for rendering the search screen.
pub struct SearchRenderConfig<'a> {
    /// The current search input text
    pub search_input: &'a str,
    /// The current search status message
    pub search_status: &'a str,
    /// The search results to display
    pub search_results: &'a Vec<Value>,
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
    let input = Paragraph::new(search_input)
        .block(Block::default().borders(Borders::ALL).title("Search Query"));
    f.render_widget(input, chunks[0]);

    // Status
    let status =
        Paragraph::new(search_status).block(Block::default().borders(Borders::ALL).title("Status"));
    f.render_widget(status, chunks[1]);

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
            .map(|v| {
                ratatui::text::Line::from(
                    serde_json::to_string_pretty(v).unwrap_or_else(|_| "<invalid>".to_string()),
                )
            })
            .collect();

        let results = Paragraph::new(results_text)
            .block(Block::default().borders(Borders::ALL).title("Results"));
        f.render_widget(results, chunks[2]);
    }
}
