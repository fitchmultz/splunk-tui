//! SPL (Splunk Processing Language) syntax highlighting.
//!
//! Provides functions to tokenize and style SPL queries for TUI display.
//!
//! This module includes both a regex-based highlighter (current default) and
//! infrastructure for tree-sitter based highlighting when a grammar is available.

use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span, Text},
};
use splunk_config::Theme;
use std::sync::LazyLock;
use tree_sitter::{Parser, Query, QueryCursor};

/// Syntax highlighter using tree-sitter for accurate parsing.
///
/// Note: This requires a tree-sitter grammar for SPL. Since `tree-sitter-spl`
/// is not currently available as a published crate, this serves as infrastructure
/// for future enhancement. The regex-based `highlight_spl()` function remains
/// the primary implementation.
pub struct SyntaxHighlighter {
    parser: Parser,
    query: Option<Query>,
}

/// Token types for syntax highlighting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenType {
    /// SPL commands like `search`, `stats`, `eval`
    Command,
    /// Boolean operators like `AND`, `OR`, `NOT`
    Operator,
    /// Statistical functions like `count`, `avg`, `sum`
    Function,
    /// String literals
    String,
    /// Numeric literals
    Number,
    /// Comments (backtick-style in SPL)
    Comment,
    /// Pipe character `|`
    Pipe,
    /// Comparison operators like `=`, `!=`, `>`, `<`
    Comparison,
    /// Punctuation like `(`, `)`, `[`, `]`, `,`
    Punctuation,
    /// Macro references like `my_macro`
    Macro,
    /// Default/unrecognized tokens
    Default,
}

impl SyntaxHighlighter {
    /// Create a new syntax highlighter.
    ///
    /// Attempts to initialize tree-sitter parser. If no SPL grammar is available,
    /// the parser will be initialized but queries will fail gracefully.
    pub fn new() -> anyhow::Result<Self> {
        let parser = Parser::new();

        // Note: When tree-sitter-spl becomes available, initialize it here:
        // parser.set_language(&tree_sitter_spl::LANGUAGE.into())?;
        // let query = Query::new(
        //     &tree_sitter_spl::LANGUAGE.into(),
        //     include_str!("spl_highlight_query.scm")
        // )?;

        // For now, return without query since no SPL grammar is available
        Ok(Self {
            parser,
            query: None,
        })
    }

    /// Check if tree-sitter highlighting is available.
    pub fn is_available(&self) -> bool {
        self.query.is_some()
    }

    /// Highlight code using tree-sitter.
    ///
    /// Returns a vector of (style, text) tuples for ratatui rendering.
    /// Falls back to regex-based highlighting if tree-sitter is not available.
    pub fn highlight(&mut self, code: &str, theme: &Theme) -> Vec<(Style, String)> {
        // If tree-sitter is not available, fall back to regex-based highlighting
        if self.query.is_none() {
            return self.highlight_fallback(code, theme);
        }

        let query = self.query.as_ref().unwrap();

        // Attempt to parse - if it fails, fall back
        let tree = match self.parser.parse(code, None) {
            Some(tree) => tree,
            None => return self.highlight_fallback(code, theme),
        };

        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(query, tree.root_node(), code.as_bytes());

        let mut results = Vec::new();
        let mut last_end = 0;

        for m in matches {
            for capture in m.captures {
                let node = capture.node;
                let start = node.start_byte();
                let end = node.end_byte();

                // Add any text between last capture and this one
                if start > last_end {
                    results.push((Style::default(), code[last_end..start].to_string()));
                }

                let token_type = self.capture_to_token_type(capture.index);
                let style = self.style_for_token(token_type, theme);
                let text = code[start..end].to_string();

                results.push((style, text));
                last_end = end;
            }
        }

        // Add remaining text
        if last_end < code.len() {
            results.push((Style::default(), code[last_end..].to_string()));
        }

        results
    }

    /// Convert tree-sitter capture index to token type.
    fn capture_to_token_type(&self, capture_index: u32) -> TokenType {
        // Map capture indices to token types based on query definition
        // This would be defined when we have an actual SPL grammar
        match capture_index {
            0 => TokenType::Command,
            1 => TokenType::Function,
            2 => TokenType::String,
            3 => TokenType::Number,
            4 => TokenType::Comment,
            5 => TokenType::Operator,
            _ => TokenType::Default,
        }
    }

