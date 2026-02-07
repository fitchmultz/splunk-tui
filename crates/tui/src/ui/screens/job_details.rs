//! Job details screen rendering.
//!
//! Renders a detailed view of a single search job, showing all available
//! metadata including status, duration, counts, and other properties.

use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
};
use splunk_client::models::SearchJobStatus;
use splunk_config::Theme;

use crate::ui::theme::ThemeExt;

/// Render detailed information about a single search job.
///
/// # Arguments
///
/// * `f` - The frame to render to
/// * `area` - The area to render within
/// * `job` - The job to display details for
pub fn render_details(f: &mut Frame, area: Rect, job: &SearchJobStatus, theme: &Theme) {
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
                .title_style(theme.title())
                .border_style(theme.border()),
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

    let status_style = if job.is_done {
        theme.success()
    } else {
        theme.warning()
    };

    let details = vec![
        Line::from(vec![
            Span::styled("Status: ", theme.title()),
            Span::styled(status_text, status_style),
        ]),
        Line::from(vec![
            Span::styled("Duration: ", theme.title()),
            Span::raw(format!("{:.2} seconds", job.run_duration)),
        ]),
        Line::from(vec![
            Span::styled("Event Count: ", theme.title()),
            Span::raw(job.event_count.to_string()),
        ]),
        Line::from(vec![
            Span::styled("Scan Count: ", theme.title()),
            Span::raw(job.scan_count.to_string()),
        ]),
        Line::from(vec![
            Span::styled("Result Count: ", theme.title()),
            Span::raw(job.result_count.to_string()),
        ]),
        Line::from(vec![
            Span::styled("Disk Usage: ", theme.title()),
            Span::raw(format!("{} MB", job.disk_usage)),
        ]),
        Line::from(vec![
            Span::styled("Priority: ", theme.title()),
            Span::raw(job.priority.map_or("N/A".to_string(), |p| p.to_string())),
        ]),
        Line::from(vec![
            Span::styled("Label: ", theme.title()),
            Span::raw(job.label.as_deref().unwrap_or("N/A")),
        ]),
        Line::from(vec![
            Span::styled("Cursor Time: ", theme.title()),
            Span::raw(job.cursor_time.as_deref().unwrap_or("N/A")),
        ]),
        Line::from(vec![
            Span::styled("Finalized: ", theme.title()),
            Span::raw(if job.is_finalized { "Yes" } else { "No" }),
        ]),
    ];

    let details_paragraph = Paragraph::new(details)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Left);
    f.render_widget(details_paragraph, chunks[1]);
}
