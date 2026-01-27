//! Search job endpoints.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::endpoints::send_request_with_retry;
use crate::error::{ClientError, Result};
use crate::models::{SavedSearchListResponse, SearchJobResults, SearchJobStatus};
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
    if let Some(earliest) = &options.earliest_time {
        form_data.push(("earliest_time", earliest.clone()));
    }
    if let Some(latest) = &options.latest_time {
        form_data.push(("latest_time", latest.clone()));
    }
    if let Some(max_count) = options.max_count {
        form_data.push(("max_count", max_count.to_string()));
    }
    if let Some(mode) = options.search_mode {
        form_data.push(("search_mode", mode.to_string()));
    }

    let builder = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .form(&form_data);
    let response = send_request_with_retry(builder, max_retries).await?;

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
) -> Result<SearchJobStatus> {
    debug!("Getting status for job: {}", sid);

    let url = format!("{}/services/search/jobs/{}", base_url, sid);

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")]);
    let response = send_request_with_retry(builder, max_retries).await?;

    let resp: serde_json::Value = response.json().await?;

    serde_json::from_value(resp["entry"][0]["content"].clone())
        .map_err(|e| ClientError::InvalidResponse(format!("Failed to parse job status: {}", e)))
}

/// Wait for a search job to complete.
pub async fn wait_for_job(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    sid: &str,
    poll_interval_ms: u64,
    max_wait_secs: u64,
    max_retries: usize,
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
) -> Result<SearchJobStatus> {
    let start = std::time::Instant::now();
    let max_wait = std::time::Duration::from_secs(max_wait_secs);

    loop {
        let status = get_job_status(client, base_url, auth_token, sid, max_retries).await?;

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
    let response = send_request_with_retry(builder, max_retries).await?;

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
    max_retries: usize,
) -> Result<Vec<crate::models::SavedSearch>> {
    debug!("Listing saved searches");

    let url = format!("{}/services/saved/searches", base_url);

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json"), ("count", "0")]);
    let response = send_request_with_retry(builder, max_retries).await?;

    let resp: SavedSearchListResponse = response.json().await.map_err(|e| {
        ClientError::InvalidResponse(format!("Failed to parse saved searches response: {}", e))
    })?;

    Ok(resp
        .entry
        .into_iter()
        .map(|e| attach_entry_name(e.name, e.content))
        .collect())
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
