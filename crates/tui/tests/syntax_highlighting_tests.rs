//! Integration tests for syntax highlighting functionality.
//!
//! These tests verify both the regex-based `highlight_spl()` function and
//! the tree-sitter `SyntaxHighlighter` infrastructure.

use splunk_config::Theme;
use splunk_tui::ui::syntax::{SyntaxHighlighter, TokenType, highlight_spl};

// =============================================================================
// Regex-based highlighting tests
// =============================================================================

#[test]
fn test_highlight_simple_search() {
    let theme = Theme::default();
    let text = highlight_spl("search index=main", &theme);

    assert!(!text.lines.is_empty());
    // First span should be "search" with command styling
    assert_eq!(text.lines[0].spans[0].content, "search");
}

#[test]
fn test_highlight_with_pipes() {
    let theme = Theme::default();
    let text = highlight_spl("search index=main | stats count by sourcetype", &theme);

    // Should have multiple lines/spans including pipe
    let line = &text.lines[0];
    let has_pipe = line.spans.iter().any(|s| s.content == "|");
    assert!(has_pipe, "Should contain pipe character");
}

#[test]
fn test_highlight_functions() {
    let theme = Theme::default();
    let text = highlight_spl("stats count, avg(duration)", &theme);

    let line = &text.lines[0];
    let has_count = line.spans.iter().any(|s| s.content == "count");
    assert!(has_count, "Should highlight count function");
}

#[test]
fn test_highlight_commands() {
    let theme = Theme::default();
    let text = highlight_spl("search index=_internal | stats count by sourcetype", &theme);
    let line = &text.lines[0];
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
    let theme = Theme::default();
    let text = highlight_spl("index=main AND status=200 OR NOT error", &theme);
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
    let theme = Theme::default();
    let text = highlight_spl("search message=\"hello world\"", &theme);
    let line = &text.lines[0];
    // search (0), " " (1), message (2), = (3), "hello world" (4)
    assert_eq!(line.spans[4].content, "\"hello world\"");
    assert_eq!(line.spans[4].style.fg, Some(Theme::default().syntax_string));

    let text = highlight_spl("search message=\"He said \"\"Hello\"\"\"", &theme);
    let line = &text.lines[0];
    assert_eq!(line.spans[4].content, "\"He said \"\"Hello\"\"\"");
    assert_eq!(line.spans[4].style.fg, Some(Theme::default().syntax_string));
}

#[test]
fn test_highlight_numbers() {
    let theme = Theme::default();
    let text = highlight_spl("eval x=123.45", &theme);
    let line = &text.lines[0];
    // eval (0), " " (1), x (2), = (3), 123.45 (4)
    assert_eq!(line.spans[4].content, "123.45");
    assert_eq!(line.spans[4].style.fg, Some(Theme::default().syntax_number));
}

#[test]
fn test_highlight_comments() {
    let theme = Theme::default();
    let text = highlight_spl("search index=main ` this is a comment", &theme);
    let line = &text.lines[0];
    // search (0), " " (1), index (2), = (3), main (4), " " (5), ` comment (6)
    assert_eq!(line.spans[6].content, "` this is a comment");
    assert_eq!(
        line.spans[6].style.fg,
        Some(Theme::default().syntax_comment)
    );

    let text = highlight_spl("search ``` block comment ``` index=main", &theme);
    let line = &text.lines[0];
    // search (0), " " (1), ``` block comment ``` (2), " " (3), index (4), = (5), main (6)
    assert_eq!(line.spans[2].content, "``` block comment ```");
    assert_eq!(
        line.spans[2].style.fg,
        Some(Theme::default().syntax_comment)
    );

    let text = highlight_spl("search `my_macro` index=main", &theme);
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
    let theme = Theme::default();
    let text = highlight_spl("search index=main\n| stats count", &theme);
    assert_eq!(text.lines.len(), 2);
    assert_eq!(text.lines[0].spans[0].content, "search");
    assert_eq!(text.lines[1].spans[0].content, "|");
}

