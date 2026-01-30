//! Search screen rendering.
//!
//! Renders the search input, status, and results for running Splunk searches.
//! Includes real-time SPL validation feedback.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Gauge, Paragraph},
};

use crate::app::SplValidationState;
use crate::ui::syntax::highlight_spl;
use splunk_config::Theme;

/// Configuration for rendering the search screen.
pub struct SearchRenderConfig<'a> {
    /// The current search input text
    pub search_input: &'a str,
    /// Cursor position within search_input (byte index)
    pub search_cursor_position: usize,
    /// Whether the query input is focused (cursor visible when true)
    pub is_query_focused: bool,
    /// The current search status message
    pub search_status: &'a str,
    /// Whether a search is currently running
    pub loading: bool,
    /// Progress of the current search (0.0 to 1.0)
    pub progress: f32,
    /// The search results to display (raw JSON values)
    pub search_results: &'a [serde_json::Value],
    /// The scroll offset for displaying results
    pub search_scroll_offset: usize,
    /// Total number of results available (if known)
    pub search_results_total_count: Option<u64>,
    /// Whether more results can be loaded
    pub search_has_more_results: bool,
    /// Theme for consistent styling.
    pub theme: &'a Theme,
    /// SPL validation state for real-time feedback.
    pub spl_validation_state: &'a SplValidationState,
    /// Whether validation is pending (debounced).
    pub spl_validation_pending: bool,
}

/// Render the search screen.
///
/// # Arguments
///
/// * `f` - The frame to render to
/// * `area` - The area to render within
/// * `config` - Configuration for rendering
pub fn render_search(f: &mut Frame, area: Rect, config: SearchRenderConfig) {
    let SearchRenderConfig {
        search_input,
        search_cursor_position,
        is_query_focused,
        search_status,
        loading,
        progress,
        search_results,
        search_scroll_offset,
        search_results_total_count,
        search_has_more_results,
        theme,
        spl_validation_state,
        spl_validation_pending,
    } = config;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(3), // Search input
                Constraint::Length(3), // Status
                Constraint::Min(0),    // Results
            ]
            .as_ref(),
        )
        .split(area);

    // Search input with validation status
    let (border_color, status_icon) = if loading {
        (theme.border, "")
    } else if spl_validation_pending {
        (theme.info, "⏳ ")
    } else {
        match spl_validation_state.valid {
            Some(true) => {
                if spl_validation_state.warnings.is_empty() {
                    (theme.success, "✓ ")
                } else {
                    (theme.warning, "⚠ ")
                }
            }
            Some(false) => (theme.error, "✗ "),
            None => (theme.border, ""),
        }
    };

    let input_title = if search_input.len() < 3 {
        "Search Query".to_string()
    } else {
        format!("{}Search Query", status_icon)
    };

    let input = Paragraph::new(highlight_spl(search_input, theme)).block(
        Block::default()
            .borders(Borders::ALL)
            .title(input_title)
            .border_style(Style::default().fg(border_color))
            .title_style(Style::default().fg(border_color)),
    );
    f.render_widget(input, chunks[0]);

    // Render cursor when query is focused
    if is_query_focused {
        // Calculate cursor position
        // Input area: chunks[0] has borders, so content starts at x+1, y+1
        let input_area = chunks[0];
        let content_x = input_area.x + 1;
        let content_y = input_area.y + 1;

        // Calculate display width of text before cursor
        // Use byte index directly since we're dealing with ASCII cursor movement
        let text_before_cursor = &search_input[..search_cursor_position.min(search_input.len())];
        let cursor_offset = text_before_cursor.chars().count() as u16;

        // Set cursor position
        let cursor_x = content_x + cursor_offset;
        let cursor_y = content_y;

        // Only show cursor if it fits within the input area
        if cursor_x < input_area.x + input_area.width - 1 {
            f.set_cursor_position(ratatui::layout::Position::new(cursor_x, cursor_y));
        }
    }

    // Status with validation feedback
    let status_text: Line = if loading {
        // During loading, just show the search status (gauge is rendered separately below)
        Line::from(search_status)
    } else if search_input.len() >= 3 && !spl_validation_pending {
        // Show validation status
        match spl_validation_state.valid {
            Some(true) => {
                if let Some(first_warning) = spl_validation_state.warnings.first() {
                    Line::from(format!("⚠ Warning: {}", first_warning))
                } else {
                    Line::from("✓ SPL syntax is valid")
                }
            }
            Some(false) => {
                let error = spl_validation_state
                    .errors
                    .first()
                    .map(|e| e.as_str())
                    .unwrap_or("Invalid SPL syntax");
                Line::from(format!("✗ Error: {}", error))
            }
            None => Line::from(search_status),
        }
    } else if spl_validation_pending {
        Line::from("⏳ Validating...")
    } else {
        Line::from(search_status)
    };

    // Render status - either gauge (when loading) or paragraph
    if loading {
        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("Status"))
            .gauge_style(
                Style::default()
                    .fg(theme.info)
                    .bg(theme.background)
                    .add_modifier(Modifier::ITALIC),
            )
            .ratio(progress.clamp(0.0, 1.0) as f64)
            .label(format!("{} ({:.0}%)", search_status, progress * 100.0));
        f.render_widget(gauge, chunks[1]);
    } else {
        let status = Paragraph::new(status_text).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Status")
                .border_style(Style::default().fg(theme.border))
                .title_style(Style::default().fg(theme.title)),
        );
        f.render_widget(status, chunks[1]);
    }

    // Calculate actual viewport height from available area
    let available_height = chunks[2].height.saturating_sub(2) as usize; // Account for borders

    // Results
    if search_results.is_empty() {
        let placeholder = Paragraph::new("No results. Enter a search query and press Enter.")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Results")
                    .border_style(Style::default().fg(theme.border))
                    .title_style(Style::default().fg(theme.title)),
            )
            .alignment(Alignment::Center);
        f.render_widget(placeholder, chunks[2]);
    } else {
        // Virtualization: Only format and render visible results
        let visible_end = (search_scroll_offset + available_height).min(search_results.len());

        let results_text: Vec<Line> = search_results
            .iter()
            .enumerate()
            .skip(search_scroll_offset)
            .take_while(|(i, _)| *i < visible_end)
            .flat_map(|(_, v)| {
                // Format each result on-demand
                let formatted =
                    serde_json::to_string_pretty(v).unwrap_or_else(|_| "<invalid>".to_string());

                // Split multi-line JSON into separate Lines
                formatted
                    .lines()
                    .map(|line| Line::from(line.to_string()))
                    .collect::<Vec<_>>()
            })
            .collect();

        // Build title with pagination info
        let title = if let Some(total) = search_results_total_count {
            if search_has_more_results {
                format!(
                    "Results ({}-{} / {} total, loading...)",
                    search_scroll_offset + 1,
                    visible_end,
                    total
                )
            } else {
                format!(
                    "Results ({}-{} / {} total)",
                    search_scroll_offset + 1,
                    visible_end,
                    total
                )
            }
        } else {
            format!("Results ({} loaded)", search_results.len())
        };

        let results = Paragraph::new(results_text).block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(theme.border))
                .title_style(Style::default().fg(theme.title)),
        );
        f.render_widget(results, chunks[2]);
    }
}
