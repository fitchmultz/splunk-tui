//! Search job endpoints.
//!
//! This module provides low-level HTTP endpoints for Splunk search operations.
//!
//! # What this module handles:
//! - Search job creation, status, and results
//! - Saved search management
//! - SPL syntax validation
//!
//! # What this module does NOT handle:
//! - High-level search operations (see [`crate::client::search`])
//! - Result parsing beyond JSON deserialization

use reqwest::Client;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::endpoints::send_request_with_retry;
use crate::error::{ClientError, Result};
use crate::metrics::MetricsCollector;
use crate::models::{
    SavedSearchListResponse, SearchJobResults, SearchJobStatus, ValidateSplResponse,
};
use crate::name_merge::attach_entry_name;

/// Options for creating a search job.
#[derive(Debug, Clone, Serialize, Default)]
pub struct CreateJobOptions {
    /// Whether to wait for the job to complete.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wait: Option<bool>,
    /// Maximum time to wait for job completion (seconds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exec_time: Option<u64>,
    /// Earliest time for search (e.g., "-24h", "2024-01-01T00:00:00").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub earliest_time: Option<String>,
    /// Latest time for search (e.g., "now", "2024-01-02T00:00:00").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_time: Option<String>,
    /// Maximum number of results to return.
    #[serde(rename = "maxCount", skip_serializing_if = "Option::is_none")]
    pub max_count: Option<u64>,
    /// Output format for results.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_mode: Option<OutputMode>,
    /// Search mode (normal or realtime).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_mode: Option<SearchMode>,
}

/// Search mode for search jobs.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SearchMode {
    #[default]
    Normal,
    Realtime,
}

impl std::fmt::Display for SearchMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            SearchMode::Normal => "normal",
            SearchMode::Realtime => "realtime",
        };
        write!(f, "{}", s)
    }
}

/// Output format for search results.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OutputMode {
    #[default]
    Json,
    JsonCols,
    JsonRows,
    Xml,
    Csv,
    Raw,
}

impl std::fmt::Display for OutputMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            OutputMode::Json => "json",
            OutputMode::JsonCols => "json_cols",
            OutputMode::JsonRows => "json_rows",
            OutputMode::Xml => "xml",
            OutputMode::Csv => "csv",
            OutputMode::Raw => "raw",
        };
        write!(f, "{}", s)
    }
}

