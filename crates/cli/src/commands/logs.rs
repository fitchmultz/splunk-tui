//! Internal logs command implementation with tail support.
//!
//! Responsibilities:
//! - Fetch internal logs with optional count limiting and time filtering
//! - Support continuous tail mode with cursor tracking
//! - Handle deduplication in tail mode using serial/content hash
//! - Format output via shared formatters including streaming format
//!
//! Does NOT handle:
//! - Log file management or rotation
//! - Direct REST API calls (handled by client crate)
//! - Output formatting details (see formatters module)
//!
//! Invariants:
//! - Tail mode is incompatible with file output (--output-file)
//! - Cursor tracks position using time, index_time, and serial/hash
//! - Deduplication ensures no duplicate entries in tail output
//! - Time filtering uses Splunk's standard time format

use anyhow::{Context, Result};
use splunk_client::models::LogEntry;
use splunk_client::models::logs::sort_logs_newest_first;
use splunk_config::constants::DEFAULT_LOGS_TAIL_POLL_INTERVAL_SECS;
use tokio::time::{Duration, sleep};
use tracing::info;

use crate::cancellation::Cancelled;
use crate::formatters::{Formatter, OutputFormat, get_formatter, write_to_file};

/// Cursor for tracking log position during tailing.
#[derive(Debug, Clone)]
struct LogCursor {
    time: String,
    index_time: String,
    serial: Option<usize>,
    content_hash: Option<usize>, // For entries without serial
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
        // Same index_time: compare by serial or content hash
        match (self.serial, entry.serial) {
            (Some(s), Some(e)) => e > s,
            (None, Some(_)) => {
                // Cursor has no serial but entry does - entry is newer
                true
            }
            (Some(_), None) => {
                // Cursor has serial but entry doesn't - need content comparison
                // Compare by content hash to detect if same entry
                let entry_hash = entry.content_hash();
                self.content_hash != Some(entry_hash)
            }
            (None, None) => {
                // Neither has serial - compare by content hash
                let entry_hash = entry.content_hash();
                self.content_hash != Some(entry_hash)
            }
        }
    }

    /// Create a new cursor from a log entry.
    fn from_entry(entry: &LogEntry) -> Self {
        LogCursor {
            time: entry.time.clone(),
            index_time: entry.index_time.clone(),
            serial: entry.serial,
            content_hash: entry.serial.is_none().then(|| entry.content_hash()),
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
            "Failed to use output file in tail mode: tail mode does not support file output"
        );
    }

    let mut client = crate::commands::build_client_from_config(&config)?;
    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);

    if tail {
        run_tail_mode(&mut client, count, &earliest, formatter.as_ref(), cancel).await
    } else {
        run_normal_mode(
            &mut client,
            count,
            &earliest,
            formatter.as_ref(),
            output_file.as_ref(),
            format,
            cancel,
        )
        .await
    }
}

/// Run in continuous tail mode, polling for new logs.
async fn run_tail_mode(
    client: &mut splunk_client::SplunkClient,
    count: usize,
    earliest: &str,
    formatter: &dyn Formatter,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Tailing internal logs...");
    let mut cursor: Option<LogCursor> = None;
    let mut is_first = true;

    loop {
        let fetch_result = fetch_logs_batch(client, count, earliest, &cursor, cancel).await;

        match fetch_result {
            Ok(logs) => {
                process_log_batch(&logs, &mut cursor, formatter, &mut is_first).await?;
            }
            Err(e) => {
                eprintln!("Failed to fetch logs: {}", e);
            }
        }

        tokio::select! {
            _ = sleep(Duration::from_secs(DEFAULT_LOGS_TAIL_POLL_INTERVAL_SECS)) => {}
            _ = cancel.cancelled() => return Err(Cancelled.into()),
        }
    }
}

