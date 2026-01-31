//! Modal popup rendering for confirmations and help.
//!
//! This module provides a Builder pattern for constructing popups with
//! customizable titles, content, and types. Popups are rendered as
//! centered modal dialogs overlaid on the main UI.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::Style,
    widgets::{
        Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap,
    },
};
use splunk_config::Theme;

use crate::app::App;
use crate::input::help;

/// Default popup dimensions as percentages of screen size.
pub const POPUP_WIDTH_PERCENT: u16 = 60;
pub const POPUP_HEIGHT_PERCENT: u16 = 50;

/// The type/kind of popup dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PopupType {
    /// Help dialog with keyboard shortcuts
    Help,
    /// Confirm cancel job (holds search ID)
    ConfirmCancel(String),
    /// Confirm delete job (holds search ID)
    ConfirmDelete(String),
    /// Confirm batch cancel (holds list of SIDs)
    ConfirmCancelBatch(Vec<String>),
    /// Confirm batch delete (holds list of SIDs)
    ConfirmDeleteBatch(Vec<String>),
    /// Export search results
    ExportSearch,
    /// Show full error details with structured information
    ErrorDetails,
    /// Show index details with full metadata
    IndexDetails,
    /// Confirm enable app (holds app name)
    ConfirmEnableApp(String),
    /// Confirm disable app (holds app name)
    ConfirmDisableApp(String),
    /// Profile selector popup with list of available profiles
    ProfileSelector {
        /// List of available profile names
        profiles: Vec<String>,
        /// Currently selected index
        selected_index: usize,
    },
    /// Index creation dialog
    CreateIndex {
        name_input: String,
        max_data_size_mb: Option<u64>,
        max_hot_buckets: Option<u64>,
        max_warm_db_count: Option<u64>,
        frozen_time_period_secs: Option<u64>,
        home_path: Option<String>,
        cold_db_path: Option<String>,
        thawed_path: Option<String>,
        cold_to_frozen_dir: Option<String>,
    },
    /// Index modification dialog
    ModifyIndex {
        index_name: String,
        current_max_data_size_mb: Option<u64>,
        current_max_hot_buckets: Option<u64>,
        current_max_warm_db_count: Option<u64>,
        current_frozen_time_period_secs: Option<u64>,
        current_home_path: Option<String>,
        current_cold_db_path: Option<String>,
        current_thawed_path: Option<String>,
        current_cold_to_frozen_dir: Option<String>,
        new_max_data_size_mb: Option<u64>,
        new_max_hot_buckets: Option<u64>,
        new_max_warm_db_count: Option<u64>,
        new_frozen_time_period_secs: Option<u64>,
        new_home_path: Option<String>,
        new_cold_db_path: Option<String>,
        new_thawed_path: Option<String>,
        new_cold_to_frozen_dir: Option<String>,
    },
    /// Index deletion confirmation
    DeleteIndexConfirm { index_name: String },
    /// User creation dialog
    CreateUser {
        name_input: String,
        password_input: String,
        roles_input: String,
        realname_input: String,
        email_input: String,
        default_app_input: String,
    },
    /// User modification dialog
    ModifyUser {
        user_name: String,
        current_roles: Vec<String>,
        current_realname: Option<String>,
        current_email: Option<String>,
        current_default_app: Option<String>,
        password_input: String,
        roles_input: String,
        realname_input: String,
        email_input: String,
        default_app_input: String,
    },
    /// User deletion confirmation
    DeleteUserConfirm { user_name: String },
    /// Confirm remove app (holds app name)
    ConfirmRemoveApp(String),
    /// App installation file path input dialog
    InstallAppDialog { file_input: String },
    /// Profile creation dialog
    CreateProfile {
        name_input: String,
        base_url_input: String,
        username_input: String,
        password_input: String,
        api_token_input: String,
        skip_verify: bool,
        timeout_seconds: u64,
        max_retries: u64,
        use_keyring: bool,
        selected_field: ProfileField,
    },
    /// Profile editing dialog
    EditProfile {
        original_name: String,
        name_input: String,
        base_url_input: String,
        username_input: String,
        password_input: String,
        api_token_input: String,
        skip_verify: bool,
        timeout_seconds: u64,
        max_retries: u64,
        use_keyring: bool,
        selected_field: ProfileField,
    },
    /// Profile deletion confirmation
    DeleteProfileConfirm { profile_name: String },
}

