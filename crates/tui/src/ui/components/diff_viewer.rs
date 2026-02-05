//! Diff viewer component.
//!
//! A unified diff viewer for comparing two text sources.
//! Uses the `similar` crate for computing diffs.
//!
//! # Example
//!
//! ```rust,ignore
//! use splunk_tui::ui::components::DiffViewer;
//!
//! let old = "line 1\nline 2\n";
//! let new = "line 1\nmodified line\n";
//! let viewer = DiffViewer::new(old, new);
//!
//! // Render the diff
//! frame.render_widget(viewer, area);
//! ```

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Widget,
};
use similar::{ChangeTag, TextDiff};
use splunk_config::Theme;

/// A diff viewer component for comparing two text sources.
#[derive(Debug, Clone)]
pub struct DiffViewer<'a> {
    old: &'a str,
    new: &'a str,
    show_line_numbers: bool,
    context_lines: usize,
    focused: bool,
}

impl<'a> DiffViewer<'a> {
    /// Create a new diff viewer with the given old and new content.
    pub fn new(old: &'a str, new: &'a str) -> Self {
        Self {
            old,
            new,
            show_line_numbers: true,
            context_lines: 3,
            focused: false,
        }
    }

    /// Set whether to show line numbers.
    pub fn show_line_numbers(mut self, show: bool) -> Self {
        self.show_line_numbers = show;
        self
    }

    /// Set the number of context lines around changes.
    pub fn context_lines(mut self, lines: usize) -> Self {
        self.context_lines = lines;
        self
    }

    /// Set whether the viewer is focused.
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    /// Check if the viewer is focused.
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// Compute and render the diff to lines.
    fn render_diff(&self, theme: &Theme) -> Vec<Line<'_>> {
        let _ = self.context_lines; // Acknowledge field for future use
        let diff = TextDiff::from_lines(self.old, self.new);
        let mut lines = Vec::new();
        let mut old_line_num = 1;
        let mut new_line_num = 1;

        for change in diff.iter_all_changes() {
            let (prefix, style, old_num, new_num) = match change.tag() {
                ChangeTag::Delete => (
                    "-",
                    Style::default().fg(theme.error),
                    Some(old_line_num),
                    None,
                ),
                ChangeTag::Insert => (
                    "+",
                    Style::default().fg(theme.success),
                    None,
                    Some(new_line_num),
                ),
                ChangeTag::Equal => (
                    " ",
                    Style::default().fg(theme.text_dim),
                    Some(old_line_num),
                    Some(new_line_num),
                ),
            };

            let line_num_str = if self.show_line_numbers {
                match (old_num, new_num) {
                    (Some(o), Some(n)) => format!("{:>4} {:>4} ", o, n),
                    (Some(o), None) => format!("{:>4}     ", o),
                    (None, Some(n)) => format!("    {:>4} ", n),
                    (None, None) => "         ".to_string(),
                }
            } else {
                String::new()
            };

            lines.push(Line::from(vec![
                Span::styled(format!("{}{}", prefix, line_num_str), style),
                Span::styled(change.value().trim_end().to_string(), style),
            ]));

            if change.tag() != ChangeTag::Insert {
                old_line_num += 1;
            }
            if change.tag() != ChangeTag::Delete {
                new_line_num += 1;
            }
        }

        lines
    }
}

impl<'a> Widget for DiffViewer<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let theme = Theme::default();
        let lines = self.render_diff(&theme);

        for (i, line) in lines.iter().enumerate().take(area.height as usize) {
            buf.set_line(area.x, area.y + i as u16, line, area.width);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_viewer_creation() {
        let old = "line 1\nline 2\n";
        let new = "line 1\nmodified line\n";
        let viewer = DiffViewer::new(old, new);

        assert!(viewer.show_line_numbers);
        assert_eq!(viewer.context_lines, 3);
        assert!(!viewer.is_focused());
    }

    #[test]
    fn test_diff_viewer_builder() {
        let viewer = DiffViewer::new("a", "b")
            .show_line_numbers(false)
            .context_lines(5);

        assert!(!viewer.show_line_numbers);
        assert_eq!(viewer.context_lines, 5);
    }

    #[test]
    fn test_diff_viewer_focus() {
        let mut viewer = DiffViewer::new("a", "b");

        assert!(!viewer.is_focused());
        viewer.set_focused(true);
        assert!(viewer.is_focused());
        viewer.set_focused(false);
        assert!(!viewer.is_focused());
    }

    #[test]
    fn test_diff_viewer_rendering() {
        let old = "line 1\nline 2\nline 3\n";
        let new = "line 1\nmodified line\nline 3\n";
        let viewer = DiffViewer::new(old, new);

        let theme = Theme::default();
        let lines = viewer.render_diff(&theme);

        // Should have lines for the diff output
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_diff_viewer_additions() {
        let old = "line 1\n";
        let new = "line 1\nline 2\n";
        let viewer = DiffViewer::new(old, new);

        let theme = Theme::default();
        let lines = viewer.render_diff(&theme);

        // Should have a line starting with "+" for the addition
        let has_addition = lines.iter().any(|line| {
            line.spans
                .first()
                .map(|s| s.content.starts_with('+'))
                .unwrap_or(false)
        });
        assert!(has_addition, "Diff should show an addition");
    }

    #[test]
    fn test_diff_viewer_deletions() {
        let old = "line 1\nline 2\n";
        let new = "line 1\n";
        let viewer = DiffViewer::new(old, new);

        let theme = Theme::default();
        let lines = viewer.render_diff(&theme);

        // Should have a line starting with "-" for the deletion
        let has_deletion = lines.iter().any(|line| {
            line.spans
                .first()
                .map(|s| s.content.starts_with('-'))
                .unwrap_or(false)
        });
        assert!(has_deletion, "Diff should show a deletion");
    }

    #[test]
    fn test_diff_viewer_no_line_numbers() {
        let old = "line 1\n";
        let new = "line 1\nline 2\n";
        let viewer = DiffViewer::new(old, new).show_line_numbers(false);

        let theme = Theme::default();
        let lines = viewer.render_diff(&theme);

        // Lines should only have prefix, no line numbers
        for line in lines {
            if let Some(first_span) = line.spans.first() {
                // Should just be "+" or " " followed by content, no numbers
                assert!(
                    first_span.content.len() < 3,
                    "Without line numbers, first span should be short"
                );
            }
        }
    }

    #[test]
    fn test_diff_viewer_widget_render() {
        let old = "line 1\nline 2\n";
        let new = "line 1\nmodified\n";
        let viewer = DiffViewer::new(old, new);

        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 10));
        viewer.render(Rect::new(0, 0, 80, 10), &mut buf);

        // Buffer should have some content
        let content: String = buf
            .content
            .iter()
            .map(|cell| cell.symbol().to_string())
            .collect();
        assert!(!content.is_empty());
    }
}