    /// Get style for a token type.
    fn style_for_token(&self, token_type: TokenType, theme: &Theme) -> Style {
        match token_type {
            TokenType::Command => Style::default()
                .fg(theme.syntax_command)
                .add_modifier(Modifier::BOLD),
            TokenType::Operator => Style::default()
                .fg(theme.syntax_operator)
                .add_modifier(Modifier::BOLD),
            TokenType::Function => Style::default().fg(theme.syntax_number),
            TokenType::String => Style::default().fg(theme.syntax_string),
            TokenType::Number => Style::default().fg(theme.syntax_number),
            TokenType::Comment => Style::default().fg(theme.syntax_comment),
            TokenType::Pipe => Style::default()
                .fg(theme.syntax_pipe)
                .add_modifier(Modifier::BOLD),
            TokenType::Comparison => Style::default().fg(theme.syntax_comparison),
            TokenType::Punctuation => Style::default().fg(theme.syntax_punctuation),
            TokenType::Macro => Style::default()
                .fg(theme.syntax_command)
                .add_modifier(Modifier::BOLD),
            TokenType::Default => Style::default(),
        }
    }

    /// Fallback highlighting using regex-based approach.
    fn highlight_fallback(&self, code: &str, theme: &Theme) -> Vec<(Style, String)> {
        let text = highlight_spl(code, theme);
        let mut results = Vec::new();

        for line in text.lines {
            for span in line.spans {
                results.push((span.style, span.content.to_string()));
            }
            results.push((Style::default(), "\n".to_string()));
        }

        // Remove trailing newline if present
        if results.last().map(|(_, s)| s == "\n").unwrap_or(false) {
            results.pop();
        }

        results
    }
}

impl Default for SyntaxHighlighter {
    fn default() -> Self {
        Self::new().expect("Failed to create SyntaxHighlighter")
    }
}

/// List of common SPL commands to highlight.
static SPL_COMMANDS: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    vec![
        "search",
        "stats",
        "eval",
        "table",
        "fields",
        "where",
        "dedup",
        "sort",
        "head",
        "tail",
        "rename",
        "lookup",
        "inputlookup",
        "outputlookup",
        "join",
        "append",
        "appendcols",
        "union",
        "map",
        "transaction",
        "timechart",
        "chart",
        "rare",
        "top",
        "contingency",
        "correlate",
        "eventstats",
        "streamstats",
        "accum",
        "fillnull",
        "filldown",
        "untable",
        "xyseries",
        "mstats",
        "tstats",
        "metadata",
        "dbinspect",
        "rest",
        "loadjob",
    ]
});

/// List of SPL boolean operators.
static SPL_OPERATORS: LazyLock<Vec<&'static str>> = LazyLock::new(|| vec!["AND", "OR", "NOT"]);

/// List of common SPL functions.
static SPL_FUNCTIONS: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
    vec![
        "count",
        "sum",
        "avg",
        "min",
        "max",
        "list",
        "values",
        "distinct_count",
        "dc",
        "first",
        "last",
        "median",
        "perc95",
        "perc99",
        "stddev",
        "var",
        "range",
    ]
});