/// Field selection for profile form navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileField {
    /// Profile name field
    Name,
    /// Base URL field
    BaseUrl,
    /// Username field
    Username,
    /// Password field
    Password,
    /// API token field
    ApiToken,
    /// Skip TLS verification toggle
    SkipVerify,
    /// Timeout seconds field
    Timeout,
    /// Max retries field
    MaxRetries,
    /// Use keyring toggle
    UseKeyring,
}

impl ProfileField {
    /// Get the next field in the form (cycles through all fields).
    pub fn next(self) -> Self {
        match self {
            ProfileField::Name => ProfileField::BaseUrl,
            ProfileField::BaseUrl => ProfileField::Username,
            ProfileField::Username => ProfileField::Password,
            ProfileField::Password => ProfileField::ApiToken,
            ProfileField::ApiToken => ProfileField::SkipVerify,
            ProfileField::SkipVerify => ProfileField::Timeout,
            ProfileField::Timeout => ProfileField::MaxRetries,
            ProfileField::MaxRetries => ProfileField::UseKeyring,
            ProfileField::UseKeyring => ProfileField::Name,
        }
    }

    /// Get the previous field in the form (cycles through all fields).
    pub fn previous(self) -> Self {
        match self {
            ProfileField::Name => ProfileField::UseKeyring,
            ProfileField::BaseUrl => ProfileField::Name,
            ProfileField::Username => ProfileField::BaseUrl,
            ProfileField::Password => ProfileField::Username,
            ProfileField::ApiToken => ProfileField::Password,
            ProfileField::SkipVerify => ProfileField::ApiToken,
            ProfileField::Timeout => ProfileField::SkipVerify,
            ProfileField::MaxRetries => ProfileField::Timeout,
            ProfileField::UseKeyring => ProfileField::MaxRetries,
        }
    }
}

/// A modal popup dialog with title, content, and type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Popup {
    /// The title displayed in the popup border
    pub title: String,
    /// The main content text of the popup
    pub content: String,
    /// The kind/type of popup (determines behavior and default styling)
    pub kind: PopupType,
}

impl Popup {
    /// Create a new `PopupBuilder` for the given popup type.
    ///
    /// # Example
    ///
    /// ```rust
    /// use splunk_tui::ui::popup::{Popup, PopupType};
    ///
    /// let popup = Popup::builder(PopupType::Help).build();
    /// assert_eq!(popup.title, "Help");
    /// ```
    pub fn builder(kind: PopupType) -> PopupBuilder {
        PopupBuilder::new(kind)
    }
}

/// Builder for constructing `Popup` instances.
pub struct PopupBuilder {
    kind: PopupType,
    title: Option<String>,
    content: Option<String>,
}

impl PopupBuilder {
    /// Create a new builder for the given popup type.
    pub fn new(kind: PopupType) -> Self {
        Self {
            kind,
            title: None,
            content: None,
        }
    }

