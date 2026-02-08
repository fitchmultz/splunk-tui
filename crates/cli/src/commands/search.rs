//! Search execution command implementation.
//!
//! Responsibilities:
//! - Execute SPL queries with configurable time bounds
//! - Support blocking (wait) and non-blocking execution modes
//! - Handle real-time search with optional window
//! - Apply search defaults from configuration when CLI flags not provided
//! - Format output via shared formatters
//!
//! Does NOT handle:
//! - Saved search management (see saved_searches module)
//! - Direct REST API calls (handled by client crate)
//! - Output formatting details (see formatters module)
//!
//! Invariants:
//! - Search queries are passed through without modification
//! - Time bounds default to configuration values or -24h/now
//! - Max results default to configuration or 100
//! - Progress callbacks are only used in non-quiet mode

use anyhow::Result;
use splunk_client::{SearchMode, SearchRequest};
use splunk_config::SearchDefaultConfig;
use tracing::info;

use crate::formatters::{OutputFormat, get_formatter, output_result};

#[allow(clippy::too_many_arguments)]
pub async fn run(
    config: splunk_config::Config,
    query: String,
    wait: bool,
    earliest: Option<&str>,
    latest: Option<&str>,
    max_results: Option<usize>,
    search_defaults: &SearchDefaultConfig,
    output_format: &str,
    quiet: bool,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
    realtime: bool,
    realtime_window: Option<u64>,
) -> Result<()> {
    info!("Executing search: {}", query);

    // Apply search defaults when CLI flags are not provided
    let earliest = earliest.unwrap_or(&search_defaults.earliest_time);
    let latest = latest.unwrap_or(&search_defaults.latest_time);
    let max_results = max_results.unwrap_or(search_defaults.max_results);

    // Determine search mode based on realtime flag
    let search_mode = if realtime {
        Some(SearchMode::Realtime)
    } else {
        Some(SearchMode::Normal)
    };

    let client = crate::commands::build_client_from_config(&config)?;

    info!("Connecting to {}", client.base_url());

    // Build the search request with common parameters
    let mut request = SearchRequest::new(&query, wait)
        .time_bounds(earliest, latest)
        .max_results(max_results)
        .search_mode(search_mode.unwrap_or(SearchMode::Normal));
    if let Some(window) = realtime_window {
        request = request.realtime_window(window);
    }

    let (results, _sid, _total) = if wait {
        let progress = crate::progress::SearchProgress::new(!quiet, "Waiting for search");

        let mut on_progress = |done_progress: f64| {
            progress.set_fraction(done_progress);
        };

        let search_result = cancellable!(
            client
                .search_with_progress(request, if quiet { None } else { Some(&mut on_progress) },),
            cancel
        )?;

        progress.finish();
        search_result
    } else {
        cancellable!(client.search_with_progress(request, None), cancel)?
    };

    // Parse output format
    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);

    // Format and print results
    let output = formatter.format_search_results(&results)?;
    output_result(&output, format, output_file.as_ref())?;

    Ok(())
}