/// Fetch a batch of logs from the Splunk server.
async fn fetch_logs_batch(
    client: &mut splunk_client::SplunkClient,
    count: usize,
    earliest: &str,
    cursor: &Option<LogCursor>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<Vec<LogEntry>> {
    let time_filter = cursor.as_ref().map(|c| c.time.as_str()).unwrap_or(earliest);

    tokio::select! {
        res = client.get_internal_logs(count, Some(time_filter)) => res.map_err(|e| e.into()),
        _ = cancel.cancelled() => Err(Cancelled.into()),
    }
}

/// Process a batch of logs, filtering already-seen entries and updating cursor.
async fn process_log_batch(
    logs: &[LogEntry],
    cursor: &mut Option<LogCursor>,
    formatter: &dyn Formatter,
    is_first: &mut bool,
) -> Result<()> {
    if logs.is_empty() {
        return Ok(());
    }

    // Filter out logs we've already seen
    let mut new_logs: Vec<_> = match cursor {
        Some(c) => logs.iter().filter(|l| c.is_after(l)).cloned().collect(),
        None => logs.to_vec(),
    };

    if new_logs.is_empty() {
        return Ok(());
    }

    // Ensure deterministic ordering before cursor update (defensive)
    // API returns sorted results, but client-side filtering could
    // theoretically disrupt ordering. Explicit sort adds resilience
    // against future changes to filtering or data flow.
    sort_logs_newest_first(&mut new_logs);

    // Update cursor to the NEWEST new log (first in sorted descending list)
    // This prevents re-querying same-timestamp events on next poll
    if let Some(newest_new) = new_logs.first() {
        *cursor = Some(LogCursor::from_entry(newest_new));
    }

    // Print new logs using streaming formatter (sorted newest-first, which is correct for tailing)
    let output = formatter.format_logs_streaming(&new_logs, *is_first)?;
    if !output.is_empty() {
        print!("{}", output);
    }
    *is_first = false;

    Ok(())
}

/// Run in normal mode, fetching logs once and outputting.
async fn run_normal_mode(
    client: &mut splunk_client::SplunkClient,
    count: usize,
    earliest: &str,
    formatter: &dyn Formatter,
    output_file: Option<&std::path::PathBuf>,
    format: OutputFormat,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Fetching internal logs...");
    let logs: Vec<LogEntry> = tokio::select! {
        res = client.get_internal_logs(count, Some(earliest)) => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    let output = formatter.format_logs(&logs)?;
    if let Some(path) = output_file {
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

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_log_entry(time: &str, index_time: &str, serial: Option<usize>) -> LogEntry {
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
            content_hash: None,
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
            content_hash: None,
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
            content_hash: None,
        };

        let entry_no_serial =
            make_log_entry("2025-01-24T12:00:00.000Z", "2025-01-24T12:00:01.000Z", None);

        // Cursor has serial but entry doesn't - use content hash comparison
        // Since content differs, entry should be considered new
        assert!(cursor.is_after(&entry_no_serial));
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
            content_hash: None,
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
            content_hash: None,
        };

        let entry_empty_idx = make_log_entry("2025-01-24T12:00:00.000Z", "", None);

        // Empty string is "less than" any valid timestamp, so entry is considered older
        assert!(!cursor.is_after(&entry_empty_idx));

        // Reverse: cursor has empty index_time, entry has valid
        let cursor_empty = LogCursor {
            time: "2025-01-24T12:00:00.000Z".to_string(),
            index_time: "".to_string(),
            serial: None,
            content_hash: None,
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
            content_hash: None,
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

    #[test]
    fn test_cursor_both_missing_serial_same_content() {
        // Both cursor and entry lack serial, same content - should NOT be after
        let entry = make_log_entry("2025-01-24T12:00:00.000Z", "2025-01-24T12:00:01.000Z", None);
        let cursor = LogCursor::from_entry(&entry);

        // Same entry - should NOT be after
        assert!(!cursor.is_after(&entry));
    }

    #[test]
    fn test_cursor_both_missing_serial_different_content() {
        // Both cursor and entry lack serial, different content - should be after
        let cursor_entry = LogEntry {
            time: "2025-01-24T12:00:00.000Z".to_string(),
            index_time: "2025-01-24T12:00:01.000Z".to_string(),
            serial: None,
            level: "INFO".to_string(),
            component: "test".to_string(),
            message: "first message".to_string(),
        };
        let cursor = LogCursor::from_entry(&cursor_entry);

        let new_entry = LogEntry {
            time: "2025-01-24T12:00:00.000Z".to_string(),
            index_time: "2025-01-24T12:00:01.000Z".to_string(),
            serial: None,
            level: "INFO".to_string(),
            component: "test".to_string(),
            message: "different message".to_string(),
        };

        // Different content - should be after
        assert!(cursor.is_after(&new_entry));
    }

    #[test]
    fn test_cursor_missing_serial_entry_has_serial() {
        // Cursor lacks serial but entry has it - entry is newer
        let cursor = LogCursor {
            time: "2025-01-24T12:00:00.000Z".to_string(),
            index_time: "2025-01-24T12:00:01.000Z".to_string(),
            serial: None,
            content_hash: None,
        };

        let entry_with_serial = make_log_entry(
            "2025-01-24T12:00:00.000Z",
            "2025-01-24T12:00:01.000Z",
            Some(100),
        );

        // Entry has serial but cursor doesn't - entry is newer
        assert!(cursor.is_after(&entry_with_serial));
    }

    #[test]
    fn test_cursor_from_entry_captures_content_hash_when_no_serial() {
        let entry = LogEntry {
            time: "2025-01-24T12:00:00.000Z".to_string(),
            index_time: "2025-01-24T12:00:01.000Z".to_string(),
            serial: None,
            level: "INFO".to_string(),
            component: "test".to_string(),
            message: "test message".to_string(),
        };

        let cursor = LogCursor::from_entry(&entry);

        assert_eq!(cursor.time, "2025-01-24T12:00:00.000Z");
        assert_eq!(cursor.index_time, "2025-01-24T12:00:01.000Z");
        assert_eq!(cursor.serial, None);
        assert!(cursor.content_hash.is_some());
        assert_eq!(cursor.content_hash, Some(entry.content_hash()));
    }

    #[test]
    fn test_cursor_from_entry_no_content_hash_when_has_serial() {
        let entry = make_log_entry(
            "2025-01-24T12:00:00.000Z",
            "2025-01-24T12:00:01.000Z",
            Some(42),
        );

        let cursor = LogCursor::from_entry(&entry);

        assert_eq!(cursor.time, "2025-01-24T12:00:00.000Z");
        assert_eq!(cursor.index_time, "2025-01-24T12:00:01.000Z");
        assert_eq!(cursor.serial, Some(42));
        assert!(cursor.content_hash.is_none());
    }
}
