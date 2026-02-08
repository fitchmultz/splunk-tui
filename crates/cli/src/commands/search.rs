//! Search execution command implementation.
//!
//! Responsibilities:
//! - Execute SPL queries with configurable time bounds
//! - Support blocking (wait) and non-blocking execution modes
//! - Handle real-time search with optional window
//! - Apply search defaults from configuration when CLI flags not provided
//! - Format output via shared formatters
//! - Validate SPL syntax without executing searches
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
use clap::Subcommand;
use splunk_client::{SearchMode, SearchRequest};
use splunk_config::SearchDefaultConfig;
use std::path::PathBuf;
use tracing::info;

use crate::formatters::{OutputFormat, get_formatter, output_result};

/// Search subcommands.
#[derive(Subcommand)]
pub enum SearchCommand {
    /// Execute a search query (default)
    Execute {
        /// The search query to execute (e.g., 'search index=main | head 10')
        query: String,

        /// Wait for the search to complete before returning results
        #[arg(long)]
        wait: bool,

        /// Earliest time for the search (e.g., '-24h', '2024-01-01T00:00:00')
        #[arg(short, long, allow_hyphen_values = true)]
        earliest: Option<String>,

        /// Latest time for the search (e.g., 'now', '2024-01-02T00:00:00')
        #[arg(short, long, allow_hyphen_values = true)]
        latest: Option<String>,

        /// Maximum number of results to return
        #[arg(short, long)]
        count: Option<usize>,

        /// Run search in real-time mode
        #[arg(long)]
        realtime: bool,

        /// Real-time window in seconds (e.g., 60 for a 60-second window)
        #[arg(long, requires = "realtime")]
        realtime_window: Option<u64>,
    },

    /// Validate SPL syntax without executing the search
    Validate {
        /// The SPL query to validate (e.g., 'search index=main | stats count')
        ///
        /// Either provide the query as an argument or use --file to validate from a file.
        query: Option<String>,

        /// Path to a file containing SPL to validate
        ///
        /// The file should contain a single SPL query. Cannot be used with positional query argument.
        #[arg(long, value_name = "FILE", conflicts_with = "query")]
        file: Option<PathBuf>,

        /// Output in machine-readable JSON format
        ///
        /// JSON output includes structured error/warning details with line/column positions.
        #[arg(long)]
        json: bool,
    },
}

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

/// Validate SPL syntax without executing the search.
///
/// This function sends the SPL query to Splunk's parser endpoint
/// to check for syntax errors without actually running the search.
///
/// # Arguments
/// * `config` - Splunk configuration
/// * `query` - Optional SPL query string (from positional arg)
/// * `file` - Optional path to file containing SPL
/// * `output_format` - Output format string (json, table, csv, xml, ndjson)
/// * `output_file` - Optional file path to write results to
/// * `_cancel` - Cancellation token (unused for validation)
///
/// # Returns
/// Returns Ok(()) on successful validation, or an error if validation fails
/// or the SPL has syntax errors.
///
/// # Exit Codes
/// - 0: SPL is valid (may have warnings)
/// - 1: SPL has syntax errors
pub async fn run_validate(
    config: splunk_config::Config,
    query: Option<String>,
    file: Option<std::path::PathBuf>,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    _cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    use anyhow::Context;

    // Extract SPL query from file or argument
    let spl_query = match (query, file) {
        (Some(q), None) => q,
        (None, Some(path)) => tokio::fs::read_to_string(&path)
            .await
            .with_context(|| format!("Failed to read SPL file: {}", path.display()))?,
        (Some(_), Some(_)) => {
            // This shouldn't happen due to clap conflicts, but handle defensively
            anyhow::bail!("Cannot specify both query argument and --file");
        }
        (None, None) => {
            anyhow::bail!("Failed to validate: either a query argument or --file must be provided");
        }
    };

    info!("Validating SPL syntax");

    let client = crate::commands::build_client_from_config(&config)?;
    let result = client.validate_spl(&spl_query).await?;

    // Parse output format
    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);

    // Format and print results
    let output = formatter.format_validation_result(&result)?;
    output_result(&output, format, output_file.as_ref())?;

    // Exit with error code if validation failed
    if !result.valid {
        std::process::exit(1);
    }

    Ok(())
}