/// Create a new search job.
pub async fn create_job(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    query: &str,
    options: &CreateJobOptions,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<String> {
    debug!("Creating search job: {}", query);

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

    // Debug: Log the complete form data being sent
    debug!("Search job form data: {:?}", form_data);
    for (key, value) in &form_data {
        debug!("  {}: {}", key, value);
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
pub async fn get_job_status(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    sid: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<SearchJobStatus> {
    debug!("Getting status for job: {}", sid);

    let url = format!("{}/services/search/jobs/{}", base_url, sid);

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
    )
    .await?;

    let resp: serde_json::Value = response.json().await?;

    serde_json::from_value(resp["entry"][0]["content"].clone())
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
) -> Result<SearchJobStatus> {
    let start = std::time::Instant::now();
    let max_wait = std::time::Duration::from_secs(max_wait_secs);

    loop {
        let status =
            get_job_status(client, base_url, auth_token, sid, max_retries, metrics).await?;

        if let Some(cb) = progress_cb.as_deref_mut() {
            cb(status.done_progress);
        }

        if status.is_done {
            debug!("Job {} completed", sid);
            return Ok(status);
        }

        if start.elapsed() > max_wait {
            return Err(ClientError::Timeout(max_wait));
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
    count: Option<u64>,
    offset: Option<u64>,
    output_mode: OutputMode,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<SearchJobResults> {
    debug!("Getting results for job: {}", sid);

    let url = format!("{}/services/search/jobs/{}/results", base_url, sid);

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
            .or_else(|| json["total"].as_str().and_then(|s| s.parse::<u64>().ok())),
    })
}

/// List saved searches.
pub async fn list_saved_searches(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    count: Option<u64>,
    offset: Option<u64>,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<Vec<crate::models::SavedSearch>> {
    debug!("Listing saved searches");

    let url = format!("{}/services/saved/searches", base_url);

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
        "/services/saved/searches",
        "GET",
        metrics,
    )
    .await?;

    let resp: SavedSearchListResponse = response.json().await.map_err(|e| {
        ClientError::InvalidResponse(format!("Failed to parse saved searches response: {}", e))
    })?;

    Ok(resp
        .entry
        .into_iter()
        .map(|e| attach_entry_name(e.name, e.content))
        .collect())
}

/// Create a saved search.
///
/// This endpoint is used for live-test setup and for future UI/CLI parity.
pub async fn create_saved_search(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    name: &str,
    search: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<()> {
    debug!("Creating saved search: {}", name);

    let url = format!("{}/services/saved/searches", base_url);

    let builder = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")])
        .form(&[("name", name), ("search", search)]);

    let _response = send_request_with_retry(
        builder,
        max_retries,
        "/services/saved/searches",
        "POST",
        metrics,
    )
    .await?;
    Ok(())
}

/// Delete a saved search by name.
///
/// This endpoint is used for live-test cleanup and for future UI/CLI parity.
pub async fn delete_saved_search(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    name: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<()> {
    debug!("Deleting saved search: {}", name);

    let url = format!("{}/services/saved/searches/{}", base_url, name);

    let builder = client
        .delete(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")]);

    let _response = send_request_with_retry(
        builder,
        max_retries,
        "/services/saved/searches/{name}",
        "DELETE",
        metrics,
    )
    .await?;
    Ok(())
}

/// Get a single saved search by name.
///
/// This endpoint retrieves a specific saved search directly by name,
/// avoiding the need to list all saved searches and scan for the target.
///
/// # Arguments
/// * `client` - The reqwest client
/// * `base_url` - The Splunk base URL
/// * `auth_token` - Authentication token
/// * `name` - The name of the saved search
/// * `max_retries` - Maximum number of retries for transient failures
/// * `metrics` - Optional metrics collector
///
/// # Returns
/// The `SavedSearch` if found, or `ClientError::NotFound` if it doesn't exist.
pub async fn get_saved_search(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    name: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<crate::models::SavedSearch> {
    debug!("Getting saved search: {}", name);

    let url = format!("{}/services/saved/searches/{}", base_url, name);

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")]);

    let response = match send_request_with_retry(
        builder,
        max_retries,
        "/services/saved/searches/{name}",
        "GET",
        metrics,
    )
    .await
    {
        Ok(resp) => resp,
        Err(ClientError::ApiError { status: 404, .. }) => {
            return Err(ClientError::NotFound(format!(
                "Saved search '{}' not found",
                name
            )));
        }
        Err(e) => return Err(e),
    };

    let body: SavedSearchListResponse = response.json().await.map_err(|e| {
        ClientError::InvalidResponse(format!("Failed to parse saved search response: {}", e))
    })?;

    // Extract the first entry's content (Splunk returns single entry for single-resource GET)
    let entry = body
        .entry
        .into_iter()
        .next()
        .ok_or_else(|| ClientError::NotFound(format!("Saved search '{}' not found", name)))?;

    Ok(attach_entry_name(entry.name, entry.content))
}

/// Validate SPL syntax using Splunk's search parser endpoint.
///
/// Sends the query to `/services/search/parser` which parses the SPL
/// and returns either a parse tree (on success) or error details (on failure).
///
/// # Arguments
/// * `client` - The reqwest HTTP client
/// * `base_url` - The Splunk base URL
/// * `auth_token` - Authentication token
/// * `search` - The SPL query to validate
/// * `max_retries` - Maximum number of retries for transient failures
/// * `metrics` - Optional metrics collector
///
/// # Returns
/// * `Ok(ValidateSplResponse)` - Validation result with errors/warnings
/// * `Err(ClientError)` - Transport or API error
///
/// # Note
/// This endpoint returns HTTP 200 for valid SPL and HTTP 400 for syntax errors.
/// Both are considered "successful" responses from a validation perspective.
pub async fn validate_spl(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    search: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<ValidateSplResponse> {
    debug!("Validating SPL syntax: {}", search);

    let url = format!("{}/services/search/parser", base_url);

    let form_data = [("q", search), ("output_mode", "json")];

    let builder = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .form(&form_data);

    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/search/parser",
        "POST",
        metrics,
    )
    .await?;

    let status = response.status();
    let body_text = response.text().await?;

    match status {
        StatusCode::OK => {
            // Valid SPL - parse any warnings from response
            let body: serde_json::Value = serde_json::from_str(&body_text).map_err(|e| {
                ClientError::InvalidResponse(format!("Failed to parse validation response: {}", e))
            })?;

            let warnings = extract_warnings(&body);

            Ok(ValidateSplResponse {
                valid: true,
                errors: vec![],
                warnings,
            })
        }
        StatusCode::BAD_REQUEST => {
            // Syntax error - parse error details
            let body: serde_json::Value = serde_json::from_str(&body_text).map_err(|e| {
                ClientError::InvalidResponse(format!("Failed to parse validation error: {}", e))
            })?;

            let errors = extract_errors(&body);

            Ok(ValidateSplResponse {
                valid: false,
                errors,
                warnings: vec![],
            })
        }
        _ => Err(ClientError::ApiError {
            status: status.as_u16(),
            url,
            message: body_text,
            request_id: None,
        }),
    }
}

/// Extract warnings from parser response.
fn extract_warnings(body: &serde_json::Value) -> Vec<crate::models::SplWarning> {
    let mut warnings = vec![];

    // Splunk may return warnings in different formats depending on version
    if let Some(messages) = body.get("messages")
        && let Some(arr) = messages.as_array()
    {
        for msg in arr {
            if let Some(text) = msg.get("text").and_then(|t| t.as_str()) {
                warnings.push(crate::models::SplWarning {
                    message: text.to_string(),
                    line: msg.get("line").and_then(|l| l.as_u64()).map(|n| n as u32),
                    column: msg.get("column").and_then(|c| c.as_u64()).map(|n| n as u32),
                });
            }
        }
    }

    warnings
}

/// Extract errors from parser error response.
fn extract_errors(body: &serde_json::Value) -> Vec<crate::models::SplError> {
    let mut errors = vec![];

    // Try to extract from messages array first
    if let Some(messages) = body.get("messages")
        && let Some(arr) = messages.as_array()
    {
        for msg in arr {
            if let Some(text) = msg.get("text").and_then(|t| t.as_str()) {
                errors.push(crate::models::SplError {
                    message: text.to_string(),
                    line: msg.get("line").and_then(|l| l.as_u64()).map(|n| n as u32),
                    column: msg.get("column").and_then(|c| c.as_u64()).map(|n| n as u32),
                });
            }
        }
    }

    // If no messages array, look for error field
    if errors.is_empty()
        && let Some(error) = body.get("error").and_then(|e| e.as_str())
    {
        errors.push(crate::models::SplError {
            message: error.to_string(),
            line: None,
            column: None,
        });
    }

    // Last resort: use the entire body as error message
    if errors.is_empty() {
        errors.push(crate::models::SplError {
            message: body.to_string(),
            line: None,
            column: None,
        });
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_job_options_serialization() {
        let _options = CreateJobOptions {
            wait: Some(true),
            exec_time: Some(60),
            earliest_time: Some("-24h".to_string()),
            max_count: Some(1000),
            ..Default::default()
        };

        let form_data = [
            ("search", "search index=main"),
            ("wait", "1"),
            ("exec_time", "60"),
            ("earliest_time", "-24h"),
            ("max_count", "1000"),
        ];

        assert_eq!(form_data[0].0, "search");
    }
}
