//! Modal popup rendering for confirmations and help.
//!
//! This module provides a Builder pattern for constructing popups with
//! customizable titles, content, and types. Popups are rendered as
//! centered modal dialogs overlaid on the main UI.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

/// Default popup dimensions as percentages of screen size.
pub const POPUP_WIDTH_PERCENT: u16 = 60;
pub const POPUP_HEIGHT_PERCENT: u16 = 20;

/// The type/kind of popup dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PopupType {
    /// Help dialog with keyboard shortcuts
    Help,
    /// Confirm cancel job (holds search ID)
    ConfirmCancel(String),
    /// Confirm delete job (holds search ID)
    ConfirmDelete(String),
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
    /// Create a new PopupBuilder for the given popup type.
    ///
    /// # Example
    ///
    /// ```rust
    /// use splunk_tui::ui::popup::{Popup, PopupType};
    ///
    /// let popup = Popup::builder(PopupType::Help)
    ///     .title("Custom Help".to_string())
    ///     .content("Custom help text".to_string())
    ///     .build();
    /// ```
    pub fn builder(kind: PopupType) -> PopupBuilder {
        PopupBuilder::new(kind)
    }
}

/// Builder for constructing Popup instances with customizable properties.
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
    #[allow(dead_code)]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the popup content.
    ///
    /// If not set, default content will be used based on the popup type.
    #[allow(dead_code)]
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Build the Popup instance.
    ///
    /// Default values are used for title and content if not explicitly set.
    pub fn build(self) -> Popup {
        let (default_title, default_content) = match &self.kind {
            PopupType::Help => (
                "Help".to_string(),
                r#"
Global Keys:
  1-4   Navigate screens
  ?     Help
  q     Quit

Search Screen:
  Enter Run search
  j/k   Navigate results
  PgDn  Page down
  PgUp  Page up
  Home  Go to top
  End   Go to bottom

Jobs Screen:
  r     Refresh jobs
  a     Toggle auto-refresh
  s     Cycle sort column
  /     Filter jobs
  c     Cancel job
  d     Delete job
  Enter Inspect job

Job Details Screen:
  Esc   Back to jobs
            "#
                .to_string(),
            ),
            PopupType::ConfirmCancel(sid) => (
                "Confirm Cancel".to_string(),
                format!("Cancel job {sid}? (y/n)"),
            ),
            PopupType::ConfirmDelete(sid) => (
                "Confirm Delete".to_string(),
                format!("Delete job {sid}? (y/n)"),
            ),
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
pub fn render_popup(f: &mut Frame, popup: &Popup) {
    let size = f.area();
    let popup_area = centered_rect(POPUP_WIDTH_PERCENT, POPUP_HEIGHT_PERCENT, size);

    f.render_widget(Clear, popup_area);

    // Determine border color based on popup type
    let border_color = match &popup.kind {
        PopupType::Help => Color::Cyan,
        PopupType::ConfirmCancel(_) | PopupType::ConfirmDelete(_) => Color::Red,
    };

    // Determine wrapping behavior based on popup type
    let wrap_mode = match &popup.kind {
        PopupType::Help => Wrap { trim: false },
        PopupType::ConfirmCancel(_) | PopupType::ConfirmDelete(_) => Wrap { trim: true },
    };

    let p = Paragraph::new(popup.content.as_str())
        .block(
            Block::default()
                .title(popup.title.as_str())
                .borders(Borders::ALL)
                .style(Style::default().fg(border_color)),
        )
        .alignment(Alignment::Center)
        .wrap(wrap_mode);
    f.render_widget(p, popup_area);
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
