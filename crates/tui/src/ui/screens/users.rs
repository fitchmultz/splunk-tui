//! Users screen rendering.
//!
//! Renders the list of Splunk users with their roles and last login times.

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
};
use splunk_client::models::User;
use splunk_config::Theme;

/// Configuration for rendering the users screen.
pub struct UsersRenderConfig<'a> {
    /// Whether data is currently loading
    pub loading: bool,
    /// The list of users to display
    pub users: Option<&'a [User]>,
    /// The current list selection state
    pub state: &'a mut ListState,
    /// Theme for consistent styling.
    pub theme: &'a Theme,
}

/// Render the users screen.
///
/// # Arguments
///
/// * `f` - The frame to render to
/// * `area` - The area to render within
/// * `config` - Configuration for rendering
pub fn render_users(f: &mut Frame, area: Rect, config: UsersRenderConfig) {
    let UsersRenderConfig {
        loading,
        users,
        state,
        theme,
    } = config;

    if loading && users.is_none() {
        let loading_widget = ratatui::widgets::Paragraph::new("Loading users...")
            .block(Block::default().borders(Borders::ALL).title("Users"))
            .alignment(Alignment::Center);
        f.render_widget(loading_widget, area);
        return;
    }

    let users = match users {
        Some(u) => u,
        None => {
            let placeholder =
                ratatui::widgets::Paragraph::new("No users loaded. Press 'r' to refresh.")
                    .block(Block::default().borders(Borders::ALL).title("Users"))
                    .alignment(Alignment::Center);
            f.render_widget(placeholder, area);
            return;
        }
    };

    let items: Vec<ListItem> = users
        .iter()
        .map(|user| {
            let name = &user.name;
            let realname = user.realname.as_deref().unwrap_or("");
            let roles = if user.roles.is_empty() {
                String::from("no roles")
            } else {
                user.roles.join(", ")
            };
            let last_login = user
                .last_successful_login
                .map(format_last_login)
                .unwrap_or_else(|| String::from("never"));

            ListItem::new(format!(
                "{} ({}) - Roles: {} - Last login: {}",
                name, realname, roles, last_login
            ))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Users")
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

fn format_last_login(timestamp: u64) -> String {
    // Format timestamp as seconds since Unix epoch
    // This is more readable than the raw timestamp while not requiring chrono
    format!("{}s", timestamp)
}
