//! Log parsing health check endpoints.

use reqwest::Client;
use tracing::{debug, warn};

use crate::client::circuit_breaker::CircuitBreaker;
use crate::endpoints::search::{
    CreateJobOptions, OutputMode, create_job, get_results, wait_for_job,
};
use crate::error::Result;
use crate::metrics::MetricsCollector;
use crate::models::{LogParsingError, LogParsingHealth};

/// Search query for detecting log parsing errors in Splunk's internal logs.
///
/// This query searches the `_internal` index for parsing-related errors from
/// specific components, excluding successful parsing messages.
const PARSING_ERROR_SEARCH_QUERY: &str = r#"search index=_internal (component=TuningParser OR component=DateParserVerbose OR component=Parser) NOT message="parsing fully" | table _time source sourcetype message log_level component | head 1000"#;

/// Check log parsing health by searching for parsing errors in internal logs.
///
/// This function creates a search job to find log parsing errors within the
/// specified time window (default last 24 hours), waits for completion, and
/// returns structured results.
///
/// # Arguments
///
/// * `client` - HTTP client for making requests
/// * `base_url` - Base URL of the Splunk server
/// * `auth_token` - Authentication token
/// * `max_retries` - Maximum number of retry attempts for failed requests
///
/// # Returns
///
/// Returns a `LogParsingHealth` struct containing:
/// - `is_healthy`: true if no errors found, false otherwise
/// - `total_errors`: count of parsing errors found
/// - `errors`: vector of individual error details
/// - `time_window`: the search time window used
#[allow(clippy::too_many_arguments)]
pub async fn check_log_parsing_health(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<LogParsingHealth> {
    debug!("Checking log parsing health");

    let time_window = "-24h".to_string();

    let options = CreateJobOptions {
        earliest_time: Some(time_window.clone()),
        output_mode: Some(OutputMode::Json),
        ..Default::default()
    };

    let sid = create_job(
        client,
        base_url,
        auth_token,
        PARSING_ERROR_SEARCH_QUERY,
        &options,
        max_retries,
        metrics,
        circuit_breaker,
    )
    .await?;

    debug!("Created search job {} for parsing health check", sid);

    // Wait for job completion with reasonable timeout (60 seconds)
    let _status = wait_for_job(
        client,
        base_url,
        auth_token,
        &sid,
        500,
        60,
        max_retries,
        metrics,
        circuit_breaker,
    )
    .await?;

    debug!("Search job {} completed, fetching results", sid);

    let results = get_results(
        client,
        base_url,
        auth_token,
        &sid,
        None,
        None,
        OutputMode::Json,
        max_retries,
        metrics,
        circuit_breaker,
    )
    .await?;

    let endpoint = "/services/search/jobs/results";
    let mut parse_failures = 0usize;
    let errors: Vec<LogParsingError> = results
        .results
        .into_iter()
        .filter_map(
            |v| match serde_json::from_value::<LogParsingError>(v.clone()) {
                Ok(entry) => Some(entry),
                Err(e) => {
                    parse_failures += 1;
                    warn!(
                        "Failed to deserialize LogParsingError from {}: error={}, value_preview={}",
                        endpoint,
                        e,
                        serde_json::to_string(&v).unwrap_or_else(|_| format!("{:?}", v))
                    );
                    if let Some(m) = metrics {
                        m.record_deserialization_failure(endpoint, "LogParsingError");
                    }
                    None
                }
            },
        )
        .collect();

    let total_errors = errors.len();
    let is_healthy = total_errors == 0;

    if parse_failures > 0 {
        debug!(
            "Completed check_log_parsing_health with {} errors and {} parse failures",
            errors.len(),
            parse_failures
        );
    }

    debug!(
        "Log parsing health check complete: {} errors found, healthy: {}",
        total_errors, is_healthy
    );

    Ok(LogParsingHealth {
        is_healthy,
        total_errors,
        errors,
        time_window,
    })
}
