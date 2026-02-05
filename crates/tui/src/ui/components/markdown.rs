//! Markdown renderer for TUI display.
//!
//! Renders markdown content to Ratatui text using `pulldown-cmark`.
//! Useful for help documentation and user guides.
//!
//! # Example
//!
//! ```rust,ignore
//! use splunk_tui::ui::components::MarkdownRenderer;
//! use splunk_config::Theme;
//!
//! let theme = Theme::default();
//! let renderer = MarkdownRenderer::new(theme);
//!
//! let markdown = "# Heading\n\nSome **bold** text.";
//! let text = renderer.render(markdown);
//!
//! // Use the text in a paragraph
//! let paragraph = Paragraph::new(text);
//! ```

use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};
use splunk_config::Theme;

use crate::ui::theme::ThemeExt;

/// Renders markdown content to Ratatui text.
#[derive(Debug, Clone)]
pub struct MarkdownRenderer {
    theme: Theme,
}

impl MarkdownRenderer {
    /// Create a new markdown renderer with the given theme.
    pub fn new(theme: Theme) -> Self {
        Self { theme }
    }

    /// Get the theme.
    pub fn theme(&self) -> &Theme {
        &self.theme
    }

    /// Set the theme.
    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    /// Render markdown string to Ratatui Text.
    pub fn render(&self, markdown: &str) -> Text<'static> {
        let parser = Parser::new(markdown);
        let mut lines: Vec<Line> = Vec::new();
        let mut current_spans: Vec<Span> = Vec::new();
        let mut current_style = Style::default().fg(self.theme.text);
        let mut in_code_block = false;
        let mut code_buffer = String::new();
        let mut code_language: Option<String> = None;
        let mut in_list = false;
        let mut list_indent = 0usize;

