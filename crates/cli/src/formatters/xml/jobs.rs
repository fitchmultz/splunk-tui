//! Jobs XML formatter.
//!
//! Responsibilities:
//! - Format job lists and job details as XML.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::escape_xml;
use anyhow::Result;
use splunk_client::SearchJobStatus;

/// Format jobs as XML.
pub fn format_jobs(jobs: &[SearchJobStatus]) -> Result<String> {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<jobs>\n");

    for job in jobs {
        xml.push_str("  <job>\n");
        xml.push_str(&format!("    <sid>{}</sid>\n", escape_xml(&job.sid)));
        xml.push_str(&format!("    <done>{}</done>\n", job.is_done));
        xml.push_str(&format!(
            "    <progress>{:.1}</progress>\n",
            job.done_progress * 100.0
        ));
        xml.push_str(&format!("    <results>{}</results>\n", job.result_count));
        xml.push_str(&format!(
            "    <duration>{:.2}</duration>\n",
            job.run_duration
        ));
        xml.push_str("  </job>\n");
    }

    xml.push_str("</jobs>");
    Ok(xml)
}

/// Format detailed job information as XML.
pub fn format_job_details(job: &SearchJobStatus) -> Result<String> {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<job>\n");

    xml.push_str(&format!("  <sid>{}</sid>\n", escape_xml(&job.sid)));
    xml.push_str(&format!("  <done>{}</done>\n", job.is_done));
    xml.push_str(&format!("  <finalized>{}</finalized>\n", job.is_finalized));
    xml.push_str(&format!(
        "  <progress>{:.2}</progress>\n",
        job.done_progress * 100.0
    ));
    xml.push_str(&format!("  <duration>{:.2}</duration>\n", job.run_duration));
    xml.push_str(&format!("  <scanCount>{}</scanCount>\n", job.scan_count));
    xml.push_str(&format!("  <eventCount>{}</eventCount>\n", job.event_count));
    xml.push_str(&format!(
        "  <resultCount>{}</resultCount>\n",
        job.result_count
    ));
    xml.push_str(&format!("  <diskUsage>{}</diskUsage>\n", job.disk_usage));

    if let Some(priority) = job.priority {
        xml.push_str(&format!("  <priority>{}</priority>\n", priority));
    }
    if let Some(ref cursor_time) = job.cursor_time {
        xml.push_str(&format!(
            "  <cursorTime>{}</cursorTime>\n",
            escape_xml(cursor_time)
        ));
    }
    if let Some(ref label) = job.label {
        xml.push_str(&format!("  <label>{}</label>\n", escape_xml(label)));
    }

    xml.push_str("</job>");
    Ok(xml)
}
