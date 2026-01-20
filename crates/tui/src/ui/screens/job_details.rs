//! Job details screen rendering.
//!
//! Renders a detailed view of a single search job, showing all available
//! metadata including status, duration, counts, and other properties.

use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
};
use splunk_client::models::SearchJobStatus;

/// Render detailed information about a single search job.
///
/// # Arguments
///
/// * `f` - The frame to render to
/// * `area` - The area to render within
/// * `job` - The job to display details for
pub fn render_details(f: &mut Frame, area: Rect, job: &SearchJobStatus) {
    // Split into title and content areas
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)].as_ref())
        .split(area);

    // Title block with job SID
    let title = Paragraph::new(format!("Job: {}", job.sid))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Job Details")
                .title_style(Style::default().fg(Color::Cyan)),
        )
        .alignment(Alignment::Center);
    f.render_widget(title, chunks[0]);

    // Content area with job information
    let status_text = if job.is_done {
        "Done"
    } else if job.done_progress > 0.0 {
        &*format!("Running ({:.0}%)", job.done_progress * 100.0)
    } else {
        "Running"
    };

    let status_color = if job.is_done {
        Color::Green
    } else {
        Color::Yellow
    };

    let details = vec![
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Cyan)),
            Span::styled(status_text, Style::default().fg(status_color)),
        ]),
        Line::from(vec![
            Span::styled("Duration: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{:.2} seconds", job.run_duration)),
        ]),
        Line::from(vec![
            Span::styled("Event Count: ", Style::default().fg(Color::Cyan)),
            Span::raw(job.event_count.to_string()),
        ]),
        Line::from(vec![
            Span::styled("Scan Count: ", Style::default().fg(Color::Cyan)),
            Span::raw(job.scan_count.to_string()),
        ]),
        Line::from(vec![
            Span::styled("Result Count: ", Style::default().fg(Color::Cyan)),
            Span::raw(job.result_count.to_string()),
        ]),
        Line::from(vec![
            Span::styled("Disk Usage: ", Style::default().fg(Color::Cyan)),
            Span::raw(format!("{} MB", job.disk_usage)),
        ]),
        Line::from(vec![
            Span::styled("Priority: ", Style::default().fg(Color::Cyan)),
            Span::raw(job.priority.map_or("N/A".to_string(), |p| p.to_string())),
        ]),
        Line::from(vec![
            Span::styled("Label: ", Style::default().fg(Color::Cyan)),
            Span::raw(job.label.as_deref().unwrap_or("N/A")),
        ]),
        Line::from(vec![
            Span::styled("Cursor Time: ", Style::default().fg(Color::Cyan)),
            Span::raw(job.cursor_time.as_deref().unwrap_or("N/A")),
        ]),
        Line::from(vec![
            Span::styled("Finalized: ", Style::default().fg(Color::Cyan)),
            Span::raw(if job.is_finalized { "Yes" } else { "No" }),
        ]),
    ];

    let details_paragraph = Paragraph::new(details)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Left);
    f.render_widget(details_paragraph, chunks[1]);
}
