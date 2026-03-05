//! Search job operations.
//!
//! This module provides endpoints for creating and managing search jobs.
//!
//! # What this module handles:
//! - Creating search jobs
//! - Getting job status
//! - Waiting for job completion
//! - Retrieving search results
//!
//! # What this module does NOT handle:
//! - Saved search management (see [`super::saved`])
//! - SPL validation (see [`super::validate`])

use reqwest::Client;
use tracing::debug;

use crate::redact_query;

use crate::client::circuit_breaker::CircuitBreaker;
use crate::endpoints::encode_path_segment;
use crate::endpoints::{extract_entry_content, send_request_with_retry};
use crate::error::{ClientError, Result};
use crate::metrics::MetricsCollector;
use crate::models::{SearchJobResults, SearchJobStatus};

use super::types::{CreateJobOptions, OutputMode};

/// Create a new search job.
#[allow(clippy::too_many_arguments)]
pub async fn create_job(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    query: &str,
    options: &CreateJobOptions,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<String> {
    // Security: Log only redacted query to avoid exposing sensitive data (tokens, PII, etc.)
    debug!("Creating search job: {}", redact_query(query));

    let url = format!("{}/services/search/jobs", base_url);

    let mut form_data: Vec<(&str, String)> = vec![("search", query.to_string())];

    // Default to JSON output if not specified
    let output_mode = options.output_mode.unwrap_or_default();
    form_data.push(("output_mode", output_mode.to_string()));

    if let Some(wait) = options.wait {
        form_data.push((
            "wait",
            if wait {
                "1".to_string()
            } else {
                "0".to_string()
            },
        ));
    }
    if let Some(exec_time) = options.exec_time {
        form_data.push(("exec_time", exec_time.to_string()));
    }
    // Only add time bounds if they are non-empty
    // Empty strings can cause 400 errors from Splunk
    if let Some(earliest) = &options.earliest_time {
        if !earliest.trim().is_empty() {
            form_data.push(("earliest_time", earliest.clone()));
        } else {
            debug!("Skipping empty earliest_time parameter");
        }
    }
    if let Some(latest) = &options.latest_time {
        if !latest.trim().is_empty() {
            form_data.push(("latest_time", latest.clone()));
        } else {
            debug!("Skipping empty latest_time parameter");
        }
    }
    if let Some(max_count) = options.max_count {
        form_data.push(("max_count", max_count.to_string()));
    }
    if let Some(mode) = options.search_mode {
        form_data.push(("search_mode", mode.to_string()));
    }
    if let Some(window) = options.realtime_window {
        form_data.push(("realtime_window", window.to_string()));
    }

    // Security: Log form data keys with redacted search query
    // The 'search' field contains the query which is already logged above (redacted)
    for (key, value) in &form_data {
        if *key == "search" {
            debug!("  {}: {}", key, redact_query(value));
        } else {
            debug!("  {}: {}", key, value);
        }
    }

    let builder = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .form(&form_data);
    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/search/jobs",
        "POST",
        metrics,
        circuit_breaker,
    )
    .await?;

    let resp: serde_json::Value = response.json().await?;

    // Splunk can return either:
    // - `{ "sid": "<sid>" }` (common on newer versions / certain output modes)
    // - `{ "entry": [ { "content": { "sid": "<sid>" } } ] }` (older/alternate shape)
    let sid = resp
        .get("sid")
        .and_then(|v| v.as_str())
        .or_else(|| {
            resp.get("entry")?
                .get(0)?
                .get("content")?
                .get("sid")?
                .as_str()
        })
        .ok_or_else(|| ClientError::InvalidResponse("Missing sid in response".to_string()))?;

    Ok(sid.to_string())
}

/// Get the status of a search job.
#[allow(clippy::too_many_arguments)]
pub async fn get_job_status(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    sid: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<SearchJobStatus> {
    debug!("Getting status for job: {}", sid);

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
        .map_err(|e| ClientError::InvalidResponse(format!("Failed to parse job status: {}", e)))
}

/// Wait for a search job to complete.
#[allow(clippy::too_many_arguments)]
pub async fn wait_for_job(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    sid: &str,
    poll_interval_ms: u64,
    max_wait_secs: u64,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<SearchJobStatus> {
    wait_for_job_with_progress(
        client,
        base_url,
        auth_token,
        sid,
        poll_interval_ms,
        max_wait_secs,
        max_retries,
        None,
        metrics,
        circuit_breaker,
    )
    .await
}

/// Wait for a search job to complete, reporting progress via callback.
///
/// The callback receives `done_progress` as a fraction (0.0–1.0).
/// This is intended for UI layers (CLI/TUI) that want to display progress.
#[allow(clippy::too_many_arguments)]
pub async fn wait_for_job_with_progress(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    sid: &str,
    poll_interval_ms: u64,
    max_wait_secs: u64,
    max_retries: usize,
    mut progress_cb: Option<&mut (dyn FnMut(f64) + Send)>,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<SearchJobStatus> {
    let start = std::time::Instant::now();
    let max_wait = std::time::Duration::from_secs(max_wait_secs);

    loop {
        let status = get_job_status(
            client,
            base_url,
            auth_token,
            sid,
            max_retries,
            metrics,
            circuit_breaker,
        )
        .await?;

        if let Some(cb) = progress_cb.as_deref_mut() {
            cb(status.done_progress);
        }

        if status.is_done {
            debug!("Job {} completed", sid);
            return Ok(status);
        }

        if start.elapsed() > max_wait {
            return Err(ClientError::OperationTimeout {
                operation: "wait_for_job",
                timeout: max_wait,
            });
        }

        tokio::time::sleep(std::time::Duration::from_millis(poll_interval_ms)).await;
    }
}

/// Get results from a search job.
#[allow(clippy::too_many_arguments)]
pub async fn get_results(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    sid: &str,
    count: Option<usize>,
    offset: Option<usize>,
    output_mode: OutputMode,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<SearchJobResults> {
    debug!("Getting results for job: {}", sid);

    let encoded_sid = encode_path_segment(sid);
    let url = format!("{}/services/search/jobs/{}/results", base_url, encoded_sid);

    let mut query_params: Vec<(String, String)> =
        vec![("output_mode".to_string(), output_mode.to_string())];

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
        "/services/search/jobs/{sid}/results",
        "GET",
        metrics,
        circuit_breaker,
    )
    .await?;

    let body = response.text().await?;
    if body.trim().is_empty() {
        return Ok(SearchJobResults {
            results: vec![],
            preview: false,
            offset,
            total: None,
        });
    }

    let json: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
        ClientError::InvalidResponse(format!("Failed to parse search results response: {}", e))
    })?;

    Ok(SearchJobResults {
        results: match output_mode {
            OutputMode::Json => {
                // Handle both array and object-wrapped responses
                if let Some(arr) = json.as_array() {
                    arr.clone()
                } else if let Some(arr) = json["results"].as_array() {
                    arr.clone()
                } else {
                    vec![]
                }
            }
            _ => json["results"].as_array().unwrap_or(&vec![]).clone(),
        },
        preview: json["preview"].as_bool().unwrap_or(false),
        offset,
        total: json["total"]
            .as_u64()
            .map(|n| n as usize)
            .or_else(|| json["total"].as_str().and_then(|s| s.parse::<usize>().ok())),
    })
}
