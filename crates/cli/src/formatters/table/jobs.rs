//! Jobs table formatter.
//!
//! Responsibilities:
//! - Format job lists and job details as tab-separated tables.
//!
//! Does NOT handle:
//! - Other resource types.

use anyhow::Result;
use splunk_client::SearchJobStatus;

/// Format jobs as a tab-separated table.
pub fn format_jobs(jobs: &[SearchJobStatus]) -> Result<String> {
    let mut output = String::new();

    if jobs.is_empty() {
        return Ok("No jobs found.".to_string());
    }

    // Header
    output.push_str("SID\tDone\tProgress\tResults\tDuration\n");

    for job in jobs {
        output.push_str(&format!(
            "{}\t{}\t{:.1}%\t{}\t{:.2}s\n",
            job.sid,
            if job.is_done { "Y" } else { "N" },
            job.done_progress * 100.0,
            job.result_count,
            job.run_duration
        ));
    }

    Ok(output)
}

/// Format detailed job information.
pub fn format_job_details(job: &SearchJobStatus) -> Result<String> {
    let mut output = String::new();

    output.push_str("--- Job Details ---\n");
    output.push_str(&format!("SID: {}\n", job.sid));

    // Status with progress
    let status_text = if job.is_done {
        "Done"
    } else if job.done_progress > 0.0 {
        &format!("Running ({:.0}%)", job.done_progress * 100.0)
    } else {
        "Running"
    };
    output.push_str(&format!("Status: {}\n", status_text));

    // Duration
    output.push_str(&format!("Duration: {:.2} seconds\n", job.run_duration));

    // Counts
    output.push_str(&format!("Event Count: {}\n", job.event_count));
    output.push_str(&format!("Scan Count: {}\n", job.scan_count));
    output.push_str(&format!("Result Count: {}\n", job.result_count));

    // Disk usage
    output.push_str(&format!("Disk Usage: {} MB\n", job.disk_usage));

    // Optional fields
    output.push_str(&format!(
        "Priority: {}\n",
        job.priority.map_or("N/A".to_string(), |p| p.to_string())
    ));
    output.push_str(&format!(
        "Label: {}\n",
        job.label.as_deref().unwrap_or("N/A")
    ));
    output.push_str(&format!(
        "Cursor Time: {}\n",
        job.cursor_time.as_deref().unwrap_or("N/A")
    ));
    output.push_str(&format!(
        "Finalized: {}\n",
        if job.is_finalized { "Yes" } else { "No" }
    ));

    Ok(output)
}
