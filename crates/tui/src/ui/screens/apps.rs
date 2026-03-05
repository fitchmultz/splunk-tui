//! Apps screen rendering.
//!
//! Renders the list of installed Splunk apps with their versions and status.

use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Borders, List, ListItem, ListState},
};
use splunk_client::models::App;
use splunk_config::Theme;

use crate::ui::theme::ThemeExt;
use crate::ui::widgets::{render_empty_state, render_loading_state};

/// Configuration for rendering the apps screen.
pub struct AppsRenderConfig<'a> {
    /// Whether data is currently loading
    pub loading: bool,
    /// The list of apps to display
    pub apps: Option<&'a [App]>,
    /// The current list selection state
    pub state: &'a mut ListState,
    /// Theme for consistent styling.
    pub theme: &'a Theme,
    /// Current spinner animation frame
    pub spinner_frame: u8,
}

/// Render the apps screen.
///
/// # Arguments
///
/// * `f` - The frame to render to
/// * `area` - The area to render within
/// * `config` - Configuration for rendering
pub fn render_apps(f: &mut Frame, area: Rect, config: AppsRenderConfig) {
    let AppsRenderConfig {
        loading,
        apps,
        state,
        theme,
        spinner_frame,
    } = config;

    if loading && apps.is_none() {
        render_loading_state(f, area, "Apps", "Loading apps...", spinner_frame, theme);
        return;
    }

    let apps = match apps {
        Some(a) => a,
        None => {
            render_empty_state(f, area, "Apps", "apps");
            return;
        }
    };

    let items: Vec<ListItem> = apps
        .iter()
        .map(|app| {
            let version = app.version.as_deref().unwrap_or("unknown");
            let status = if app.disabled { " [disabled]" } else { "" };
            let label = app.label.as_deref().unwrap_or("");
            let label_part = if label.is_empty() { "" } else { " " };
            ListItem::new(format!(
                "{}{}{} v{}{}",
                app.name, label_part, label, version, status
            ))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Apps")
                .border_style(theme.border())
                .title_style(theme.title()),
        )
        .highlight_style(theme.highlight());
    f.render_stateful_widget(list, area, state);
}
