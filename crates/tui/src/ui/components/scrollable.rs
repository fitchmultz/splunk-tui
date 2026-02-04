//! Scrollable container component.
//!
//! Provides a scrollable container for content that exceeds the available space.
//!
//! # Example
//!
//! ```rust,ignore
//! use splunk_tui::ui::components::ScrollableContainer;
//!
//! let mut container = ScrollableContainer::new(100); // 100 lines of content
//!
//! // Scroll
//! container.scroll_down(5);
//! container.scroll_up(2);
//!
//! // Render
//! container.render(frame, area, theme, |frame, area, offset, height| {
//!     // Render visible content
//! });
//! ```

use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};
use splunk_config::Theme;

use crate::ui::theme::ThemeExt;

/// A scrollable container for content.
#[derive(Debug, Clone)]
pub struct ScrollableContainer {
    /// Total number of lines in the content.
    total_lines: usize,
    /// Current scroll offset (top visible line).
    scroll_offset: usize,
    /// Whether to show the scrollbar.
    show_scrollbar: bool,
    /// Optional block (borders/title).
    block: Option<Block<'static>>,
    /// Whether the container is focused.
    focused: bool,
}

impl ScrollableContainer {
    /// Create a new scrollable container.
    pub fn new(total_lines: usize) -> Self {
        Self {
            total_lines,
            scroll_offset: 0,
            show_scrollbar: true,
            block: None,
            focused: false,
        }
    }

    /// Set whether to show the scrollbar.
    pub fn show_scrollbar(mut self, show: bool) -> Self {
        self.show_scrollbar = show;
        self
    }

    /// Set the block (borders/title).
    pub fn block(mut self, block: Block<'static>) -> Self {
        self.block = Some(block);
        self
    }

    /// Set whether the container is focused.
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    /// Check if the container is focused.
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// Get the current scroll offset.
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Set the scroll offset.
    pub fn set_scroll_offset(&mut self, offset: usize) {
        self.scroll_offset = offset.min(self.max_scroll());
    }

    /// Get the maximum scroll offset.
    pub fn max_scroll(&self) -> usize {
        self.total_lines.saturating_sub(1)
    }

    /// Scroll down by the given number of lines.
    pub fn scroll_down(&mut self, lines: usize) {
        self.scroll_offset = (self.scroll_offset + lines).min(self.max_scroll());
    }

    /// Scroll up by the given number of lines.
    pub fn scroll_up(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }

    /// Scroll to the top.
    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    /// Scroll to the bottom.
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.max_scroll();
    }

    /// Scroll down one page.
    pub fn page_down(&mut self, page_height: usize) {
        self.scroll_down(page_height.saturating_sub(1));
    }

    /// Scroll up one page.
    pub fn page_up(&mut self, page_height: usize) {
        self.scroll_up(page_height.saturating_sub(1));
    }

    /// Update the total number of lines.
    pub fn set_total_lines(&mut self, total: usize) {
        self.total_lines = total;
        self.scroll_offset = self.scroll_offset.min(self.max_scroll());
    }

    /// Get the visible range for the given viewport height.
    pub fn visible_range(&self, viewport_height: usize) -> std::ops::Range<usize> {
        let end = (self.scroll_offset + viewport_height).min(self.total_lines);
        self.scroll_offset..end
    }

    /// Check if a line is currently visible.
    pub fn is_visible(&self, line: usize, viewport_height: usize) -> bool {
        self.visible_range(viewport_height).contains(&line)
    }

    /// Render the scrollable container with content.
    ///
    /// The `render_content` callback receives:
    /// - `frame`: The frame to render into
    /// - `area`: The content area (accounting for borders if present)
    /// - `scroll_offset`: The current scroll offset
    /// - `viewport_height`: The height of the viewport
    pub fn render<F>(&self, frame: &mut Frame, area: Rect, theme: &Theme, render_content: F)
    where
        F: FnOnce(&mut Frame, Rect, usize, usize), // frame, area, scroll_offset, viewport_height
    {
        let viewport_height = area.height as usize;

        // Calculate content area (accounting for borders if present)
        let content_area = if let Some(block) = &self.block {
            let border_style = if self.focused {
                theme.border_focused()
            } else {
                theme.border()
            };
            let inner = block.inner(area);
            frame.render_widget(block.clone().border_style(border_style), area);
            inner
        } else {
            area
        };

        // Render content
        render_content(frame, content_area, self.scroll_offset, viewport_height);

        // Render scrollbar if enabled and needed
        if self.show_scrollbar && self.total_lines > viewport_height {
            let scrollbar = Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));

            let mut scrollbar_state = ScrollbarState::new(self.total_lines)
                .position(self.scroll_offset)
                .viewport_content_length(viewport_height);

            frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
        }
    }

    /// Render text content with scrolling.
    pub fn render_text(&self, frame: &mut Frame, area: Rect, theme: &Theme, lines: &[String]) {
        self.render(
            frame,
            area,
            theme,
            |frame, content_area, scroll_offset, viewport_height| {
                let visible_lines: Vec<ratatui::text::Line> = lines
                    .iter()
                    .skip(scroll_offset)
                    .take(viewport_height)
                    .map(|line| ratatui::text::Line::from(line.clone()).style(theme.text()))
                    .collect();

                let paragraph = Paragraph::new(visible_lines);
                frame.render_widget(paragraph, content_area);
            },
        );
    }
}

