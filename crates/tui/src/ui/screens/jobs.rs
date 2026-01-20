//! Jobs screen rendering.
//!
//! Renders the search jobs list as a table with status, duration, and result counts.
//! Supports filtering jobs by SID or status substring match with highlighting,
//! and sorting by any column.

use crate::app::{SortColumn, SortDirection};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
};
use splunk_client::models::SearchJobStatus;
use std::cmp::Ordering;

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
    pub filter_input: &'a str,
    /// Whether the user is currently in filter mode
    pub is_filtering: bool,
    /// Current sort column
    pub sort_column: SortColumn,
    /// Current sort direction
    pub sort_direction: SortDirection,
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
            format!("Filter: {}", filter_input)
        } else if let Some(f) = filter {
            format!("Filter: {} (Press ESC to clear)", f)
        } else {
            String::new()
        };

        let filter_paragraph = Paragraph::new(filter_text)
            .block(Block::default().borders(Borders::ALL).title("Filter"))
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
            let status_text = if job.is_done {
                "Done"
            } else if job.done_progress > 0.0 {
                // Allocate formatted status text with enough lifetime
                let status = format!("Running ({:.0}%)", job.done_progress * 100.0);
                Box::leak(Box::new(status)) as &str
            } else {
                "Running"
            };

            let status_style = if job.is_done {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Yellow)
            };

            // Highlight matching text if filter is active
            let sid_cell = if let Some(filter_str) = filter {
                highlight_match(&job.sid, filter_str, Color::Cyan)
            } else {
                Cell::from(job.sid.clone())
            };

            let status_cell = if let Some(filter_str) = filter {
                highlight_match(status_text, filter_str, Color::Cyan)
            } else {
                Cell::from(status_text.to_string()).style(status_style)
            };

            Row::new(vec![
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
            // SID, Status, Duration, Results, Events
            Constraint::Max(40),
            Constraint::Length(15),
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(10),
        ],
    )
    .header(Row::new(header_cells).style(Style::default().fg(Color::Cyan)))
    .block(
        Block::default()
            .title(if auto_refresh {
                "Search Jobs [AUTO]"
            } else {
                "Search Jobs"
            })
            .borders(Borders::ALL),
    )
    .row_highlight_style(Style::default().bg(Color::DarkGray).fg(Color::Yellow))
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

/// Compare two jobs based on the specified column and direction.
/// NOTE: No longer used as filtering/sorting is done in the App.
/// Kept for reference/potential future use.
#[allow(dead_code)]
fn compare_jobs(
    a: &SearchJobStatus,
    b: &SearchJobStatus,
    column: &SortColumn,
    direction: &SortDirection,
) -> Ordering {
    let ordering = match column {
        SortColumn::Sid => a.sid.cmp(&b.sid),
        SortColumn::Status => {
            // Sort by is_done first, then by progress
            match (a.is_done, b.is_done) {
                (true, false) => Ordering::Less,
                (false, true) => Ordering::Greater,
                _ => a
                    .done_progress
                    .partial_cmp(&b.done_progress)
                    .unwrap_or(Ordering::Equal),
            }
        }
        SortColumn::Duration => a
            .run_duration
            .partial_cmp(&b.run_duration)
            .unwrap_or(Ordering::Equal),
        SortColumn::Results => a.result_count.cmp(&b.result_count),
        SortColumn::Events => a.event_count.cmp(&b.event_count),
    };

    match direction {
        SortDirection::Asc => ordering,
        SortDirection::Desc => ordering.reverse(),
    }
}

/// Highlight matching text in the given string with the specified color.
fn highlight_match<'a>(text: &'a str, pattern: &str, color: Color) -> Cell<'a> {
    let pattern_lower = pattern.to_lowercase();
    let text_lower = text.to_lowercase();

    if let Some(pos) = text_lower.find(&pattern_lower) {
        let before = &text[..pos];
        let matched = &text[pos..pos + pattern.len()];
        let after = &text[pos + pattern.len()..];

        let spans = vec![
            Span::raw(before),
            Span::styled(matched, Style::default().fg(color)),
            Span::raw(after),
        ];
        Cell::from(Line::from(spans))
    } else {
        Cell::from(text.to_string())
    }
}