        for event in parser {
            match event {
                Event::Start(tag) => {
                    match tag {
                        Tag::Strong => {
                            current_style = current_style.add_modifier(Modifier::BOLD);
                        }
                        Tag::Emphasis => {
                            current_style = current_style.add_modifier(Modifier::ITALIC);
                        }
                        Tag::Strikethrough => {
                            current_style = current_style.add_modifier(Modifier::CROSSED_OUT);
                        }
                        Tag::Link { .. } => {
                            current_style = Style::default()
                                .fg(self.theme.accent)
                                .add_modifier(Modifier::UNDERLINED);
                        }
                        Tag::List(start_num) => {
                            in_list = true;
                            list_indent = list_indent.saturating_add(2);
                            // Store the start number for ordered lists
                            let _ = start_num;
                        }
                        Tag::Item => {
                            // Will be handled when rendering text
                        }
                        Tag::CodeBlock(lang) => {
                            in_code_block = true;
                            code_language = match lang {
                                pulldown_cmark::CodeBlockKind::Fenced(lang_str) => {
                                    Some(lang_str.to_string())
                                }
                                pulldown_cmark::CodeBlockKind::Indented => None,
                            };
                            code_buffer.clear();
                        }
                        Tag::Heading { level, .. } => {
                            current_style = match level {
                                pulldown_cmark::HeadingLevel::H1 => {
                                    self.theme.title().add_modifier(Modifier::UNDERLINED)
                                }
                                pulldown_cmark::HeadingLevel::H2 => self.theme.title(),
                                _ => Style::default()
                                    .fg(self.theme.text)
                                    .add_modifier(Modifier::BOLD),
                            };
                        }
                        _ => {}
                    }
                }
                Event::End(tag_end) => {
                    match tag_end {
                        TagEnd::Strong => {
                            current_style = current_style.remove_modifier(Modifier::BOLD);
                        }
                        TagEnd::Emphasis => {
                            current_style = current_style.remove_modifier(Modifier::ITALIC);
                        }
                        TagEnd::Strikethrough => {
                            current_style = current_style.remove_modifier(Modifier::CROSSED_OUT);
                        }
                        TagEnd::Link => {
                            current_style = Style::default().fg(self.theme.text);
                        }
                        TagEnd::CodeBlock => {
                            in_code_block = false;
                            // Render code block
                            if !code_buffer.is_empty() {
                                // Add a blank line before code block if there's previous content
                                if !lines.is_empty() && !current_spans.is_empty() {
                                    lines.push(Line::from(current_spans.clone()));
                                    current_spans.clear();
                                }

                                // Language indicator if available
                                if let Some(ref lang) = code_language {
                                    lines.push(Line::from(vec![Span::styled(
                                        format!("// {}", lang),
                                        Style::default()
                                            .fg(self.theme.text_dim)
                                            .add_modifier(Modifier::ITALIC),
                                    )]));
                                }

                                // Render code block lines
                                for code_line in code_buffer.lines() {
                                    lines.push(Line::from(vec![Span::styled(
                                        code_line.to_string(),
                                        Style::default()
                                            .bg(Color::DarkGray)
                                            .fg(self.theme.syntax_string),
                                    )]));
                                }

                                code_buffer.clear();
                                code_language = None;
                            }
                        }
                        TagEnd::Paragraph => {
                            if !current_spans.is_empty() {
                                lines.push(Line::from(current_spans.clone()));
                                current_spans.clear();
                            }
                            lines.push(Line::from(""));
                        }
                        TagEnd::Heading(_) => {
                            if !current_spans.is_empty() {
                                lines.push(Line::from(current_spans.clone()));
                                current_spans.clear();
                            }
                            lines.push(Line::from(""));
                            current_style = Style::default().fg(self.theme.text);
                        }
                        TagEnd::List(_) => {
                            in_list = false;
                            list_indent = list_indent.saturating_sub(2);
                        }
                        TagEnd::Item => {
                            // End of list item - push the line
                            if !current_spans.is_empty() {
                                // Add indentation for nested lists
                                let indent = "  ".repeat(list_indent / 2);
                                let mut spans_with_indent = vec![Span::raw(indent)];
                                spans_with_indent.extend(current_spans.clone());
                                lines.push(Line::from(spans_with_indent));
                                current_spans.clear();
                            }
                        }
                        _ => {}
                    }
                }
                Event::Text(text) => {
                    if in_code_block {
                        code_buffer.push_str(&text);
                    } else {
                        // Handle list item markers
                        let text_str = text.to_string();
                        if in_list && text_str.starts_with("- ") {
                            let marker = Span::styled("• ", Style::default().fg(self.theme.accent));
                            current_spans.push(marker);
                            current_spans
                                .push(Span::styled(text_str[2..].to_string(), current_style));
                        } else if in_list {
                            // Check for ordered list markers like "1. "
                            let chars: Vec<char> = text_str.chars().collect();
                            if chars.len() >= 3
                                && chars[0].is_ascii_digit()
                                && chars[1] == '.'
                                && chars[2] == ' '
                            {
                                let marker = Span::styled(
                                    format!("{} ", &text_str[..2]),
                                    Style::default().fg(self.theme.accent),
                                );
                                current_spans.push(marker);
                                current_spans
                                    .push(Span::styled(text_str[3..].to_string(), current_style));
                            } else {
                                current_spans.push(Span::styled(text_str, current_style));
                            }
                        } else {
                            current_spans.push(Span::styled(text_str, current_style));
                        }
                    }
                }
                Event::Code(code) => {
                    current_spans.push(Span::styled(
                        code.to_string(),
                        Style::default()
                            .bg(Color::DarkGray)
                            .fg(self.theme.syntax_string),
                    ));
                }
                Event::Html(html) => {
                    // Render HTML as plain text for now
                    current_spans.push(Span::styled(html.to_string(), current_style));
                }
                Event::HardBreak => {
                    lines.push(Line::from(current_spans.clone()));
                    current_spans.clear();
                }
                Event::SoftBreak => {
                    // Soft breaks are converted to spaces in most markdown renderers
                    current_spans.push(Span::styled(" ", current_style));
                }
                Event::Rule => {
                    lines.push(Line::from("─".repeat(40)).style(self.theme.text_dim()));
                    lines.push(Line::from(""));
                }
                _ => {}
            }
        }

        // Add any remaining spans
        if !current_spans.is_empty() {
            lines.push(Line::from(current_spans));
        }