impl Default for ScrollableContainer {
    fn default() -> Self {
        Self::new(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scrollable_container_creation() {
        let container = ScrollableContainer::new(100);

        assert_eq!(container.scroll_offset(), 0);
        assert_eq!(container.max_scroll(), 99);
        assert!(container.show_scrollbar);
    }

    #[test]
    fn test_scrollable_container_navigation() {
        let mut container = ScrollableContainer::new(100);

        assert_eq!(container.scroll_offset(), 0);

        container.scroll_down(10);
        assert_eq!(container.scroll_offset(), 10);

        container.scroll_up(5);
        assert_eq!(container.scroll_offset(), 5);

        container.scroll_to_bottom();
        assert_eq!(container.scroll_offset(), 99);

        container.scroll_to_top();
        assert_eq!(container.scroll_offset(), 0);
    }

    #[test]
    fn test_scrollable_container_bounds() {
        let mut container = ScrollableContainer::new(10);

        // Try to scroll past the end
        container.scroll_down(100);
        assert_eq!(container.scroll_offset(), 9);

        // Try to scroll before the beginning
        container.scroll_up(100);
        assert_eq!(container.scroll_offset(), 0);
    }

    #[test]
    fn test_visible_range() {
        let mut container = ScrollableContainer::new(100);
        container.scroll_down(20);

        let range = container.visible_range(10);
        assert_eq!(range.start, 20);
        assert_eq!(range.end, 30);
    }

    #[test]
    fn test_is_visible() {
        let container = ScrollableContainer::new(100);

        assert!(container.is_visible(0, 10));
        assert!(container.is_visible(9, 10));
        assert!(!container.is_visible(10, 10));
    }

    #[test]
    fn test_page_navigation() {
        let mut container = ScrollableContainer::new(100);

        container.page_down(10);
        assert_eq!(container.scroll_offset(), 9);

        container.page_down(10);
        assert_eq!(container.scroll_offset(), 18);

        container.page_up(10);
        assert_eq!(container.scroll_offset(), 9);
    }

    #[test]
    fn test_set_total_lines_adjusts_scroll() {
        let mut container = ScrollableContainer::new(100);
        container.scroll_to_bottom();
        assert_eq!(container.scroll_offset(), 99);

        // Reduce total lines, scroll should adjust
        container.set_total_lines(50);
        assert_eq!(container.scroll_offset(), 49);
    }

    #[test]
    fn test_set_scroll_offset() {
        let mut container = ScrollableContainer::new(100);

        container.set_scroll_offset(50);
        assert_eq!(container.scroll_offset(), 50);

        // Should clamp to max
        container.set_scroll_offset(200);
        assert_eq!(container.scroll_offset(), 99);
    }

    #[test]
    fn test_focus() {
        let mut container = ScrollableContainer::new(100);

        assert!(!container.is_focused());

        container.set_focused(true);
        assert!(container.is_focused());

        container.set_focused(false);
        assert!(!container.is_focused());
    }

    #[test]
    fn test_show_scrollbar_builder() {
        let container = ScrollableContainer::new(100).show_scrollbar(false);
        assert!(!container.show_scrollbar);
    }

    #[test]
    fn test_empty_container() {
        let container = ScrollableContainer::new(0);

        assert_eq!(container.max_scroll(), 0);
        assert_eq!(container.scroll_offset(), 0);

        let range = container.visible_range(10);
        assert!(range.is_empty());
    }

    #[test]
    fn test_visible_range_clamps_to_total() {
        let mut container = ScrollableContainer::new(15);
        container.scroll_down(10);

        // Viewport of 10 lines starting at offset 10 should end at 15, not 20
        let range = container.visible_range(10);
        assert_eq!(range.start, 10);
        assert_eq!(range.end, 15);
    }

    #[test]
    fn test_default() {
        let container: ScrollableContainer = Default::default();

        assert_eq!(container.total_lines, 0);
        assert_eq!(container.scroll_offset(), 0);
        assert!(container.show_scrollbar);
        assert!(!container.is_focused());
    }
}
