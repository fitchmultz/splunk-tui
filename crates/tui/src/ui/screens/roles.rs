//! Roles screen rendering.
//!
//! Renders the list of Splunk roles with their capabilities and settings.

use crate::ui::theme::ThemeExt;
use crate::ui::widgets::{render_empty_state, render_loading_state};
use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Borders, List, ListItem, ListState},
};
use splunk_client::models::Role;
use splunk_config::Theme;

/// Configuration for rendering the roles screen.
pub struct RolesRenderConfig<'a> {
    /// Whether data is currently loading
    pub loading: bool,
    /// The list of roles to display
    pub roles: Option<&'a [Role]>,
    /// The current list selection state
    pub state: &'a mut ListState,
    /// Theme for consistent styling.
    pub theme: &'a Theme,
    /// Current spinner frame for loading animation
    pub spinner_frame: u8,
}

/// Render the roles screen.
///
/// # Arguments
///
/// * `f` - The frame to render to
/// * `area` - The area to render within
/// * `config` - Configuration for rendering
pub fn render_roles(f: &mut Frame, area: Rect, config: RolesRenderConfig) {
    let RolesRenderConfig {
        loading,
        roles,
        state,
        theme,
        spinner_frame,
    } = config;

    if loading && roles.is_none() {
        render_loading_state(f, area, "Roles", "Loading roles...", spinner_frame, theme);
        return;
    }

    let roles = match roles {
        Some(r) => r,
        None => {
            render_empty_state(f, area, "Roles", "roles");
            return;
        }
    };

    let items: Vec<ListItem> = roles
        .iter()
        .map(|role| {
            let name = &role.name;
            let capabilities = if role.capabilities.is_empty() {
                String::from("no capabilities")
            } else if role.capabilities.len() <= 3 {
                role.capabilities.join(", ")
            } else {
                format!("{} capabilities", role.capabilities.len())
            };
            let indexes = if role.search_indexes.is_empty() {
                String::from("no indexes")
            } else if role.search_indexes.len() <= 2 {
                format!("indexes: {}", role.search_indexes.join(", "))
            } else {
                format!("{} indexes", role.search_indexes.len())
            };

            ListItem::new(format!("{} - {} - {}", name, capabilities, indexes))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Roles")
                .border_style(theme.border())
                .title_style(theme.title()),
        )
        .highlight_style(theme.highlight());
    f.render_stateful_widget(list, area, state);
}