        Text::from(lines)
    }

    /// Render markdown with a specific heading style.
    pub fn render_with_heading_style(&self, markdown: &str, heading_style: Style) -> Text<'static> {
        let mut text = self.render(markdown);

        // Post-process to apply heading styles
        for line in &mut text.lines {
            if let Some(first_span) = line.spans.first() {
                let content = first_span.content.to_string();
                if content.starts_with("# ")
                    || content.starts_with("## ")
                    || content.starts_with("### ")
                {
                    for span in &mut line.spans {
                        span.style = heading_style;
                    }
                }
            }
        }

        text
    }

    /// Render a simple inline code snippet.
    pub fn render_code(&self, code: &str, language: Option<&str>) -> Text<'static> {
        let mut lines = Vec::new();

        if let Some(lang) = language {
            lines.push(Line::from(vec![Span::styled(
                format!("// {}", lang),
                Style::default()
                    .fg(self.theme.text_dim)
                    .add_modifier(Modifier::ITALIC),
            )]));
        }

        for code_line in code.lines() {
            lines.push(Line::from(vec![Span::styled(
                code_line.to_string(),
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(self.theme.syntax_string),
            )]));
        }

        Text::from(lines)
    }
}

/// Convenience function to render markdown to text.
pub fn render_markdown(markdown: &str, theme: &Theme) -> Text<'static> {
    let renderer = MarkdownRenderer::new(*theme);
    renderer.render(markdown)
}

/// Render a simple help text with basic formatting.
pub fn render_help_text(content: &str, theme: &Theme) -> Text<'static> {
    let mut lines = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            lines.push(Line::from(""));
        } else if let Some(stripped) = trimmed.strip_prefix("# ") {
            // H1
            lines.push(Line::from(vec![Span::styled(
                stripped.to_string(),
                theme.title(),
            )]));
        } else if let Some(stripped) = trimmed.strip_prefix("## ") {
            // H2
            lines.push(Line::from(vec![Span::styled(
                stripped.to_string(),
                theme.title().add_modifier(Modifier::UNDERLINED),
            )]));
        } else if let Some(stripped) = trimmed.strip_prefix("### ") {
            // H3
            lines.push(Line::from(vec![Span::styled(
                stripped.to_string(),
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            )]));
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            // List item
            lines.push(Line::from(vec![
                Span::styled("  • ", Style::default().fg(theme.accent)),
                Span::styled(trimmed[2..].to_string(), Style::default().fg(theme.text)),
            ]));
        } else if trimmed.starts_with("`") && trimmed.ends_with("`") && trimmed.len() > 2 {
            // Inline code
            lines.push(Line::from(vec![Span::styled(
                trimmed.to_string(),
                Style::default().bg(Color::DarkGray).fg(theme.syntax_string),
            )]));
        } else {
            lines.push(Line::from(vec![Span::styled(
                line.to_string(),
                Style::default().fg(theme.text),
            )]));
        }
    }

    Text::from(lines)
}

#[cfg(test)]
mod tests {
    use super::*;
    use splunk_config::{ColorTheme, Theme};

    fn test_theme() -> Theme {
        Theme::from_color_theme(ColorTheme::Default)
    }

    #[test]
    fn test_markdown_renderer_creation() {
        let theme = test_theme();
        let renderer = MarkdownRenderer::new(theme);

        assert_eq!(renderer.theme().text, theme.text);
    }

    #[test]
    fn test_markdown_simple_text() {
        let theme = test_theme();
        let renderer = MarkdownRenderer::new(theme);
        let text = renderer.render("Hello world");

        assert!(!text.lines.is_empty());
    }

    #[test]
    fn test_markdown_heading() {
        let theme = test_theme();
        let renderer = MarkdownRenderer::new(theme);
        let text = renderer.render("# Heading 1\n\n## Heading 2");

        // Should have lines for headings
        assert!(!text.lines.is_empty());
    }

    #[test]
    fn test_markdown_bold_text() {
        let theme = test_theme();
        let renderer = MarkdownRenderer::new(theme);
        let text = renderer.render("This is **bold** text");

        assert!(!text.lines.is_empty());
    }

    #[test]
    fn test_markdown_italic_text() {
        let theme = test_theme();
        let renderer = MarkdownRenderer::new(theme);
        let text = renderer.render("This is *italic* text");

        assert!(!text.lines.is_empty());
    }

    #[test]
    fn test_markdown_inline_code() {
        let theme = test_theme();
        let renderer = MarkdownRenderer::new(theme);
        let text = renderer.render("Use `search` command");

        assert!(!text.lines.is_empty());
    }

