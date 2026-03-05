//! Data models screen rendering.
//!
//! Renders the list of Splunk data models.

use crate::ui::theme::ThemeExt;
use crate::ui::widgets::{render_empty_state, render_loading_state};
use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Borders, List, ListItem, ListState},
};
use splunk_client::models::DataModel;
use splunk_config::Theme;

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
        render_loading_state(
            f,
            area,
            "Data Models",
            "Loading data models...",
            spinner_frame,
            theme,
        );
        return;
    }

    let data_models = match data_models {
        Some(d) => d,
        None => {
            render_empty_state(f, area, "Data Models", "data models");
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
                .border_style(theme.border())
                .title_style(theme.title()),
        )
        .highlight_style(theme.highlight());
    f.render_stateful_widget(list, area, state);
}
