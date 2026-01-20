//! Jobs screen rendering.
//!
//! Renders the search jobs list as a table with status, duration, and result counts.

use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Cell, Row, Table, TableState};
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
};
use splunk_client::models::SearchJobStatus;

/// Render the jobs table.
///
/// # Arguments
///
/// * `f` - The frame to render to
/// * `area` - The area to render within
/// * `jobs` - The list of jobs to display
/// * `state` - The current table selection state
/// * `auto_refresh` - Whether auto-refresh is enabled
pub fn render_jobs(
    f: &mut Frame,
    area: Rect,
    jobs: &[SearchJobStatus],
    state: &mut TableState,
    auto_refresh: bool,
) {
    let rows: Vec<Row> = jobs
        .iter()
        .map(|job| {
            let status_cell = if job.is_done {
                Cell::from("Done").style(Style::default().fg(Color::Green))
            } else if job.done_progress > 0.0 {
                Cell::from(format!("Running ({:.0}%)", job.done_progress * 100.0))
                    .style(Style::default().fg(Color::Yellow))
            } else {
                Cell::from("Running").style(Style::default().fg(Color::Yellow))
            };

            Row::new(vec![
                Cell::from(job.sid.clone()),
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
    .header(
        Row::new(vec![
            Cell::from("SID"),
            Cell::from("Status"),
            Cell::from("Duration"),
            Cell::from("Results"),
            Cell::from("Events"),
        ])
        .style(Style::default().fg(Color::Cyan)),
    )
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

    f.render_stateful_widget(table, area, state);
}
