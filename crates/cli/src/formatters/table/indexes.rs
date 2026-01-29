//! Indexes table formatter.
//!
//! Responsibilities:
//! - Format index lists as tab-separated tables.
//!
//! Does NOT handle:
//! - Other resource types.
//! - Pagination (handled in mod.rs).

use anyhow::Result;
use splunk_client::Index;

/// Format indexes as a tab-separated table.
pub fn format_indexes(indexes: &[Index], detailed: bool) -> Result<String> {
    let mut output = String::new();

    if indexes.is_empty() {
        return Ok("No indexes found.".to_string());
    }

    // Header
    if detailed {
        output.push_str("Name\tSize (MB)\tEvents\tMax Size (MB)\tRetention (s)\tHome Path\tCold Path\tThawed Path\n");
    } else {
        output.push_str("Name\tSize (MB)\tEvents\tMax Size (MB)\n");
    }

    for index in indexes {
        let max_size = index
            .max_total_data_size_mb
            .map(|v: u64| v.to_string())
            .unwrap_or_else(|| "N/A".to_string());

        if detailed {
            let retention = index
                .frozen_time_period_in_secs
                .map(|v| v.to_string())
                .unwrap_or_else(|| "N/A".to_string());
            let home_path = index.home_path.as_deref().unwrap_or("N/A");
            let cold_path = index.cold_db_path.as_deref().unwrap_or("N/A");
            let thawed_path = index.thawed_path.as_deref().unwrap_or("N/A");
            output.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                index.name,
                index.current_db_size_mb,
                index.total_event_count,
                max_size,
                retention,
                home_path,
                cold_path,
                thawed_path
            ));
        } else {
            output.push_str(&format!(
                "{}\t{}\t{}\t{}\n",
                index.name, index.current_db_size_mb, index.total_event_count, max_size
            ));
        }
    }

    Ok(output)
}
