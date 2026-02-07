//! Empty state widget for TUI screens.
//!
//! Provides a consistent empty state display with refresh hint
//! for all TUI screens when no data has been loaded.

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    widgets::{Block, Borders, Paragraph},
};

/// Render an empty state widget with refresh hint.
///
/// # Arguments
///
/// * `f` - The frame to render to
/// * `area` - The area to render within
/// * `title` - The title for the widget border (e.g., "Overview", "Fired Alerts")
/// * `resource_name` - The resource name for the message (e.g., "overview data", "fired alerts")
///
/// # Example
///
/// ```rust,ignore
/// render_empty_state(f, area, "Overview", "overview data");
/// // Displays: "No overview data loaded. Press 'r' to refresh."
/// ```
pub fn render_empty_state(f: &mut Frame, area: Rect, title: &str, resource_name: &str) {
    let message = format!("No {} loaded. Press 'r' to refresh.", resource_name);
    let placeholder = Paragraph::new(message)
        .block(Block::default().borders(Borders::ALL).title(title))
        .alignment(Alignment::Center);
    f.render_widget(placeholder, area);
}

/// Render an empty state with a completely custom message.
///
/// # Arguments
///
/// * `f` - The frame to render to
/// * `area` - The area to render within
/// * `title` - The title for the widget border
/// * `message` - The custom message to display
///
/// # Example
///
/// ```rust,ignore
/// render_empty_state_custom(f, area, "Macros", "Press 'r' to load macros");
/// ```
pub fn render_empty_state_custom(f: &mut Frame, area: Rect, title: &str, message: &str) {
    let placeholder = Paragraph::new(message)
        .block(Block::default().borders(Borders::ALL).title(title))
        .alignment(Alignment::Center);
    f.render_widget(placeholder, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn test_render_empty_state() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                render_empty_state(f, f.area(), "Overview", "overview data");
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        let content = buffer
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(content.contains("Overview"));
        assert!(content.contains("No overview data loaded"));
        assert!(content.contains("Press 'r' to refresh"));
    }

    #[test]
    fn test_render_empty_state_custom() {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|f| {
                render_empty_state_custom(f, f.area(), "Macros", "Press 'r' to load macros");
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        let content = buffer
            .content
            .iter()
            .map(|c| c.symbol())
            .collect::<String>();
        assert!(content.contains("Macros"));
        assert!(content.contains("Press 'r' to load macros"));
    }
}
