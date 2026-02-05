//! Line number widget with diagnostic support.
//!
//! Provides a line number gutter with optional diagnostic highlighting
//! for features like SPL query validation feedback.
//!
//! # Example
//!
//! ```rust,ignore
//! use splunk_tui::ui::components::{LineNumberWidget, Diagnostic, DiagnosticSeverity};
//!
//! let content = "search index=main\n| stats count()\n| table count";
//! let mut widget = LineNumberWidget::new(content);
//!
//! // Add a diagnostic for line 2
//! widget.add_diagnostic(2, Diagnostic {
//!     severity: DiagnosticSeverity::Warning,
//!     message: "Deprecated function".to_string(),
//!     column: Some(8),
//! });
//!
//! frame.render_widget(widget, area);
//! ```

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};
use std::collections::HashMap;

/// Diagnostic severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticSeverity {
    /// Error - critical issue that prevents execution.
    Error,
    /// Warning - potential issue that should be addressed.
    Warning,
    /// Info - informational message.
    Info,
    /// Hint - suggestion for improvement.
    Hint,
}

impl DiagnosticSeverity {
    /// Get the color associated with this severity level.
    pub fn color(&self) -> Color {
        match self {
            DiagnosticSeverity::Error => Color::Red,
            DiagnosticSeverity::Warning => Color::Yellow,
            DiagnosticSeverity::Info => Color::Blue,
            DiagnosticSeverity::Hint => Color::Gray,
        }
    }

    /// Get the indicator character for this severity level.
    pub fn indicator(&self) -> &'static str {
        match self {
            DiagnosticSeverity::Error => "✗",
            DiagnosticSeverity::Warning => "▲",
            DiagnosticSeverity::Info => "ℹ",
            DiagnosticSeverity::Hint => "➤",
        }
    }

    /// Get the priority of this severity (lower = higher priority).
    fn priority(&self) -> u8 {
        match self {
            DiagnosticSeverity::Error => 0,
            DiagnosticSeverity::Warning => 1,
            DiagnosticSeverity::Info => 2,
            DiagnosticSeverity::Hint => 3,
        }
    }
}

/// A diagnostic message for a specific line.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Severity level of the diagnostic.
    pub severity: DiagnosticSeverity,
    /// Human-readable message.
    pub message: String,
    /// Optional column position (0-indexed).
    pub column: Option<usize>,
}

impl Diagnostic {
    /// Create a new diagnostic with the given severity and message.
    pub fn new(severity: DiagnosticSeverity, message: impl Into<String>) -> Self {
        Self {
            severity,
            message: message.into(),
            column: None,
        }
    }

    /// Set the column position for this diagnostic.
    pub fn with_column(mut self, column: usize) -> Self {
        self.column = Some(column);
        self
    }
}

/// Widget that renders content with line numbers and diagnostic indicators.
#[derive(Debug, Clone)]
pub struct LineNumberWidget<'a> {
    content: &'a str,
    line_number_fg: Color,
    content_fg: Color,
    diagnostics: HashMap<usize, Vec<Diagnostic>>,
    show_line_numbers: bool,
    start_line: usize,
    focused: bool,
}

impl<'a> LineNumberWidget<'a> {
    /// Create a new line number widget for the given content.
    pub fn new(content: &'a str) -> Self {
        Self {
            content,
            line_number_fg: Color::DarkGray,
            content_fg: Color::White,
            diagnostics: HashMap::new(),
            show_line_numbers: true,
            start_line: 1,
            focused: false,
        }
    }

    /// Set the foreground color for line numbers.
    pub fn line_number_color(mut self, color: Color) -> Self {
        self.line_number_fg = color;
        self
    }

    /// Set the foreground color for content.
    pub fn content_color(mut self, color: Color) -> Self {
        self.content_fg = color;
        self
    }

    /// Set whether to show line numbers.
    pub fn show_line_numbers(mut self, show: bool) -> Self {
        self.show_line_numbers = show;
        self
    }

    /// Set the starting line number (for partial views).
    pub fn start_line(mut self, start: usize) -> Self {
        self.start_line = start;
        self
    }

    /// Set whether the widget is focused.
    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    /// Check if the widget is focused.
    pub fn is_focused(&self) -> bool {
        self.focused
    }

    /// Add a diagnostic for a specific line (1-indexed).
    pub fn add_diagnostic(&mut self, line: usize, diagnostic: Diagnostic) {
        self.diagnostics.entry(line).or_default().push(diagnostic);
    }

    /// Set all diagnostics at once.
    pub fn with_diagnostics(mut self, diagnostics: HashMap<usize, Vec<Diagnostic>>) -> Self {
        self.diagnostics = diagnostics;
        self
    }

