//! Users screen rendering.
//!
//! Renders the list of Splunk users with their roles and last login times.

use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Borders, List, ListItem, ListState},
};
use splunk_client::models::User;
use splunk_config::Theme;

use crate::ui::theme::ThemeExt;
use crate::ui::widgets::{render_empty_state, render_loading_state};

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
    /// Current spinner frame for loading animation
    pub spinner_frame: u8,
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
        spinner_frame,
    } = config;

    if loading && users.is_none() {
        render_loading_state(f, area, "Users", "Loading users...", spinner_frame, theme);
        return;
    }

    let users = match users {
        Some(u) => u,
        None => {
            render_empty_state(f, area, "Users", "users");
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
                .border_style(theme.border())
                .title_style(theme.title()),
        )
        .highlight_style(theme.highlight());
    f.render_stateful_widget(list, area, state);
}

fn format_last_login(timestamp: usize) -> String {
    // Format timestamp as seconds since Unix epoch
    // This is more readable than the raw timestamp while not requiring chrono
    format!("{}s", timestamp)
}
