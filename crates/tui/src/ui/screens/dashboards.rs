//! Dashboards screen rendering.
//!
//! Renders the list of Splunk dashboards.

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    widgets::{Block, Borders, List, ListItem, ListState},
};
use splunk_client::models::Dashboard;
use splunk_config::Theme;

use crate::ui::theme::{ThemeExt, spinner_char};

/// Configuration for rendering the dashboards screen.
pub struct DashboardsRenderConfig<'a> {
    /// Whether data is currently loading
    pub loading: bool,
    /// The list of dashboards to display
    pub dashboards: Option<&'a [Dashboard]>,
    /// The current list selection state
    pub state: &'a mut ListState,
    /// Theme for consistent styling.
    pub theme: &'a Theme,
    /// Current spinner frame for loading animation
    pub spinner_frame: u8,
}

/// Render the dashboards screen.
///
/// # Arguments
///
/// * `f` - The frame to render to
/// * `area` - The area to render within
/// * `config` - Configuration for rendering
pub fn render_dashboards(f: &mut Frame, area: Rect, config: DashboardsRenderConfig) {
    let DashboardsRenderConfig {
        loading,
        dashboards,
        state,
        theme,
        spinner_frame,
    } = config;

    if loading && dashboards.is_none() {
        let spinner = spinner_char(spinner_frame);
        let loading_widget =
            ratatui::widgets::Paragraph::new(format!("{} Loading dashboards...", spinner))
                .block(Block::default().borders(Borders::ALL).title("Dashboards"))
                .alignment(Alignment::Center);
        f.render_widget(loading_widget, area);
        return;
    }

    let dashboards = match dashboards {
        Some(d) => d,
        None => {
            let placeholder =
                ratatui::widgets::Paragraph::new("No dashboards loaded. Press 'r' to refresh.")
                    .block(Block::default().borders(Borders::ALL).title("Dashboards"))
                    .alignment(Alignment::Center);
            f.render_widget(placeholder, area);
            return;
        }
    };

    let items: Vec<ListItem> = dashboards
        .iter()
        .map(|d| {
            let label = if d.label.is_empty() {
                &d.name
            } else {
                &d.label
            };
            let author_info = if d.author.is_empty() {
                String::new()
            } else {
                format!(" (by {})", d.author)
            };
            ListItem::new(format!("{}{}", label, author_info))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Dashboards")
                .border_style(theme.border())
                .title_style(theme.title()),
        )
        .highlight_style(theme.highlight());
    f.render_stateful_widget(list, area, state);
}
