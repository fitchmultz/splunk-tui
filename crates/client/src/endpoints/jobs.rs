//! Job management endpoints.

use reqwest::Client;

use crate::client::circuit_breaker::CircuitBreaker;
use crate::endpoints::encode_path_segment;
use crate::endpoints::extract_entry_content;
use crate::endpoints::send_request_with_retry;
use crate::error::{ClientError, Result};
use crate::metrics::MetricsCollector;
use crate::models::{SearchJobListResponse, SearchJobStatus};

/// Get a specific search job.
#[allow(clippy::too_many_arguments)]
pub async fn get_job(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    sid: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<SearchJobStatus> {
    let encoded_sid = encode_path_segment(sid);
    let url = format!("{}/services/search/jobs/{}", base_url, encoded_sid);

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")]);
    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/search/jobs/{sid}",
        "GET",
        metrics,
        circuit_breaker,
    )
    .await?;

    let resp: serde_json::Value = response.json().await?;

    let content = extract_entry_content(&resp)?;
    serde_json::from_value(content.clone())
        .map_err(|e| ClientError::InvalidResponse(format!("Failed to parse job: {}", e)))
}

/// List all search jobs.
#[allow(clippy::too_many_arguments)]
pub async fn list_jobs(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    count: Option<usize>,
    offset: Option<usize>,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<Vec<SearchJobStatus>> {
    let url = format!("{}/services/search/jobs", base_url);

    let mut query_params: Vec<(String, String)> =
        vec![("output_mode".to_string(), "json".to_string())];

    if let Some(c) = count {
        query_params.push(("count".to_string(), c.to_string()));
    }
    if let Some(o) = offset {
        query_params.push(("offset".to_string(), o.to_string()));
    }

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&query_params);
    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/search/jobs",
        "GET",
        metrics,
        circuit_breaker,
    )
    .await?;

    let resp: SearchJobListResponse = response.json().await?;

    Ok(resp
        .entry
        .into_iter()
        .map(|e| SearchJobStatus {
            sid: e.content.sid,
            is_done: e.content.is_done,
            is_finalized: e.content.is_finalized,
            done_progress: e.content.done_progress,
            run_duration: e.content.runDuration,
            cursor_time: None,
            scan_count: e.content.scanCount,
            event_count: e.content.eventCount,
            result_count: e.content.resultCount,
            disk_usage: e.content.diskUsage,
            priority: e.content.priority,
            label: e.content.label.clone(),
        })
        .collect())
}

/// Cancel a search job.
#[allow(clippy::too_many_arguments)]
pub async fn cancel_job(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    sid: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<()> {
    let encoded_sid = encode_path_segment(sid);
    let url = format!("{}/services/search/jobs/{}/control", base_url, encoded_sid);

    let builder = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .form(&[("action", "cancel")]);
    let _response = send_request_with_retry(
        builder,
        max_retries,
        "/services/search/jobs/{sid}/control",
        "POST",
        metrics,
        circuit_breaker,
    )
    .await?;

    Ok(())
}

/// Delete a search job.
#[allow(clippy::too_many_arguments)]
pub async fn delete_job(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    sid: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<()> {
    let encoded_sid = encode_path_segment(sid);
    let url = format!("{}/services/search/jobs/{}", base_url, encoded_sid);

    let builder = client
        .delete(&url)
        .header("Authorization", format!("Bearer {}", auth_token));
    let _response = send_request_with_retry(
        builder,
        max_retries,
        "/services/search/jobs/{sid}",
        "DELETE",
        metrics,
        circuit_breaker,
    )
    .await?;

    Ok(())
}
