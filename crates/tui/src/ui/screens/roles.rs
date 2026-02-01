//! Roles screen rendering.
//!
//! Renders the list of Splunk roles with their capabilities and settings.

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
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
    } = config;

    if loading && roles.is_none() {
        let loading_widget = ratatui::widgets::Paragraph::new("Loading roles...")
            .block(Block::default().borders(Borders::ALL).title("Roles"))
            .alignment(Alignment::Center);
        f.render_widget(loading_widget, area);
        return;
    }

    let roles = match roles {
        Some(r) => r,
        None => {
            let placeholder =
                ratatui::widgets::Paragraph::new("No roles loaded. Press 'r' to refresh.")
                    .block(Block::default().borders(Borders::ALL).title("Roles"))
                    .alignment(Alignment::Center);
            f.render_widget(placeholder, area);
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
