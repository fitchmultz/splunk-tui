//! Data models screen rendering.
//!
//! Renders the list of Splunk data models.

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
};
use splunk_client::models::DataModel;
use splunk_config::Theme;

/// Spinner characters for animated loading indicator.
const SPINNER_CHARS: [char; 8] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧'];

/// Configuration for rendering the data models screen.
pub struct DataModelsRenderConfig<'a> {
    /// Whether data is currently loading
    pub loading: bool,
    /// The list of data models to display
    pub data_models: Option<&'a [DataModel]>,
    /// The current list selection state
    pub state: &'a mut ListState,
    /// Theme for consistent styling.
    pub theme: &'a Theme,
    /// Current spinner frame for loading animation
    pub spinner_frame: u8,
}

/// Render the data models screen.
///
/// # Arguments
///
/// * `f` - The frame to render to
/// * `area` - The area to render within
/// * `config` - Configuration for rendering
pub fn render_datamodels(f: &mut Frame, area: Rect, config: DataModelsRenderConfig) {
    let DataModelsRenderConfig {
        loading,
        data_models,
        state,
        theme,
        spinner_frame,
    } = config;

    if loading && data_models.is_none() {
        let spinner = SPINNER_CHARS[spinner_frame as usize % SPINNER_CHARS.len()];
        let loading_widget =
            ratatui::widgets::Paragraph::new(format!("{} Loading data models...", spinner))
                .block(Block::default().borders(Borders::ALL).title("Data Models"))
                .alignment(Alignment::Center);
        f.render_widget(loading_widget, area);
        return;
    }

    let data_models = match data_models {
        Some(d) => d,
        None => {
            let placeholder =
                ratatui::widgets::Paragraph::new("No data models loaded. Press 'r' to refresh.")
                    .block(Block::default().borders(Borders::ALL).title("Data Models"))
                    .alignment(Alignment::Center);
            f.render_widget(placeholder, area);
            return;
        }
    };

    let items: Vec<ListItem> = data_models
        .iter()
        .map(|d| {
            let display_name = if d.displayName.is_empty() {
                &d.name
            } else {
                &d.displayName
            };
            let owner_info = if d.owner.is_empty() {
                String::new()
            } else {
                format!(" (by {})", d.owner)
            };
            ListItem::new(format!("{}{}", display_name, owner_info))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Data Models")
                .border_style(Style::default().fg(theme.border))
                .title_style(Style::default().fg(theme.title)),
        )
        .highlight_style(
            Style::default()
                .fg(theme.highlight_fg)
                .bg(theme.highlight_bg)
                .add_modifier(Modifier::BOLD),
        );
    f.render_stateful_widget(list, area, state);
}