    #[test]
    fn test_markdown_code_block() {
        let theme = test_theme();
        let renderer = MarkdownRenderer::new(theme);
        let markdown = r#"```rust
fn main() {
    println!("Hello");
}
```"#;
        let text = renderer.render(markdown);

        assert!(!text.lines.is_empty());
    }

    #[test]
    fn test_markdown_list() {
        let theme = test_theme();
        let renderer = MarkdownRenderer::new(theme);
        let markdown = "- Item 1\n- Item 2\n- Item 3";
        let text = renderer.render(markdown);

        assert!(!text.lines.is_empty());
    }

    #[test]
    fn test_markdown_ordered_list() {
        let theme = test_theme();
        let renderer = MarkdownRenderer::new(theme);
        let markdown = "1. First\n2. Second\n3. Third";
        let text = renderer.render(markdown);

        assert!(!text.lines.is_empty());
    }

    #[test]
    fn test_markdown_link() {
        let theme = test_theme();
        let renderer = MarkdownRenderer::new(theme);
        let text = renderer.render("[Link text](https://example.com)");

        assert!(!text.lines.is_empty());
    }

    #[test]
    fn test_markdown_horizontal_rule() {
        let theme = test_theme();
        let renderer = MarkdownRenderer::new(theme);
        let text = renderer.render("Text\n\n---\n\nMore text");

        assert!(!text.lines.is_empty());
    }

    #[test]
    fn test_markdown_empty() {
        let theme = test_theme();
        let renderer = MarkdownRenderer::new(theme);
        let text = renderer.render("");

        assert!(text.lines.is_empty());
    }

    #[test]
    fn test_render_markdown_function() {
        let theme = test_theme();
        let text = render_markdown("# Test\n\nHello", &theme);

        assert!(!text.lines.is_empty());
    }

    #[test]
    fn test_render_help_text() {
        let theme = test_theme();
        let text = render_help_text("# Help\n\n- Item 1\n- Item 2", &theme);

        assert!(!text.lines.is_empty());
    }

    #[test]
    fn test_render_help_text_with_headings() {
        let theme = test_theme();
        let help = r#"# Main Title

## Section

Some text here.

### Subsection

- Bullet 1
- Bullet 2

`code` example"#;

        let text = render_help_text(help, &theme);
        assert!(!text.lines.is_empty());
    }

    #[test]
    fn test_render_code() {
        let theme = test_theme();
        let renderer = MarkdownRenderer::new(theme);
        let text = renderer.render_code("fn main() {}", Some("rust"));

        assert!(!text.lines.is_empty());
    }

    #[test]
    fn test_markdown_strikethrough() {
        let theme = test_theme();
        let renderer = MarkdownRenderer::new(theme);
        let text = renderer.render("This is ~~deleted~~ text");

        assert!(!text.lines.is_empty());
    }

    #[test]
    fn test_markdown_mixed_formatting() {
        let theme = test_theme();
        let renderer = MarkdownRenderer::new(theme);
        let markdown = "# Title\n\nThis has **bold** and *italic* and `code`.";
        let text = renderer.render(markdown);

        assert!(!text.lines.is_empty());
        assert!(text.lines.len() >= 2); // Title + content
    }

    #[test]
    fn test_markdown_multiline_code_block() {
        let theme = test_theme();
        let renderer = MarkdownRenderer::new(theme);
        let markdown = r#"```python
def hello():
    print("Hello")
    return True
```"#;
        let text = renderer.render(markdown);

        // Should have multiple lines for the code block
        assert!(!text.lines.is_empty());
    }

    #[test]
    fn test_renderer_set_theme() {
        let theme1 = Theme::from_color_theme(ColorTheme::Default);
        let theme2 = Theme::from_color_theme(ColorTheme::Dark);

        let mut renderer = MarkdownRenderer::new(theme1);
        renderer.set_theme(theme2);

        assert_eq!(renderer.theme().background, theme2.background);
    }

    #[test]
    fn test_render_with_heading_style() {
        let theme = test_theme();
        let renderer = MarkdownRenderer::new(theme);
        let custom_style = Style::default().fg(Color::Red);

        let text = renderer.render_with_heading_style("# Test", custom_style);

        assert!(!text.lines.is_empty());
    }
}