    /// Get diagnostics for a specific line.
    pub fn get_diagnostics(&self, line: usize) -> Option<&Vec<Diagnostic>> {
        self.diagnostics.get(&line)
    }

    /// Check if a line has any diagnostics.
    pub fn has_diagnostics(&self, line: usize) -> bool {
        self.diagnostics.contains_key(&line)
    }

    /// Get the highest severity diagnostic for a line (if any).
    fn get_highest_severity(&self, line_num: usize) -> Option<DiagnosticSeverity> {
        self.diagnostics.get(&line_num).map(|diags| {
            diags
                .iter()
                .map(|d| d.severity)
                .min_by_key(|s| s.priority())
                .unwrap_or(DiagnosticSeverity::Hint)
        })
    }

    /// Get the style and indicator for a line based on diagnostics.
    fn get_line_style(&self, line_num: usize) -> (Style, &'static str) {
        match self.get_highest_severity(line_num) {
            Some(severity) => {
                let color = severity.color();
                (Style::default().fg(color), severity.indicator())
            }
            None => (Style::default().fg(self.line_number_fg), " "),
        }
    }

    /// Calculate the width needed for line numbers.
    fn line_number_width(&self, total_lines: usize) -> usize {
        let max_line = self.start_line + total_lines - 1;
        max_line.to_string().len().max(3) + 2 // +2 for indicator and space
    }
}

