//! Internal logs endpoint.

use reqwest::Client;
use tracing::{debug, warn};

use crate::client::circuit_breaker::CircuitBreaker;
use crate::endpoints::search::{CreateJobOptions, OutputMode, create_job, get_results};
use crate::error::Result;
use crate::metrics::MetricsCollector;
use crate::models::LogEntry;

/// Get internal logs from Splunk.
#[allow(clippy::too_many_arguments)]
pub async fn get_internal_logs(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    count: usize,
    earliest: Option<&str>,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<Vec<LogEntry>> {
    debug!("Fetching internal logs (count={})", count);

    let query = format!(
        "search index=_internal | sort -_time, -_indextime, -_serial | head {}",
        count
    );

    let options = CreateJobOptions {
        earliest_time: earliest.map(|s| s.to_string()),
        latest_time: Some("now".to_string()),
        output_mode: Some(OutputMode::Json),
        exec_time: Some(30),
        wait: Some(true),
        ..Default::default()
    };

    let sid = create_job(
        client,
        base_url,
        auth_token,
        &query,
        &options,
        max_retries,
        metrics,
        circuit_breaker,
    )
    .await?;

    let results = get_results(
        client,
        base_url,
        auth_token,
        &sid,
        Some(count),
        None,
        OutputMode::Json,
        max_retries,
        metrics,
        circuit_breaker,
    )
    .await?;

    let endpoint = "/services/search/jobs/results";
    let mut parse_failures = 0usize;
    let logs: Vec<LogEntry> = results
        .results
        .into_iter()
        .filter_map(|v| match serde_json::from_value::<LogEntry>(v.clone()) {
            Ok(entry) => Some(entry),
            Err(e) => {
                parse_failures += 1;
                warn!(
                    "Failed to deserialize LogEntry from {}: error={}, value_preview={}",
                    endpoint,
                    e,
                    serde_json::to_string(&v).unwrap_or_else(|_| format!("{:?}", v))
                );
                if let Some(m) = metrics {
                    m.record_deserialization_failure(endpoint, "LogEntry");
                }
                None
            }
        })
        .collect();

    if parse_failures > 0 {
        debug!(
            "Completed get_internal_logs with {} entries and {} parse failures",
            logs.len(),
            parse_failures
        );
    }

    Ok(logs)
}
