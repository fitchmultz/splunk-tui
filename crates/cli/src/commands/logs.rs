//! Logs command implementation with tail support.

use anyhow::{Context, Result};
use splunk_client::SplunkClient;
use splunk_client::models::LogEntry;
use tokio::time::{Duration, sleep};
use tracing::info;

use crate::cancellation::Cancelled;
use crate::formatters::{OutputFormat, get_formatter, write_to_file};

/// Cursor for tracking log position during tailing.
#[derive(Debug, Clone)]
struct LogCursor {
    time: String,
    index_time: String,
    serial: Option<u64>,
}

impl LogCursor {
    /// Returns true if * log entry is NEWER than this cursor.
    fn is_after(&self, entry: &LogEntry) -> bool {
        // Compare by timestamp first
        if entry.time != self.time {
            return entry.time > self.time;
        }
        // Same timestamp: compare by index_time
        if entry.index_time != self.index_time {
            return entry.index_time > self.index_time;
        }
        // Same index_time: compare by serial
        match (self.serial, entry.serial) {
            (Some(s), Some(e)) => e > s,
            _ => false, // Conservative: if no serial, assume already seen
        }
    }

    /// Create a new cursor from a log entry.
    fn from_entry(entry: &LogEntry) -> Self {
        LogCursor {
            time: entry.time.clone(),
            index_time: entry.index_time.clone(),
            serial: entry.serial,
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn run(
    config: splunk_config::Config,
    count: usize,
    earliest: String,
    tail: bool,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    // Tail mode is incompatible with file output
    if tail && output_file.is_some() {
        anyhow::bail!(
            "--output-file cannot be used with --tail mode. Tail mode streams output continuously."
        );
    }

    let auth_strategy = crate::commands::convert_auth_strategy(&config.auth.strategy);

    let mut client = SplunkClient::builder()
        .base_url(config.connection.base_url)
        .auth_strategy(auth_strategy)
        .skip_verify(config.connection.skip_verify)
        .timeout(config.connection.timeout)
        .session_ttl_seconds(config.connection.session_ttl_seconds)
        .session_expiry_buffer_seconds(config.connection.session_expiry_buffer_seconds)
        .build()?;

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);

    if tail {
        info!("Tailing internal logs...");
        let mut cursor: Option<LogCursor> = None;

        loop {
            let fetch_result: Result<Vec<LogEntry>> = tokio::select! {
                res = client.get_internal_logs(count as u64, Some(cursor.as_ref().map(|c| c.time.as_str()).unwrap_or(&earliest))) => res.map_err(|e| e.into()),
                _ = cancel.cancelled() => return Err(Cancelled.into()),
            };

            match fetch_result {
                Ok(logs) => {
                    let logs: Vec<LogEntry> = logs;
                    if !logs.is_empty() {
                        // Filter out logs we've already seen
                        let new_logs: Vec<_> = if let Some(ref cursor) = cursor {
                            logs.into_iter().filter(|l| cursor.is_after(l)).collect()
                        } else {
                            logs
                        };

                        if !new_logs.is_empty() {
                            // Update cursor to the NEWEST new log (first in sorted descending list)
                            // This prevents re-querying same-timestamp events on next poll
                            if let Some(newest_new) = new_logs.first() {
                                cursor = Some(LogCursor::from_entry(newest_new));
                            }

                            // Print new logs (sorted newest-first, which is correct for tailing)
                            let output = formatter.format_logs(&new_logs)?;
                            if !output.trim().is_empty() {
                                println!("{}", output.trim());
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error fetching logs: {}", e);
                }
            }

            tokio::select! {
                _ = sleep(Duration::from_secs(2)) => {}
                _ = cancel.cancelled() => return Err(Cancelled.into()),
            }
        }
    } else {
        info!("Fetching internal logs...");
        let logs: Vec<LogEntry> = tokio::select! {
            res = client.get_internal_logs(count as u64, Some(&earliest)) => res?,
            _ = cancel.cancelled() => return Err(Cancelled.into()),
        };
        let output = formatter.format_logs(&logs)?;
        if let Some(ref path) = output_file {
            write_to_file(&output, path)
                .with_context(|| format!("Failed to write output to {}", path.display()))?;
            eprintln!(
                "Results written to {} ({:?} format)",
                path.display(),
                format
            );
        } else {
            println!("{}", output);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_log_entry(time: &str, index_time: &str, serial: Option<u64>) -> LogEntry {
        LogEntry {
            time: time.to_string(),
            index_time: index_time.to_string(),
            serial,
            level: "INFO".to_string(),
            component: "test".to_string(),
            message: "test message".to_string(),
        }
    }

    #[test]
    fn test_cursor_is_after_same_timestamp_different_serial() {
        let cursor = LogCursor {
            time: "2025-01-24T12:00:00.000Z".to_string(),
            index_time: "2025-01-24T12:00:01.000Z".to_string(),
            serial: Some(100),
        };

        let entry1 = make_log_entry(
            "2025-01-24T12:00:00.000Z",
            "2025-01-24T12:00:01.001Z",
            Some(101),
        );
        let entry2 = make_log_entry(
            "2025-01-24T12:00:00.000Z",
            "2025-01-24T12:00:01.000Z",
            Some(99),
        );

        assert!(cursor.is_after(&entry1)); // Newer by index_time
        assert!(!cursor.is_after(&entry2)); // Older by serial
    }

    #[test]
    fn test_cursor_is_after_different_timestamp() {
        let cursor = LogCursor {
            time: "2025-01-24T12:00:00.000Z".to_string(),
            index_time: "2025-01-24T12:00:01.000Z".to_string(),
            serial: Some(100),
        };

        let newer_entry = make_log_entry(
            "2025-01-24T12:00:01.000Z",
            "2025-01-24T12:00:02.000Z",
            Some(50),
        );

        assert!(cursor.is_after(&newer_entry));
    }

    #[test]
    fn test_cursor_with_missing_serial() {
        let cursor = LogCursor {
            time: "2025-01-24T12:00:00.000Z".to_string(),
            index_time: "2025-01-24T12:00:01.000Z".to_string(),
            serial: Some(100),
        };

        let entry_no_serial =
            make_log_entry("2025-01-24T12:00:00.000Z", "2025-01-24T12:00:01.000Z", None);

        // Conservative: if no serial, assume it's same entry
        assert!(!cursor.is_after(&entry_no_serial));
    }

    #[test]
    fn test_cursor_from_entry() {
        let entry = make_log_entry(
            "2025-01-24T12:00:00.000Z",
            "2025-01-24T12:00:01.000Z",
            Some(42),
        );

        let cursor = LogCursor::from_entry(&entry);
        assert_eq!(cursor.time, "2025-01-24T12:00:00.000Z");
        assert_eq!(cursor.index_time, "2025-01-24T12:00:01.000Z");
        assert_eq!(cursor.serial, Some(42));
    }

    #[test]
    fn test_cursor_is_after_by_index_time_only() {
        let cursor = LogCursor {
            time: "2025-01-24T12:00:00.000Z".to_string(),
            index_time: "2025-01-24T12:00:01.000Z".to_string(),
            serial: Some(100),
        };

        // Same time, same serial, different index_time (edge case)
        let entry_same_time_serial = make_log_entry(
            "2025-01-24T12:00:00.000Z",
            "2025-01-24T12:00:02.000Z",
            Some(100),
        );

        assert!(cursor.is_after(&entry_same_time_serial));
    }

    #[test]
    fn test_cursor_with_empty_index_time() {
        // Cursor has valid index_time, entry has empty (missing) index_time
        let cursor = LogCursor {
            time: "2025-01-24T12:00:00.000Z".to_string(),
            index_time: "2025-01-24T12:00:01.000Z".to_string(),
            serial: Some(100),
        };

        let entry_empty_idx = make_log_entry("2025-01-24T12:00:00.000Z", "", None);

        // Empty string is "less than" any valid timestamp, so entry is considered older
        assert!(!cursor.is_after(&entry_empty_idx));

        // Reverse: cursor has empty index_time, entry has valid
        let cursor_empty = LogCursor {
            time: "2025-01-24T12:00:00.000Z".to_string(),
            index_time: "".to_string(),
            serial: None,
        };

        let entry_valid_idx = make_log_entry(
            "2025-01-24T12:00:00.000Z",
            "2025-01-24T12:00:01.000Z",
            Some(50),
        );

        // Valid index_time is "greater than" empty string, so entry is considered newer
        assert!(cursor_empty.is_after(&entry_valid_idx));
    }

    #[test]
    fn test_cursor_update_uses_newest_entry() {
        // This test verifies fix for duplicate bug:
        // When we have multiple new logs, cursor should update to the NEWEST one (first in list),
        // not the oldest. This prevents re-querying same-timestamp events.

        let cursor = LogCursor {
            time: "2025-01-24T12:00:00.000Z".to_string(),
            index_time: "2025-01-24T12:00:01.000Z".to_string(),
            serial: Some(10),
        };

        // Simulate new logs returned from query (sorted descending by time, index_time, serial)
        let new_logs = [
            make_log_entry(
                "2025-01-24T12:00:01.000Z",
                "2025-01-24T12:00:02.000Z",
                Some(30),
            ),
            make_log_entry(
                "2025-01-24T12:00:01.000Z",
                "2025-01-24T12:00:02.000Z",
                Some(20),
            ),
            make_log_entry(
                "2025-01-24T12:00:01.000Z",
                "2025-01-24T12:00:02.000Z",
                Some(15),
            ),
        ];

        // All should be "after" current cursor
        assert!(cursor.is_after(&new_logs[0]));
        assert!(cursor.is_after(&new_logs[1]));
        assert!(cursor.is_after(&new_logs[2]));

        // Cursor should update to the FIRST (newest) entry
        let new_cursor = LogCursor::from_entry(&new_logs[0]);
        assert_eq!(new_cursor.serial, Some(30));

        // On next query, none of these entries should be "after" new cursor
        assert!(!new_cursor.is_after(&new_logs[0]));
        assert!(!new_cursor.is_after(&new_logs[1]));
        assert!(!new_cursor.is_after(&new_logs[2]));
    }
}
