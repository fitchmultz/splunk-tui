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
        | PopupType::ProfileSelector { .. } => theme.border,
        PopupType::ConfirmCancel(_)
        | PopupType::ConfirmDelete(_)
        | PopupType::ConfirmCancelBatch(_)
        | PopupType::ConfirmDeleteBatch(_)
        | PopupType::ConfirmEnableApp(_)
        | PopupType::ConfirmDisableApp(_) => theme.error,
    };

    // Determine wrapping behavior based on popup type
    let wrap_mode = match &popup.kind {
        PopupType::Help
        | PopupType::ExportSearch
        | PopupType::ErrorDetails
        | PopupType::IndexDetails
        | PopupType::ProfileSelector { .. } => Wrap { trim: false },
        PopupType::ConfirmCancel(_)
        | PopupType::ConfirmDelete(_)
        | PopupType::ConfirmCancelBatch(_)
        | PopupType::ConfirmDeleteBatch(_)
        | PopupType::ConfirmEnableApp(_)
        | PopupType::ConfirmDisableApp(_) => Wrap { trim: true },
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