    /// Set the popup title.
    ///
    /// If not set, a default title will be used based on the popup type.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the popup content.
    ///
    /// If not set, default content will be used based on the popup type.
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Build the `Popup` instance using defaults derived from `PopupType`.
    pub fn build(self) -> Popup {
        let (default_title, default_content) = match &self.kind {
            PopupType::Help => ("Help".to_string(), help::help_text()),
            PopupType::ConfirmCancel(sid) => (
                "Confirm Cancel".to_string(),
                format!("Cancel job {sid}? (y/n)"),
            ),
            PopupType::ConfirmDelete(sid) => (
                "Confirm Delete".to_string(),
                format!("Delete job {sid}? (y/n)"),
            ),
            PopupType::ConfirmCancelBatch(sids) => (
                "Confirm Batch Cancel".to_string(),
                format!("Cancel {} job(s)? (y/n)", sids.len()),
            ),
            PopupType::ConfirmDeleteBatch(sids) => (
                "Confirm Batch Delete".to_string(),
                format!("Delete {} job(s)? (y/n)", sids.len()),
            ),
            PopupType::ExportSearch => (
                "Export".to_string(),
                "Enter filename: export.json\nFormat: JSON (Tab to toggle)".to_string(),
            ),
            PopupType::ErrorDetails => (
                "Error Details".to_string(),
                "Press Esc, q, or e to close".to_string(),
            ),
            PopupType::IndexDetails => (
                "Index Details".to_string(),
                "Press Esc or q to close, j/k to scroll".to_string(),
            ),
            PopupType::ConfirmEnableApp(name) => (
                "Confirm Enable".to_string(),
                format!("Enable app '{}'? (y/n)", name),
            ),
            PopupType::ConfirmDisableApp(name) => (
                "Confirm Disable".to_string(),
                format!("Disable app '{}'? (y/n)", name),
            ),
            PopupType::ProfileSelector {
                profiles,
                selected_index,
            } => {
                let title = "Select Profile".to_string();
                let mut content = String::from("Select a profile to switch to:\n\n");
                for (i, profile) in profiles.iter().enumerate() {
                    if i == *selected_index {
                        content.push_str(&format!("> {} <\n", profile));
                    } else {
                        content.push_str(&format!("  {}\n", profile));
                    }
                }
                content.push_str("\n↑/↓ to navigate, Enter to select, Esc to cancel");
                (title, content)
            }
            PopupType::CreateIndex { name_input, .. } => {
                let title = "Create Index".to_string();
                let content = format!(
                    "Create new index:\n\nName: {}\n\nEnter name and press Enter to create, Esc to cancel",
                    name_input
                );
                (title, content)
            }
            PopupType::ModifyIndex { index_name, .. } => {
                let title = "Modify Index".to_string();
                let content = format!(
                    "Modify index '{}':\n\nPress Enter to apply changes, Esc to cancel",
                    index_name
                );
                (title, content)
            }
            PopupType::DeleteIndexConfirm { index_name } => {
                let title = "Confirm Delete".to_string();
                let content = format!(
                    "Delete index '{}' ?\n\nThis action cannot be undone.\n\nPress 'y' to confirm, 'n' or Esc to cancel",
                    index_name
                );
                (title, content)
            }
            PopupType::CreateUser { name_input, .. } => {
                let title = "Create User".to_string();
                let content = format!(
                    "Create new user:\n\nName: {}\n\nEnter name and press Enter to create, Esc to cancel",
                    name_input
                );
                (title, content)
            }
            PopupType::ModifyUser { user_name, .. } => {
                let title = "Modify User".to_string();
                let content = format!(
                    "Modify user '{}':\n\nPress Enter to apply changes, Esc to cancel",
                    user_name
                );
                (title, content)
            }
            PopupType::DeleteUserConfirm { user_name } => {
                let title = "Confirm Delete".to_string();
                let content = format!(
                    "Delete user '{}' ?\n\nThis action cannot be undone.\n\nPress 'y' to confirm, 'n' or Esc to cancel",
                    user_name
                );
                (title, content)
            }
            PopupType::ConfirmRemoveApp(app_name) => {
                let title = "Confirm Remove App".to_string();
                let content = format!(
                    "Remove app '{}' ?\n\nThis action cannot be undone.\n\nPress 'y' to confirm, 'n' or Esc to cancel",
                    app_name
                );
                (title, content)
            }
            PopupType::InstallAppDialog { file_input } => {
                let title = "Install App".to_string();
                let content = format!(
                    "Enter path to .spl file:\n\n{}\n\nPress Enter to install, Esc to cancel",
                    file_input
                );
                (title, content)
            }
            PopupType::CreateProfile {
                name_input,
                base_url_input,
                username_input,
                password_input,
                api_token_input,
                skip_verify,
                timeout_seconds,
                max_retries,
                use_keyring,
                selected_field,
            } => {
                let title = "Create Profile".to_string();
                let mut content = String::from("Create new profile:\n\n");

                let name_marker = if *selected_field == ProfileField::Name {
                    "> "
                } else {
                    "  "
                };
                let base_url_marker = if *selected_field == ProfileField::BaseUrl {
                    "> "
                } else {
                    "  "
                };
                let username_marker = if *selected_field == ProfileField::Username {
                    "> "
                } else {
                    "  "
                };
                let password_marker = if *selected_field == ProfileField::Password {
                    "> "
                } else {
                    "  "
                };
                let api_token_marker = if *selected_field == ProfileField::ApiToken {
                    "> "
                } else {
                    "  "
                };
                let skip_verify_marker = if *selected_field == ProfileField::SkipVerify {
                    "> "
                } else {
                    "  "
                };
                let timeout_marker = if *selected_field == ProfileField::Timeout {
                    "> "
                } else {
                    "  "
                };
                let max_retries_marker = if *selected_field == ProfileField::MaxRetries {
                    "> "
                } else {
                    "  "
                };
                let use_keyring_marker = if *selected_field == ProfileField::UseKeyring {
                    "> "
                } else {
                    "  "
                };

                content.push_str(&format!("{}Name: {}\n", name_marker, name_input));
                content.push_str(&format!(
                    "{}Base URL: {}\n",
                    base_url_marker, base_url_input
                ));
                content.push_str(&format!(
                    "{}Username: {}\n",
                    username_marker, username_input
                ));
                let password_display = if password_input.is_empty() {
                    "(empty)".to_string()
                } else {
                    "(set)".to_string()
                };
                content.push_str(&format!(
                    "{}Password: {}\n",
                    password_marker, password_display
                ));
                let token_display = if api_token_input.is_empty() {
                    "(empty)".to_string()
                } else {
                    "(set)".to_string()
                };
                content.push_str(&format!(
                    "{}API Token: {}\n",
                    api_token_marker, token_display
                ));
                content.push_str(&format!(
                    "{}Skip TLS Verify: {}\n",
                    skip_verify_marker, skip_verify
                ));
                content.push_str(&format!(
                    "{}Timeout (s): {}\n",
                    timeout_marker, timeout_seconds
                ));
                content.push_str(&format!(
                    "{}Max Retries: {}\n",
                    max_retries_marker, max_retries
                ));
                content.push_str(&format!(
                    "{}Use Keyring: {}\n",
                    use_keyring_marker, use_keyring
                ));

                content.push_str("\nTab/↑↓ to navigate fields, Enter to save, Esc to cancel");
                (title, content)
            }
            PopupType::EditProfile {
                original_name,
                name_input,
                base_url_input,
                username_input,
                password_input,
                api_token_input,
                skip_verify,
                timeout_seconds,
                max_retries,
                use_keyring,
                selected_field,
            } => {
                let title = format!("Edit Profile '{}'", original_name);
                let mut content = String::from("Edit profile:\n\n");

                let name_marker = if *selected_field == ProfileField::Name {
                    "> "
                } else {
                    "  "
                };
                let base_url_marker = if *selected_field == ProfileField::BaseUrl {
                    "> "
                } else {
                    "  "
                };
                let username_marker = if *selected_field == ProfileField::Username {
                    "> "
                } else {
                    "  "
                };
                let password_marker = if *selected_field == ProfileField::Password {
                    "> "
                } else {
                    "  "
                };
                let api_token_marker = if *selected_field == ProfileField::ApiToken {
                    "> "
                } else {
                    "  "
                };
                let skip_verify_marker = if *selected_field == ProfileField::SkipVerify {
                    "> "
                } else {
                    "  "
                };
                let timeout_marker = if *selected_field == ProfileField::Timeout {
                    "> "
                } else {
                    "  "
                };
                let max_retries_marker = if *selected_field == ProfileField::MaxRetries {
                    "> "
                } else {
                    "  "
                };
                let use_keyring_marker = if *selected_field == ProfileField::UseKeyring {
                    "> "
                } else {
                    "  "
                };

                content.push_str(&format!("{}Name: {}\n", name_marker, name_input));
                content.push_str(&format!(
                    "{}Base URL: {}\n",
                    base_url_marker, base_url_input
                ));
                content.push_str(&format!(
                    "{}Username: {}\n",
                    username_marker, username_input
                ));
                let password_display = if password_input.is_empty() {
                    "(keep existing)".to_string()
                } else {
                    "(will update)".to_string()
                };
                content.push_str(&format!(
                    "{}Password: {}\n",
                    password_marker, password_display
                ));
                let token_display = if api_token_input.is_empty() {
                    "(keep existing)".to_string()
                } else {
                    "(will update)".to_string()
                };
                content.push_str(&format!(
                    "{}API Token: {}\n",
                    api_token_marker, token_display
                ));
                content.push_str(&format!(
                    "{}Skip TLS Verify: {}\n",
                    skip_verify_marker, skip_verify
                ));
                content.push_str(&format!(
                    "{}Timeout (s): {}\n",
                    timeout_marker, timeout_seconds
                ));
                content.push_str(&format!(
                    "{}Max Retries: {}\n",
                    max_retries_marker, max_retries
                ));
                content.push_str(&format!(
                    "{}Use Keyring: {}\n",
                    use_keyring_marker, use_keyring
                ));

                content.push_str("\nTab/↑↓ to navigate fields, Enter to save, Esc to cancel");
                (title, content)
            }
            PopupType::DeleteProfileConfirm { profile_name } => {
                let title = "Confirm Delete Profile".to_string();
                let content = format!(
                    "Delete profile '{}' ?\n\nThis action cannot be undone.\n\nPress 'y' to confirm, 'n' or Esc to cancel",
                    profile_name
                );
                (title, content)
            }
        };

        Popup {
            title: self.title.unwrap_or(default_title),
            content: self.content.unwrap_or(default_content),
            kind: self.kind,
        }
    }
}

