//! Jobs CSV formatter.
//!
//! Responsibilities:
//! - Format search jobs as CSV.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::{build_csv_header, build_csv_row, escape_csv};
use anyhow::Result;
use splunk_client::SearchJobStatus;

/// Format jobs list as CSV.
pub fn format_jobs(jobs: &[SearchJobStatus]) -> Result<String> {
    let mut output = String::new();

    // Header
    output.push_str(&build_csv_header(&[
        "SID", "Done", "Progress", "Results", "Duration",
    ]));

    for job in jobs {
        output.push_str(&build_csv_row(&[
            escape_csv(&job.sid),
            escape_csv(if job.is_done { "Y" } else { "N" }),
            escape_csv(&format!("{:.1}", job.done_progress * 100.0)),
            escape_csv(&job.result_count.to_string()),
            escape_csv(&format!("{:.2}", job.run_duration)),
        ]));
    }

    Ok(output)
}

/// Format detailed job information as CSV.
pub fn format_job_details(job: &SearchJobStatus) -> Result<String> {
    let mut csv = String::new();

    // Header
    csv.push_str(&build_csv_header(&[
        "sid",
        "is_done",
        "is_finalized",
        "done_progress",
        "run_duration",
        "cursor_time",
        "scan_count",
        "event_count",
        "result_count",
        "disk_usage",
        "priority",
        "label",
    ]));

    // Data row
    let priority = job.priority.map_or("N/A".to_string(), |p| p.to_string());
    let cursor_time = job.cursor_time.as_deref().unwrap_or("N/A");
    let label = job.label.as_deref().unwrap_or("N/A");

    csv.push_str(&build_csv_row(&[
        escape_csv(&job.sid),
        escape_csv(&job.is_done.to_string()),
        escape_csv(&job.is_finalized.to_string()),
        escape_csv(&job.done_progress.to_string()),
        escape_csv(&job.run_duration.to_string()),
        escape_csv(cursor_time),
        escape_csv(&job.scan_count.to_string()),
        escape_csv(&job.event_count.to_string()),
        escape_csv(&job.result_count.to_string()),
        escape_csv(&job.disk_usage.to_string()),
        escape_csv(&priority),
        escape_csv(label),
    ]));

    Ok(csv)
}
