//! SPL (Splunk Processing Language) syntax highlighting.
//!
//! Provides functions to tokenize and style SPL queries for TUI display.

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
};
use std::sync::LazyLock;

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
pub fn highlight_spl(input: &str) -> Text<'static> {
    let mut lines = Vec::new();
    let mut current_line_spans = Vec::new();
    let mut chars = input.char_indices().peekable();
    let mut current_token = String::new();
    let mut current_start = 0;

    while let Some((idx, c)) = chars.next() {
        match c {
            // Pipes
            '|' => {
                push_token(&mut current_line_spans, &mut current_token, current_start);
                current_line_spans.push(Span::styled(
                    "|",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ));
                current_start = idx + 1;
            }
            // Comparison operators
            '=' | '!' | '>' | '<' => {
                push_token(&mut current_line_spans, &mut current_token, current_start);
                let mut op = c.to_string();
                if matches!(chars.peek(), Some((_, '='))) {
                    op.push('=');
                    chars.next();
                }
                let op_len = op.len();
                current_line_spans.push(Span::styled(op, Style::default().fg(Color::Red)));
                current_start = idx + op_len;
            }
            // Punctuation
            '(' | ')' | '[' | ']' | ',' => {
                push_token(&mut current_line_spans, &mut current_token, current_start);
                current_line_spans.push(Span::styled(
                    c.to_string(),
                    Style::default().fg(Color::DarkGray),
                ));
                current_start = idx + 1;
            }
            // Comments and Macros
            '`' => {
                push_token(&mut current_line_spans, &mut current_token, current_start);
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
                                Style::default().fg(Color::Gray),
                            );
                            current_start = idx + 1; // dummy, will be reset
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
                        Style::default().fg(Color::Gray),
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
                        Style::default().fg(Color::Cyan),
                    );
                }
                current_start = idx + 1; // dummy
            }
            // Strings
            '"' | '\'' => {
                push_token(&mut current_line_spans, &mut current_token, current_start);
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
                    Style::default().fg(Color::Green),
                );
                current_start = idx + 1; // dummy
            }
            // Whitespace
            c if c.is_whitespace() => {
                push_token(&mut current_line_spans, &mut current_token, current_start);
                if c == '\n' {
                    lines.push(Line::from(std::mem::take(&mut current_line_spans)));
                } else {
                    current_line_spans.push(Span::raw(c.to_string()));
                }
                current_start = idx + 1;
            }
            // Accumulate word/number
            _ => {
                if current_token.is_empty() {
                    current_start = idx;
                }
                current_token.push(c);
            }
        }
    }

    push_token(&mut current_line_spans, &mut current_token, current_start);
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

fn push_token(spans: &mut Vec<Span<'static>>, token: &mut String, _start: usize) {
    if token.is_empty() {
        return;
    }

    let style = if SPL_COMMANDS.contains(&token.to_lowercase().as_str()) {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else if SPL_OPERATORS.contains(&token.to_uppercase().as_str()) {
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD)
    } else if SPL_FUNCTIONS.contains(&token.to_lowercase().as_str())
        || token.chars().all(|c| c.is_numeric() || c == '.')
    {
        Style::default().fg(Color::Blue)
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
        let text = highlight_spl("search index=_internal | stats count by sourcetype");
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
        assert_eq!(line.spans[0].style.fg, Some(Color::Cyan));

        assert_eq!(line.spans[6].content, "|");
        assert_eq!(line.spans[6].style.fg, Some(Color::Yellow));

        assert_eq!(line.spans[8].content, "stats");
        assert_eq!(line.spans[8].style.fg, Some(Color::Cyan));

        assert_eq!(line.spans[10].content, "count");
        assert_eq!(line.spans[10].style.fg, Some(Color::Blue));
    }

    #[test]
    fn test_highlight_operators() {
        let text = highlight_spl("index=main AND status=200 OR NOT error");
        let line = &text.lines[0];
        // index (0), = (1), main (2), " " (3), AND (4), " " (5), status (6), = (7), 200 (8), " " (9), OR (10), " " (11), NOT (12), " " (13), error (14)
        assert_eq!(line.spans[4].content, "AND");
        assert_eq!(line.spans[4].style.fg, Some(Color::Magenta));
        assert_eq!(line.spans[10].content, "OR");
        assert_eq!(line.spans[10].style.fg, Some(Color::Magenta));
        assert_eq!(line.spans[12].content, "NOT");
        assert_eq!(line.spans[12].style.fg, Some(Color::Magenta));
    }

    #[test]
    fn test_highlight_strings() {
        let text = highlight_spl("search message=\"hello world\"");
        let line = &text.lines[0];
        // search (0), " " (1), message (2), = (3), "hello world" (4)
        assert_eq!(line.spans[4].content, "\"hello world\"");
        assert_eq!(line.spans[4].style.fg, Some(Color::Green));

        let text = highlight_spl("search message=\"He said \"\"Hello\"\"\"");
        let line = &text.lines[0];
        assert_eq!(line.spans[4].content, "\"He said \"\"Hello\"\"\"");
        assert_eq!(line.spans[4].style.fg, Some(Color::Green));
    }

    #[test]
    fn test_highlight_numbers() {
        let text = highlight_spl("eval x=123.45");
        let line = &text.lines[0];
        // eval (0), " " (1), x (2), = (3), 123.45 (4)
        assert_eq!(line.spans[4].content, "123.45");
        assert_eq!(line.spans[4].style.fg, Some(Color::Blue));
    }

    #[test]
    fn test_highlight_comments() {
        let text = highlight_spl("search index=main ` this is a comment");
        let line = &text.lines[0];
        // search (0), " " (1), index (2), = (3), main (4), " " (5), ` comment (6)
        assert_eq!(line.spans[6].content, "` this is a comment");
        assert_eq!(line.spans[6].style.fg, Some(Color::Gray));

        let text = highlight_spl("search ``` block comment ``` index=main");
        let line = &text.lines[0];
        // search (0), " " (1), ``` block comment ``` (2), " " (3), index (4), = (5), main (6)
        assert_eq!(line.spans[2].content, "``` block comment ```");
        assert_eq!(line.spans[2].style.fg, Some(Color::Gray));

        let text = highlight_spl("search `my_macro` index=main");
        let line = &text.lines[0];
        // search (0), " " (1), `my_macro` (2), " " (3), index (4), = (5), main (6)
        assert_eq!(line.spans[2].content, "`my_macro`");
        assert_eq!(line.spans[2].style.fg, Some(Color::Cyan));
    }

    #[test]
    fn test_highlight_multiline() {
        let text = highlight_spl("search index=main\n| stats count");
        assert_eq!(text.lines.len(), 2);
        assert_eq!(text.lines[0].spans[0].content, "search");
        assert_eq!(text.lines[1].spans[0].content, "|");
    }
}
