//! Indexes screen rendering.
//!
//! Renders the list of Splunk indexes with their event counts and sizes.

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    widgets::{Block, Borders, List, ListItem, ListState},
};
use splunk_client::models::Index;
use splunk_config::Theme;

use crate::ui::theme::{ThemeExt, spinner_char};

/// Configuration for rendering the indexes screen.
pub struct IndexesRenderConfig<'a> {
    /// Whether data is currently loading
    pub loading: bool,
    /// The list of indexes to display
    pub indexes: Option<&'a [Index]>,
    /// The current list selection state
    pub state: &'a mut ListState,
    /// Theme for consistent styling.
    pub theme: &'a Theme,
    /// Current spinner frame for loading animation (0-7).
    pub spinner_frame: u8,
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
        theme,
        spinner_frame,
    } = config;

    if loading && indexes.is_none() {
        let spinner = spinner_char(spinner_frame);
        let loading_widget =
            ratatui::widgets::Paragraph::new(format!("{} Loading indexes...", spinner))
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
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Indexes")
                .border_style(theme.border())
                .title_style(theme.title()),
        )
        .highlight_style(theme.highlight());
    f.render_stateful_widget(list, area, state);
}