/// Highlight an SPL query string.
///
/// Returns a `Text` containing styled `Line`s.
pub fn highlight_spl(input: &str, theme: &Theme) -> Text<'static> {
    let mut lines = Vec::new();
    let mut current_line_spans = Vec::new();
    let mut chars = input.char_indices().peekable();
    let mut current_token = String::new();

    while let Some((_idx, c)) = chars.next() {
        match c {
            // Pipes
            '|' => {
                push_token(&mut current_line_spans, &mut current_token, theme);
                current_line_spans.push(Span::styled(
                    "|",
                    Style::default()
                        .fg(theme.syntax_pipe)
                        .add_modifier(Modifier::BOLD),
                ));
            }
            // Comparison operators
            '=' | '!' | '>' | '<' => {
                push_token(&mut current_line_spans, &mut current_token, theme);
                let mut op = c.to_string();
                if matches!(chars.peek(), Some((_, '='))) {
                    op.push('=');
                    chars.next();
                }
                current_line_spans.push(Span::styled(
                    op,
                    Style::default().fg(theme.syntax_comparison),
                ));
            }
            // Punctuation
            '(' | ')' | '[' | ']' | ',' => {
                push_token(&mut current_line_spans, &mut current_token, theme);
                current_line_spans.push(Span::styled(
                    c.to_string(),
                    Style::default().fg(theme.syntax_punctuation),
                ));
            }
            // Comments and Macros
            '`' => {
                push_token(&mut current_line_spans, &mut current_token, theme);
                let mut content = "`".to_string();

                if matches!(chars.peek(), Some((_, '`'))) {
                    // Check for block comment ```
                    if let Some((_, _)) = chars.next() {
                        content.push('`');
                        if let Some((_, _)) = chars.next_if(|(_, c)| *c == '`') {
                            content.push('`');
                            // Block comment: ``` ... ```
                            let mut end_count = 0;
                            for (_, next_c) in chars.by_ref() {
                                content.push(next_c);
                                if next_c == '`' {
                                    end_count += 1;
                                } else {
                                    end_count = 0;
                                }
                                if end_count == 3 {
                                    break;
                                }
                            }
                            // Split multiline comment into spans/lines
                            push_multiline_content(
                                &mut lines,
                                &mut current_line_spans,
                                content,
                                Style::default().fg(theme.syntax_comment),
                            );
                            continue;
                        }
                    }
                }

                // Check if it's a single line comment ` (backtick + space)
                if matches!(chars.peek(), Some((_, c)) if c.is_whitespace()) {
                    for (_, next_c) in chars.by_ref() {
                        content.push(next_c);
                        if next_c == '\n' {
                            break;
                        }
                    }
                    push_multiline_content(
                        &mut lines,
                        &mut current_line_spans,
                        content,
                        Style::default().fg(theme.syntax_comment),
                    );
                } else {
                    // Macro: `...`
                    for (_, next_c) in chars.by_ref() {
                        content.push(next_c);
                        if next_c == '`' || next_c == '\n' {
                            break;
                        }
                    }
                    push_multiline_content(
                        &mut lines,
                        &mut current_line_spans,
                        content,
                        Style::default().fg(theme.syntax_command),
                    );
                }
            }
            // Strings
            '"' | '\'' => {
                push_token(&mut current_line_spans, &mut current_token, theme);
                let quote = c;
                let mut string_val = quote.to_string();
                while let Some((_, next_c)) = chars.next() {
                    string_val.push(next_c);
                    if next_c == quote {
                        // Check for escaped quote (e.g. "" in SPL)
                        if quote == '"' && matches!(chars.peek(), Some((_, '"'))) {
                            string_val.push('"');
                            chars.next();
                            continue;
                        }
                        break;
                    }
                }
                push_multiline_content(
                    &mut lines,
                    &mut current_line_spans,
                    string_val,
                    Style::default().fg(theme.syntax_string),
                );
            }
            // Whitespace
            c if c.is_whitespace() => {
                push_token(&mut current_line_spans, &mut current_token, theme);
                if c == '\n' {
                    lines.push(Line::from(std::mem::take(&mut current_line_spans)));
                } else {
                    current_line_spans.push(Span::raw(c.to_string()));
                }
            }
            // Accumulate word/number
            _ => {
                current_token.push(c);
            }
        }
    }

    push_token(&mut current_line_spans, &mut current_token, theme);
    if !current_line_spans.is_empty() {
        lines.push(Line::from(current_line_spans));
    }

    Text::from(lines)
}

fn push_multiline_content(
    lines: &mut Vec<Line<'static>>,
    current_line_spans: &mut Vec<Span<'static>>,
    content: String,
    style: Style,
) {
    let mut parts = content.split('\n').peekable();
    while let Some(part) = parts.next() {
        if !part.is_empty() {
            current_line_spans.push(Span::styled(part.to_string(), style));
        }
        if parts.peek().is_some() {
            lines.push(Line::from(std::mem::take(current_line_spans)));
        }
    }
}

