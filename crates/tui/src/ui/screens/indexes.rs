//! Indexes screen rendering.
//!
//! Renders the list of Splunk indexes with their event counts and sizes.

use ratatui::{
    Frame,
    layout::Alignment,
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, List, ListItem, ListState},
};
use splunk_client::models::Index;

/// Configuration for rendering the indexes screen.
pub struct IndexesRenderConfig<'a> {
    /// Whether data is currently loading
    pub loading: bool,
    /// The list of indexes to display
    pub indexes: Option<&'a [Index]>,
    /// The current list selection state
    pub state: &'a mut ListState,
}

/// Render the indexes screen.
///
/// # Arguments
///
/// * `f` - The frame to render to
/// * `area` - The area to render within
/// * `config` - Configuration for rendering
pub fn render_indexes(f: &mut Frame, area: Rect, config: IndexesRenderConfig) {
    let IndexesRenderConfig {
        loading,
        indexes,
        state,
    } = config;

    if loading && indexes.is_none() {
        let loading_widget = ratatui::widgets::Paragraph::new("Loading indexes...")
            .block(Block::default().borders(Borders::ALL).title("Indexes"))
            .alignment(Alignment::Center);
        f.render_widget(loading_widget, area);
        return;
    }

    let indexes = match indexes {
        Some(i) => i,
        None => {
            let placeholder =
                ratatui::widgets::Paragraph::new("No indexes loaded. Press 'r' to refresh.")
                    .block(Block::default().borders(Borders::ALL).title("Indexes"))
                    .alignment(Alignment::Center);
            f.render_widget(placeholder, area);
            return;
        }
    };

    let items: Vec<ListItem> = indexes
        .iter()
        .map(|i| {
            ListItem::new(format!(
                "{} - {} events, {} MB",
                i.name, i.total_event_count, i.current_db_size_mb
            ))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Indexes"))
        .highlight_style(Style::default().fg(ratatui::style::Color::Yellow));
    f.render_stateful_widget(list, area, state);
}
