//! Internal logs endpoint.

use reqwest::Client;
use tracing::debug;

use crate::endpoints::search::{CreateJobOptions, OutputMode, create_job, get_results};
use crate::error::Result;
use crate::models::LogEntry;

/// Get internal logs from Splunk.
pub async fn get_internal_logs(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    count: u64,
    earliest: Option<&str>,
    max_retries: usize,
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

    let sid = create_job(client, base_url, auth_token, &query, &options, max_retries).await?;

    let results = get_results(
        client,
        base_url,
        auth_token,
        &sid,
        Some(count),
        None,
        OutputMode::Json,
        max_retries,
    )
    .await?;

    let logs: Vec<LogEntry> = results
        .results
        .into_iter()
        .filter_map(|v| serde_json::from_value(v).ok())
        .collect();

    Ok(logs)
}
