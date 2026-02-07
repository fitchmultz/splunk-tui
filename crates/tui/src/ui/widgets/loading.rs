//! Loading state widget for TUI screens.
//!
//! Provides a consistent loading indicator with animated spinner
//! for all TUI screens.

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    widgets::{Block, Borders, Paragraph},
};
use splunk_config::Theme;

use crate::ui::theme::spinner_char;

/// Render a loading state widget with spinner animation.
///
/// # Arguments
///
/// * `f` - The frame to render to
/// * `area` - The area to render within
/// * `title` - The title for the widget border (e.g., "Overview", "Fired Alerts")
/// * `message` - The loading message to display (e.g., "Loading overview...", "Loading fired alerts...")
/// * `spinner_frame` - The current spinner animation frame
/// * `_theme` - The theme for styling (currently unused but kept for API consistency)
///
/// # Example
///
/// ```rust,ignore
/// render_loading_state(
///     f,
///     area,
///     "Overview",
///     "Loading overview...",
///     spinner_frame,
///     theme,
/// );
/// ```
pub fn render_loading_state(
    f: &mut Frame,
    area: Rect,
    title: &str,
    message: &str,
    spinner_frame: u8,
    _theme: &Theme,
) {
    let spinner = spinner_char(spinner_frame);
    let loading_widget = Paragraph::new(format!("{} {}", spinner, message))
        .block(Block::default().borders(Borders::ALL).title(title))
        .alignment(Alignment::Center);
    f.render_widget(loading_widget, area);
}

/// Render a loading state with the standard "Loading {resource}..." message format.
///
/// This is a convenience wrapper around [`render_loading_state`] that formats
/// the message as "Loading {resource}...".
///
/// # Arguments
///
/// * `f` - The frame to render to
/// * `area` - The area to render within
/// * `title` - The title for the widget border
/// * `resource` - The resource name (e.g., "overview", "fired alerts")
/// * `spinner_frame` - The current spinner animation frame
/// * `theme` - The theme for styling
///
/// # Example
///
/// ```rust,ignore
/// render_loading(
///     f,
///     area,
///     "Overview",
///     "overview",
///     spinner_frame,
///     theme,
/// );
/// // Displays: "⣾ Loading overview..."
/// ```
pub fn render_loading(
    f: &mut Frame,
    area: Rect,
    title: &str,
    resource: &str,
    spinner_frame: u8,
    theme: &Theme,
) {
    let message = format!("Loading {}...", resource);
    render_loading_state(f, area, title, &message, spinner_frame, theme);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn test_render_loading_state() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::default();

        terminal
            .draw(|f| {
                render_loading_state(f, f.area(), "Test", "Loading test data...", 0, &theme);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        let content = buffer
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(content.contains("Test"));
        assert!(content.contains("Loading"));
    }

    #[test]
    fn test_render_loading() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let theme = Theme::default();

        terminal
            .draw(|f| {
                render_loading(f, f.area(), "Overview", "overview", 0, &theme);
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        let content = buffer
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(content.contains("Overview"));
        assert!(content.contains("Loading"));
    }
}
