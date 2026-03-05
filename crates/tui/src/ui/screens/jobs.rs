//! Jobs screen rendering.
//!
//! Renders the search jobs list as a table with status, duration, and result counts.
//! Supports filtering jobs by SID or status substring match with highlighting,
//! and sorting by any column.
//!
//! Uses the centralized theme system via [`ThemeExt`] for consistent styling.

use crate::app::input::components::SingleLineInput;
use crate::app::{SortColumn, SortDirection};
use crate::ui::theme::ThemeExt;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
};
use splunk_client::models::SearchJobStatus;
use splunk_config::Theme;
use std::collections::HashSet;

/// Configuration for rendering the jobs table.
pub struct JobsRenderConfig<'a> {
    /// The list of jobs to display (already filtered and sorted by the App)
    pub jobs: &'a [&'a SearchJobStatus],
    /// The current table selection state
    pub state: &'a mut TableState,
    /// Whether auto-refresh is enabled
    pub auto_refresh: bool,
    /// Optional filter string for filtering jobs
    pub filter: &'a Option<String>,
    /// Current filter input (for display when filtering)
    pub filter_input: &'a SingleLineInput,
    /// Whether the user is currently in filter mode
    pub is_filtering: bool,
    /// Current sort column
    pub sort_column: SortColumn,
    /// Current sort direction
    pub sort_direction: SortDirection,
    /// Selected job SIDs for multi-selection
    pub selected_jobs: &'a HashSet<String>,
    /// Theme for consistent styling.
    pub theme: &'a Theme,
}

/// Render the jobs table.
///
/// # Arguments
///
/// * `f` - The frame to render to
/// * `area` - The area to render within
/// * `config` - Configuration for rendering
pub fn render_jobs(f: &mut Frame, area: Rect, config: JobsRenderConfig) {
    let JobsRenderConfig {
        jobs,
        state,
        auto_refresh,
        filter,
        filter_input,
        is_filtering,
        sort_column,
        sort_direction,
        selected_jobs,
        theme,
    } = config;

    // If filtering, show the filter input at the top
    let (table_area, filter_area) = if is_filtering || filter.is_some() {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
            .split(area);
        (chunks[1], Some(chunks[0]))
    } else {
        (area, None)
    };

    // Render filter input area if active
    if let Some(filter_area) = filter_area {
        let filter_text = if is_filtering {
            format!(
                "Filter: {} (Esc to cancel, Enter to apply)",
                filter_input.value()
            )
        } else if let Some(f) = filter {
            format!("Filter: {} (Press / to edit, Esc to clear)", f)
        } else {
            String::new()
        };

        let filter_paragraph = Paragraph::new(filter_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Filter")
                    .border_style(theme.border())
                    .title_style(theme.title()),
            )
            .alignment(Alignment::Left);
        f.render_widget(filter_paragraph, filter_area);
    }

    // The jobs are already filtered and sorted by the App, so we can use them directly.
    // We just need to convert from &[&SearchJobStatus] to Vec<&SearchJobStatus> for compatibility.
    let display_jobs: Vec<&SearchJobStatus> = jobs.to_vec();

    // Create header with sort indicators
    let sort_indicator = match sort_direction {
        SortDirection::Asc => "↑",
        SortDirection::Desc => "↓",
    };

    let header_cells: Vec<Cell> = vec![
        header_cell("Sel", false, ""),
        header_cell("SID", sort_column == SortColumn::Sid, sort_indicator),
        header_cell("Status", sort_column == SortColumn::Status, sort_indicator),
        header_cell(
            "Duration",
            sort_column == SortColumn::Duration,
            sort_indicator,
        ),
        header_cell(
            "Results",
            sort_column == SortColumn::Results,
            sort_indicator,
        ),
        header_cell("Events", sort_column == SortColumn::Events, sort_indicator),
    ];

    // Create rows with highlighting
    let rows: Vec<Row> = display_jobs
        .iter()
        .map(|job| {
            let selection_indicator = if selected_jobs.contains(&job.sid) {
                "[x]"
            } else {
                "[ ]"
            };

            let status_text: String = if job.is_done {
                "Done".to_string()
            } else if job.done_progress > 0.0 {
                format!("Running ({:.0}%)", job.done_progress * 100.0)
            } else {
                "Running".to_string()
            };

            let status_style = if job.is_done {
                theme.success()
            } else {
                theme.warning()
            };

            // Highlight matching text if filter is active
            let sid_cell = if let Some(filter_str) = filter {
                highlight_match(job.sid.clone(), filter_str, theme)
            } else {
                Cell::from(job.sid.clone())
            };

            let status_cell = if let Some(filter_str) = filter {
                highlight_match(status_text.clone(), filter_str, theme)
            } else {
                Cell::from(status_text).style(status_style)
            };

            Row::new(vec![
                Cell::from(selection_indicator),
                sid_cell,
                status_cell,
                Cell::from(format!("{:.2}s", job.run_duration)),
                Cell::from(job.result_count.to_string()),
                Cell::from(job.event_count.to_string()),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        &[
            // Sel, SID, Status, Duration, Results, Events
            Constraint::Length(4),
            Constraint::Max(40),
            Constraint::Length(15),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(10),
        ],
    )
    .header(Row::new(header_cells).style(theme.table_header()))
    .block(
        Block::default()
            .title(if auto_refresh {
                "Search Jobs [AUTO]"
            } else {
                "Search Jobs"
            })
            .borders(Borders::ALL)
            .border_style(theme.border())
            .title_style(theme.title()),
    )
    .row_highlight_style(theme.highlight())
    .column_spacing(1);

    f.render_stateful_widget(table, table_area, state);
}

/// Create a header cell with optional sort indicator.
fn header_cell<'a>(text: &'a str, is_sorted: bool, indicator: &str) -> Cell<'a> {
    if is_sorted {
        Cell::from(format!("{} {}", text, indicator))
    } else {
        Cell::from(text.to_string())
    }
}

/// Highlight matching text in the given string with the specified color.
fn highlight_match(text: String, pattern: &str, theme: &Theme) -> Cell<'static> {
    let pattern_lower = pattern.to_lowercase();
    let text_lower = text.to_lowercase();

    if let Some(pos) = text_lower.find(&pattern_lower) {
        let before = text[..pos].to_string();
        let matched = text[pos..pos + pattern.len()].to_string();
        let after = text[pos + pattern.len()..].to_string();

        let spans = vec![
            Span::raw(before),
            Span::styled(matched, theme.info()),
            Span::raw(after),
        ];
        Cell::from(Line::from(spans))
    } else {
        Cell::from(text)
    }
}
