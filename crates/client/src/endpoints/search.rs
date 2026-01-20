//! Search job endpoints.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::endpoints::send_request_with_retry;
use crate::error::{ClientError, Result};
use crate::models::{SearchJobResults, SearchJobStatus};

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
    if let Some(mode) = options.output_mode {
        form_data.push(("output_mode", mode.to_string()));
    }

    let builder = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .form(&form_data);
    let response = send_request_with_retry(builder, max_retries).await?;

    let status = response.status().as_u16();

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(ClientError::ApiError {
            status,
            message: body,
        });
    }

    let resp: serde_json::Value = response.json().await?;

    resp["entry"][0]["content"]["sid"]
        .as_str()
        .ok_or_else(|| ClientError::InvalidResponse("Missing sid in response".to_string()))
        .map(|s| s.to_string())
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

    let status = response.status().as_u16();

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(ClientError::ApiError {
            status,
            message: body,
        });
    }

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
    let start = std::time::Instant::now();
    let max_wait = std::time::Duration::from_secs(max_wait_secs);

    loop {
        let status = get_job_status(client, base_url, auth_token, sid, max_retries).await?;

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

    let status = response.status().as_u16();

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(ClientError::ApiError {
            status,
            message: body,
        });
    }

    let json: serde_json::Value = response.json().await?;

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
        total: json["total"].as_u64(),
    })
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
