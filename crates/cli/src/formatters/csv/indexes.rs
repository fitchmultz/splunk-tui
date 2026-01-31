//! Indexes CSV formatter.
//!
//! Responsibilities:
//! - Format index lists as CSV.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::{build_csv_header, build_csv_row, escape_csv, format_opt_str};
use anyhow::Result;
use splunk_client::Index;

/// Format indexes as CSV.
pub fn format_indexes(indexes: &[Index], detailed: bool) -> Result<String> {
    if indexes.is_empty() {
        return Ok(String::new());
    }

    let mut output = String::new();

    // Header
    if detailed {
        output.push_str(&build_csv_header(&[
            "Name",
            "SizeMB",
            "Events",
            "MaxSizeMB",
            "RetentionSecs",
            "HomePath",
            "ColdPath",
            "ThawedPath",
        ]));
    } else {
        output.push_str(&build_csv_header(&[
            "Name",
            "SizeMB",
            "Events",
            "MaxSizeMB",
        ]));
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
            output.push_str(&build_csv_row(&[
                escape_csv(&index.name),
                escape_csv(&index.current_db_size_mb.to_string()),
                escape_csv(&index.total_event_count.to_string()),
                escape_csv(&max_size),
                escape_csv(&retention),
                format_opt_str(index.home_path.as_deref(), "N/A"),
                format_opt_str(index.cold_db_path.as_deref(), "N/A"),
                format_opt_str(index.thawed_path.as_deref(), "N/A"),
            ]));
        } else {
            output.push_str(&build_csv_row(&[
                escape_csv(&index.name),
                escape_csv(&index.current_db_size_mb.to_string()),
                escape_csv(&index.total_event_count.to_string()),
                escape_csv(&max_size),
            ]));
        }
    }

    Ok(output)
}