impl<'a> Widget for LineNumberWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let lines: Vec<&str> = self.content.lines().collect();

        if lines.is_empty() || area.height == 0 {
            return;
        }

        let num_width = self.line_number_width(lines.len());

        for (i, line) in lines.iter().enumerate().take(area.height as usize) {
            let line_num = i + self.start_line;
            let y = area.y + i as u16;

            let (style, indicator) = self.get_line_style(line_num);

            if self.show_line_numbers {
                let num_str = format!("{:>width$} {}", line_num, indicator, width = num_width - 2);

                // Render line number
                buf.set_stringn(area.x, y, &num_str, num_str.len(), style);

                // Render content
                let content_x = area.x + num_str.len() as u16;
                let content_width = area.width.saturating_sub(num_str.len() as u16);

                if content_width > 0 {
                    let visible_line = if line.len() > content_width as usize {
                        &line[..content_width as usize]
                    } else {
                        line
                    };

                    buf.set_stringn(
                        content_x,
                        y,
                        visible_line,
                        content_width as usize,
                        Style::default().fg(self.content_fg),
                    );
                }
            } else {
                // Just render content without line numbers
                let visible_line = if line.len() > area.width as usize {
                    &line[..area.width as usize]
                } else {
                    line
                };

                buf.set_stringn(
                    area.x,
                    y,
                    visible_line,
                    area.width as usize,
                    Style::default().fg(self.content_fg),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_number_widget_creation() {
        let content = "line 1\nline 2\nline 3\n";
        let widget = LineNumberWidget::new(content);

        assert!(widget.show_line_numbers);
        assert_eq!(widget.start_line, 1);
        assert!(!widget.is_focused());
    }

    #[test]
    fn test_line_number_widget_builder() {
        let widget = LineNumberWidget::new("test")
            .line_number_color(Color::Blue)
            .content_color(Color::Green)
            .show_line_numbers(false)
            .start_line(10);

        assert!(!widget.show_line_numbers);
        assert_eq!(widget.start_line, 10);
        assert_eq!(widget.line_number_fg, Color::Blue);
        assert_eq!(widget.content_fg, Color::Green);
    }

    #[test]
    fn test_diagnostic_severity() {
        assert_eq!(DiagnosticSeverity::Error.color(), Color::Red);
        assert_eq!(DiagnosticSeverity::Warning.color(), Color::Yellow);
        assert_eq!(DiagnosticSeverity::Info.color(), Color::Blue);
        assert_eq!(DiagnosticSeverity::Hint.color(), Color::Gray);

        assert_eq!(DiagnosticSeverity::Error.indicator(), "✗");
        assert_eq!(DiagnosticSeverity::Warning.indicator(), "▲");
        assert_eq!(DiagnosticSeverity::Info.indicator(), "ℹ");
        assert_eq!(DiagnosticSeverity::Hint.indicator(), "➤");
    }

    #[test]
    fn test_diagnostic_severity_priority() {
        assert_eq!(DiagnosticSeverity::Error.priority(), 0);
        assert_eq!(DiagnosticSeverity::Warning.priority(), 1);
        assert_eq!(DiagnosticSeverity::Info.priority(), 2);
        assert_eq!(DiagnosticSeverity::Hint.priority(), 3);
    }

    #[test]
    fn test_diagnostic_creation() {
        let diag = Diagnostic::new(DiagnosticSeverity::Error, "Test error");
        assert_eq!(diag.severity, DiagnosticSeverity::Error);
        assert_eq!(diag.message, "Test error");
        assert_eq!(diag.column, None);

        let diag_with_col =
            Diagnostic::new(DiagnosticSeverity::Warning, "Test warning").with_column(5);
        assert_eq!(diag_with_col.column, Some(5));
    }

    #[test]
    fn test_add_diagnostic() {
        let mut widget = LineNumberWidget::new("line 1\nline 2\n");

        widget.add_diagnostic(
            1,
            Diagnostic::new(DiagnosticSeverity::Error, "Syntax error"),
        );

        assert!(widget.has_diagnostics(1));
        assert!(!widget.has_diagnostics(2));

        let diags = widget.get_diagnostics(1).unwrap();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].message, "Syntax error");
    }

    #[test]
    fn test_with_diagnostics() {
        let mut diagnostics = HashMap::new();
        diagnostics.insert(
            1,
            vec![Diagnostic::new(DiagnosticSeverity::Warning, "Test")],
        );

        let widget = LineNumberWidget::new("test").with_diagnostics(diagnostics);

        assert!(widget.has_diagnostics(1));
    }

    #[test]
    fn test_get_highest_severity() {
        let mut widget = LineNumberWidget::new("test");

        // Add multiple diagnostics to the same line
        widget.add_diagnostic(1, Diagnostic::new(DiagnosticSeverity::Info, "Info message"));
        widget.add_diagnostic(
            1,
            Diagnostic::new(DiagnosticSeverity::Error, "Error message"),
        );
        widget.add_diagnostic(
            1,
            Diagnostic::new(DiagnosticSeverity::Warning, "Warning message"),
        );

        // Error should be the highest priority
        let highest = widget.get_highest_severity(1);
        assert_eq!(highest, Some(DiagnosticSeverity::Error));
    }

    #[test]
    fn test_focus() {
        let mut widget = LineNumberWidget::new("test");

        assert!(!widget.is_focused());
        widget.set_focused(true);
        assert!(widget.is_focused());
        widget.set_focused(false);
        assert!(!widget.is_focused());
    }

    #[test]
    fn test_line_number_width() {
        let widget = LineNumberWidget::new("test");
        assert_eq!(widget.line_number_width(5), 5); // "    5 " (3 + 2)

        let widget_large = LineNumberWidget::new("test").start_line(100);
        assert_eq!(widget_large.line_number_width(50), 5); // "  150 " (3 + 2)
    }

    #[test]
    fn test_widget_render() {
        let content = "line 1\nline 2\n";
        let widget = LineNumberWidget::new(content);

        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 10));
        widget.render(Rect::new(0, 0, 80, 10), &mut buf);

        // Buffer should have content
        let content_str: String = buf
            .content
            .iter()
            .map(|cell| cell.symbol().to_string())
            .collect();
        assert!(!content_str.is_empty());
    }

    #[test]
    fn test_widget_render_with_diagnostics() {
        let mut widget = LineNumberWidget::new("line 1\nline 2\n");
        widget.add_diagnostic(
            1,
            Diagnostic::new(DiagnosticSeverity::Error, "Error on line 1"),
        );

        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 10));
        widget.render(Rect::new(0, 0, 80, 10), &mut buf);

        // Should render without panic
        let content_str: String = buf
            .content
            .iter()
            .map(|cell| cell.symbol().to_string())
            .collect();
        assert!(!content_str.is_empty());
    }

    #[test]
    fn test_widget_render_no_line_numbers() {
        let content = "line 1\nline 2\n";
        let widget = LineNumberWidget::new(content).show_line_numbers(false);

        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 10));
        widget.render(Rect::new(0, 0, 80, 10), &mut buf);

        // Should render without line numbers
        let content_str: String = buf
            .content
            .iter()
            .map(|cell| cell.symbol().to_string())
            .collect();
        assert!(content_str.contains("line 1"));
    }

    #[test]
    fn test_widget_render_empty_content() {
        let widget = LineNumberWidget::new("");

        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 10));
        widget.render(Rect::new(0, 0, 80, 10), &mut buf);

        // Should handle empty content gracefully
    }

    #[test]
    fn test_widget_render_zero_height() {
        let widget = LineNumberWidget::new("line 1\n");

        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 0));
        widget.render(Rect::new(0, 0, 80, 0), &mut buf);

        // Should handle zero height gracefully
    }
}