fn push_token(spans: &mut Vec<Span<'static>>, token: &mut String, theme: &Theme) {
    if token.is_empty() {
        return;
    }

    let style = if SPL_COMMANDS.contains(&token.to_lowercase().as_str()) {
        Style::default()
            .fg(theme.syntax_command)
            .add_modifier(Modifier::BOLD)
    } else if SPL_OPERATORS.contains(&token.to_uppercase().as_str()) {
        Style::default()
            .fg(theme.syntax_operator)
            .add_modifier(Modifier::BOLD)
    } else if SPL_FUNCTIONS.contains(&token.to_lowercase().as_str())
        || token.chars().all(|c| c.is_numeric() || c == '.')
    {
        Style::default().fg(theme.syntax_number)
    } else {
        Style::default()
    };

    spans.push(Span::styled(token.clone(), style));
    token.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_commands() {
        let text = highlight_spl(
            "search index=_internal | stats count by sourcetype",
            &Theme::default(),
        );
        let line = &text.lines[0];
        assert_eq!(line.spans.len(), 15);
        // 1. "search" (Cyan)
        // 2. " " (Raw)
        // 3. "index" (Default)
        // 4. "=" (Red)
        // 5. "_internal" (Default)
        // 6. " " (Raw)
        // 7. "|" (Yellow)
        // 8. " " (Raw)
        // 9. "stats" (Cyan)
        // 10. " " (Raw)
        // 11. "count" (Blue)
        // 12. " " (Raw)
        // 13. "by" (Default)
        // 14. " " (Raw)
        // 15. "sourcetype" (Default)

        assert_eq!(line.spans[0].content, "search");
        assert_eq!(
            line.spans[0].style.fg,
            Some(Theme::default().syntax_command)
        );

        assert_eq!(line.spans[6].content, "|");
        assert_eq!(line.spans[6].style.fg, Some(Theme::default().syntax_pipe));

        assert_eq!(line.spans[8].content, "stats");
        assert_eq!(
            line.spans[8].style.fg,
            Some(Theme::default().syntax_command)
        );

        assert_eq!(line.spans[10].content, "count");
        assert_eq!(
            line.spans[10].style.fg,
            Some(Theme::default().syntax_number)
        );
    }

    #[test]
    fn test_highlight_operators() {
        let text = highlight_spl("index=main AND status=200 OR NOT error", &Theme::default());
        let line = &text.lines[0];
        // index (0), = (1), main (2), " " (3), AND (4), " " (5), status (6), = (7), 200 (8), " " (9), OR (10), " " (11), NOT (12), " " (13), error (14)
        assert_eq!(line.spans[4].content, "AND");
        assert_eq!(
            line.spans[4].style.fg,
            Some(Theme::default().syntax_operator)
        );
        assert_eq!(line.spans[10].content, "OR");
        assert_eq!(
            line.spans[10].style.fg,
            Some(Theme::default().syntax_operator)
        );
        assert_eq!(line.spans[12].content, "NOT");
        assert_eq!(
            line.spans[12].style.fg,
            Some(Theme::default().syntax_operator)
        );
    }

    #[test]
    fn test_highlight_strings() {
        let text = highlight_spl("search message=\"hello world\"", &Theme::default());
        let line = &text.lines[0];
        // search (0), " " (1), message (2), = (3), "hello world" (4)
        assert_eq!(line.spans[4].content, "\"hello world\"");
        assert_eq!(line.spans[4].style.fg, Some(Theme::default().syntax_string));

        let text = highlight_spl(
            "search message=\"He said \"\"Hello\"\"\"",
            &Theme::default(),
        );
        let line = &text.lines[0];
        assert_eq!(line.spans[4].content, "\"He said \"\"Hello\"\"\"");
        assert_eq!(line.spans[4].style.fg, Some(Theme::default().syntax_string));
    }

    #[test]
    fn test_highlight_numbers() {
        let text = highlight_spl("eval x=123.45", &Theme::default());
        let line = &text.lines[0];
        // eval (0), " " (1), x (2), = (3), 123.45 (4)
        assert_eq!(line.spans[4].content, "123.45");
        assert_eq!(line.spans[4].style.fg, Some(Theme::default().syntax_number));
    }

    #[test]
    fn test_highlight_comments() {
        let text = highlight_spl("search index=main ` this is a comment", &Theme::default());
        let line = &text.lines[0];
        // search (0), " " (1), index (2), = (3), main (4), " " (5), ` comment (6)
        assert_eq!(line.spans[6].content, "` this is a comment");
        assert_eq!(
            line.spans[6].style.fg,
            Some(Theme::default().syntax_comment)
        );

        let text = highlight_spl("search ``` block comment ``` index=main", &Theme::default());
        let line = &text.lines[0];
        // search (0), " " (1), ``` block comment ``` (2), " " (3), index (4), = (5), main (6)
        assert_eq!(line.spans[2].content, "``` block comment ```");
        assert_eq!(
            line.spans[2].style.fg,
            Some(Theme::default().syntax_comment)
        );

        let text = highlight_spl("search `my_macro` index=main", &Theme::default());
        let line = &text.lines[0];
        // search (0), " " (1), `my_macro` (2), " " (3), index (4), = (5), main (6)
        assert_eq!(line.spans[2].content, "`my_macro`");
        assert_eq!(
            line.spans[2].style.fg,
            Some(Theme::default().syntax_command)
        );
    }

    #[test]
    fn test_highlight_multiline() {
        let text = highlight_spl("search index=main\n| stats count", &Theme::default());
        assert_eq!(text.lines.len(), 2);
        assert_eq!(text.lines[0].spans[0].content, "search");
        assert_eq!(text.lines[1].spans[0].content, "|");
    }
}