/// Render a modal popup dialog.
///
/// # Arguments
///
/// * `f` - The frame to render to
/// * `popup` - The popup to render
/// * `theme` - The color theme to use
/// * `app` - The app state (for accessing scroll offsets)
pub fn render_popup(f: &mut Frame, popup: &Popup, theme: &Theme, app: &App) {
    let size = f.area();
    let popup_area = centered_rect(POPUP_WIDTH_PERCENT, POPUP_HEIGHT_PERCENT, size);

    f.render_widget(Clear, popup_area);

    // Determine border color based on popup type
    let border_color = match &popup.kind {
        PopupType::Help
        | PopupType::ExportSearch
        | PopupType::ErrorDetails
        | PopupType::IndexDetails
        | PopupType::ProfileSelector { .. }
        | PopupType::CreateIndex { .. }
        | PopupType::ModifyIndex { .. }
        | PopupType::CreateUser { .. }
        | PopupType::ModifyUser { .. }
        | PopupType::InstallAppDialog { .. }
        | PopupType::CreateProfile { .. }
        | PopupType::EditProfile { .. } => theme.border,
        PopupType::ConfirmCancel(_)
        | PopupType::ConfirmDelete(_)
        | PopupType::ConfirmCancelBatch(_)
        | PopupType::ConfirmDeleteBatch(_)
        | PopupType::ConfirmEnableApp(_)
        | PopupType::ConfirmDisableApp(_)
        | PopupType::DeleteIndexConfirm { .. }
        | PopupType::DeleteUserConfirm { .. }
        | PopupType::ConfirmRemoveApp(_)
        | PopupType::DeleteProfileConfirm { .. } => theme.error,
    };

    // Determine wrapping behavior based on popup type
    let wrap_mode = match &popup.kind {
        PopupType::Help
        | PopupType::ExportSearch
        | PopupType::ErrorDetails
        | PopupType::IndexDetails
        | PopupType::ProfileSelector { .. }
        | PopupType::CreateIndex { .. }
        | PopupType::ModifyIndex { .. }
        | PopupType::DeleteIndexConfirm { .. }
        | PopupType::CreateUser { .. }
        | PopupType::ModifyUser { .. }
        | PopupType::DeleteUserConfirm { .. }
        | PopupType::InstallAppDialog { .. }
        | PopupType::CreateProfile { .. }
        | PopupType::EditProfile { .. }
        | PopupType::DeleteProfileConfirm { .. } => Wrap { trim: false },
        PopupType::ConfirmCancel(_)
        | PopupType::ConfirmDelete(_)
        | PopupType::ConfirmCancelBatch(_)
        | PopupType::ConfirmDeleteBatch(_)
        | PopupType::ConfirmEnableApp(_)
        | PopupType::ConfirmDisableApp(_)
        | PopupType::ConfirmRemoveApp(_) => Wrap { trim: true },
    };

    // Determine alignment based on popup type
    // Help popup uses left alignment for better readability of keybindings
    let alignment = match &popup.kind {
        PopupType::Help => Alignment::Left,
        _ => Alignment::Center,
    };

    // For Help popup, apply scroll offset and render scrollbar if needed
    if popup.kind == PopupType::Help {
        let scroll_offset = app.help_scroll_offset;

        let p = Paragraph::new(popup.content.as_str())
            .block(
                Block::default()
                    .title(popup.title.as_str())
                    .borders(Borders::ALL)
                    .style(Style::default().fg(border_color)),
            )
            .alignment(alignment)
            .wrap(wrap_mode)
            .scroll((scroll_offset as u16, 0));
        f.render_widget(p, popup_area);

        // Calculate content height and render scrollbar if needed
        // Content height is the number of lines in the content
        let content_height = popup.content.lines().count();
        let visible_lines = popup_area.height.saturating_sub(2) as usize; // Account for borders

        if content_height > visible_lines {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));
            let mut scrollbar_state =
                ScrollbarState::new(content_height.saturating_sub(1)).position(scroll_offset);
            f.render_stateful_widget(
                scrollbar,
                popup_area.inner(Margin::new(0, 1)),
                &mut scrollbar_state,
            );
        }
    } else {
        let p = Paragraph::new(popup.content.as_str())
            .block(
                Block::default()
                    .title(popup.title.as_str())
                    .borders(Borders::ALL)
                    .style(Style::default().fg(border_color)),
            )
            .alignment(alignment)
            .wrap(wrap_mode);
        f.render_widget(p, popup_area);
    }
}

/// Create a centered rectangle with the given percentage of the screen size.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}
