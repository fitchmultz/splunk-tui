//! Big text widget for large ASCII headers.
//!
//! A wrapper around `tui-big-text` for consistent theming.
//! Useful for branding headers or important status displays.
//!
//! # Example
//!
//! ```rust,ignore
//! use splunk_tui::ui::components::BigTextWidget;
//! use ratatui::layout::Alignment;
//!
//! // Create a header
//! let header = BigTextWidget::header("SPLUNK");
//!
//! // Create a custom big text
//! let big_text = BigTextWidget::new(vec!["HELLO".to_string(), "WORLD".to_string()])
//!     .color(Color::Cyan)
//!     .alignment(Alignment::Center);
//!
//! frame.render_widget(big_text, area);
//! ```

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::Widget,
};
use tui_big_text::{BigText, PixelSize};

/// A wrapper around tui-big-text for consistent theming.
#[derive(Debug, Clone)]
pub struct BigTextWidget {
    lines: Vec<String>,
    pixel_size: PixelSize,
    fg: Color,
    alignment: Alignment,
}

impl BigTextWidget {
    /// Create a new big text widget with the given lines.
    pub fn new(lines: Vec<String>) -> Self {
        Self {
            lines,
            pixel_size: PixelSize::Full,
            fg: Color::Cyan,
            alignment: Alignment::Center,
        }
    }

    /// Create a new big text widget from string slices.
    pub fn from_slices(lines: &[&str]) -> Self {
        Self::new(lines.iter().map(|s| s.to_string()).collect())
    }

    /// Set the pixel size for rendering.
    pub fn pixel_size(mut self, size: PixelSize) -> Self {
        self.pixel_size = size;
        self
    }

    /// Set the foreground color.
    pub fn color(mut self, color: Color) -> Self {
        self.fg = color;
        self
    }

    /// Set the alignment.
    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Create a default header for the application.
    pub fn header(text: impl Into<String>) -> Self {
        Self::new(vec![text.into()])
            .pixel_size(PixelSize::Full)
            .color(Color::Cyan)
            .alignment(Alignment::Center)
    }

    /// Create a smaller sub-header.
    pub fn sub_header(text: impl Into<String>) -> Self {
        Self::new(vec![text.into()])
            .pixel_size(PixelSize::HalfHeight)
            .color(Color::White)
            .alignment(Alignment::Left)
    }

    /// Create a compact header.
    pub fn compact(text: impl Into<String>) -> Self {
        Self::new(vec![text.into()])
            .pixel_size(PixelSize::Quadrant)
            .color(Color::White)
            .alignment(Alignment::Left)
    }

    /// Get the lines.
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// Get the pixel size.
    pub fn get_pixel_size(&self) -> PixelSize {
        self.pixel_size
    }

    /// Get the color.
    pub fn color_value(&self) -> Color {
        self.fg
    }

    /// Get the alignment.
    pub fn get_alignment(&self) -> Alignment {
        self.alignment
    }
}

impl Widget for BigTextWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // tui-big-text 0.7 uses builder pattern that returns BigText directly
        let lines: Vec<ratatui::text::Line> = self.lines.iter().map(|s| s.clone().into()).collect();

        let big_text = BigText::builder()
            .pixel_size(self.pixel_size)
            .style(Style::default().fg(self.fg))
            .lines(lines)
            .build();

        // tui-big-text handles internal centering, use full area
        big_text.render(area, buf);
    }
}

/// Convenience function to render a big text header.
pub fn render_header(text: &str, area: Rect, buf: &mut Buffer, color: Color) {
    BigTextWidget::header(text).color(color).render(area, buf);
}

/// Convenience function to render a sub-header.
pub fn render_sub_header(text: &str, area: Rect, buf: &mut Buffer, color: Color) {
    BigTextWidget::sub_header(text)
        .color(color)
        .render(area, buf);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_big_text_creation() {
        let lines = vec!["HELLO".to_string(), "WORLD".to_string()];
        let widget = BigTextWidget::new(lines.clone());

        assert_eq!(widget.lines(), lines.as_slice());
        assert_eq!(widget.get_pixel_size(), PixelSize::Full);
        assert_eq!(widget.color_value(), Color::Cyan);
        assert_eq!(widget.get_alignment(), Alignment::Center);
    }

    #[test]
    fn test_big_text_from_slices() {
        let widget = BigTextWidget::from_slices(&["HELLO", "WORLD"]);

        assert_eq!(widget.lines(), &["HELLO", "WORLD"]);
    }

    #[test]
    fn test_big_text_builder() {
        let widget = BigTextWidget::new(vec!["TEST".to_string()])
            .pixel_size(PixelSize::HalfHeight)
            .color(Color::Red)
            .alignment(Alignment::Left);

        assert_eq!(widget.get_pixel_size(), PixelSize::HalfHeight);
        assert_eq!(widget.color_value(), Color::Red);
        assert_eq!(widget.get_alignment(), Alignment::Left);
    }

    #[test]
    fn test_big_text_header() {
        let header = BigTextWidget::header("SPLUNK");

        assert_eq!(header.lines(), &["SPLUNK"]);
        assert_eq!(header.get_pixel_size(), PixelSize::Full);
        assert_eq!(header.color_value(), Color::Cyan);
        assert_eq!(header.get_alignment(), Alignment::Center);
    }

    #[test]
    fn test_big_text_sub_header() {
        let sub_header = BigTextWidget::sub_header("Status");

        assert_eq!(sub_header.lines(), &["Status"]);
        assert_eq!(sub_header.get_pixel_size(), PixelSize::HalfHeight);
        assert_eq!(sub_header.color_value(), Color::White);
        assert_eq!(sub_header.get_alignment(), Alignment::Left);
    }

    #[test]
    fn test_big_text_compact() {
        let compact = BigTextWidget::compact("Small");

        assert_eq!(compact.lines(), &["Small"]);
        assert_eq!(compact.get_pixel_size(), PixelSize::Quadrant);
    }

    #[test]
    fn test_big_text_widget_render() {
        let widget = BigTextWidget::header("TEST");

        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 10));
        widget.render(Rect::new(0, 0, 80, 10), &mut buf);

        // Buffer should have some content (big text uses multiple cells per character)
        let has_content = buf.content.iter().any(|cell| cell.symbol() != " ");
        assert!(has_content, "Big text should render content");
    }

    #[test]
    fn test_render_header_function() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 10));
        render_header("TEST", Rect::new(0, 0, 80, 10), &mut buf, Color::Green);

        // Should render without panic
        let has_content = buf.content.iter().any(|cell| cell.symbol() != " ");
        assert!(has_content, "render_header should render content");
    }

    #[test]
    fn test_render_sub_header_function() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 10));
        render_sub_header("TEST", Rect::new(0, 0, 80, 10), &mut buf, Color::Yellow);

        // Should render without panic
    }

    #[test]
    fn test_big_text_empty_lines() {
        let widget = BigTextWidget::new(vec![]);

        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 10));
        widget.render(Rect::new(0, 0, 80, 10), &mut buf);

        // Should handle empty lines gracefully
    }

    #[test]
    fn test_big_text_small_area() {
        let widget = BigTextWidget::header("VERYLONGTEXT");

        let mut buf = Buffer::empty(Rect::new(0, 0, 5, 2));
        widget.render(Rect::new(0, 0, 5, 2), &mut buf);

        // Should handle small area gracefully
    }
}