// =============================================================================
// Tree-sitter SyntaxHighlighter infrastructure tests
// =============================================================================

#[test]
fn test_syntax_highlighter_creation() {
    let highlighter = SyntaxHighlighter::new();
    // SyntaxHighlighter::new() is infallible - it always returns successfully
    // since syntax highlighting is non-critical
    assert!(!highlighter.is_available());
}

#[test]
fn test_syntax_highlighter_default() {
    let highlighter = SyntaxHighlighter::default();
    // Should create without panicking
    assert!(!highlighter.is_available());
}

#[test]
fn test_syntax_highlighter_fallback() {
    let mut highlighter = SyntaxHighlighter::default();
    let theme = Theme::default();

    // Since tree-sitter SPL grammar is not available, this should use fallback
    let results = highlighter.highlight("search index=main | stats count", &theme);

    // Should return styled results from regex-based fallback
    assert!(!results.is_empty());

    // Check that we got some styled content
    let has_search = results.iter().any(|(_, text)| text == "search");
    assert!(has_search, "Should contain 'search' token");
}

#[test]
fn test_token_type_variants() {
    // Verify all token types exist and can be matched
    let types = vec![
        TokenType::Command,
        TokenType::Operator,
        TokenType::Function,
        TokenType::String,
        TokenType::Number,
        TokenType::Comment,
        TokenType::Pipe,
        TokenType::Comparison,
        TokenType::Punctuation,
        TokenType::Macro,
        TokenType::Default,
    ];

    for token_type in types {
        // Just verify they can be created and compared
        let _ = format!("{:?}", token_type);
    }
}

// =============================================================================
// Complex query tests
// =============================================================================

#[test]
fn test_complex_spl_query() {
    let theme = Theme::default();
    let query = r#"search index=_internal sourcetype=splunkd log_level=ERROR
| eval error_type=case(
    match(_raw, "connection"), "connection",
    match(_raw, "timeout"), "timeout",
    true(), "other"
)
| stats count, avg(duration) as avg_duration by error_type
| where count > 10
| sort -count
| head 20"#;

    let text = highlight_spl(query, &theme);

    // Should produce multiple lines
    assert!(text.lines.len() > 1);

    // Verify pipe characters are styled
    let has_pipes = text.lines.iter().any(|line| {
        line.spans
            .iter()
            .any(|s| s.content == "|" && s.style.fg == Some(theme.syntax_pipe))
    });
    assert!(has_pipes, "Should style pipe characters");

    // Verify commands are styled
    let has_commands = text.lines.iter().any(|line| {
        line.spans.iter().any(|s| {
            ["search", "eval", "stats", "where", "sort", "head"].contains(&s.content.as_ref())
                && s.style.fg == Some(theme.syntax_command)
        })
    });
    assert!(has_commands, "Should style SPL commands");
}

#[test]
fn test_highlight_with_macros_and_functions() {
    let theme = Theme::default();
    let query = "`get_security_logs` | stats count by src_ip | `format_results`";

    let text = highlight_spl(query, &theme);

    // Should highlight macros
    let has_macros = text.lines.iter().any(|line| {
        line.spans.iter().any(|s| {
            s.content.contains("get_security_logs") || s.content.contains("format_results")
        })
    });
    assert!(has_macros, "Should contain macro references");

    // Stats should be highlighted as command
    let has_stats = text.lines.iter().any(|line| {
        line.spans
            .iter()
            .any(|s| s.content == "stats" && s.style.fg == Some(theme.syntax_command))
    });
    assert!(has_stats, "Should highlight stats command");
}

#[test]
fn test_highlight_comparison_operators() {
    let theme = Theme::default();
    let queries = vec![
        "field=value",
        "field!=value",
        "field>10",
        "field<10",
        "field>=10",
        "field<=10",
    ];

    for query in queries {
        let text = highlight_spl(query, &theme);
        assert!(!text.lines.is_empty(), "Should parse: {}", query);
    }
}
