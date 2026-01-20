//! Job management endpoints.

use reqwest::Client;

use crate::error::{ClientError, Result};
use crate::models::{SearchJobListResponse, SearchJobStatus};

/// Get a specific search job.
pub async fn get_job(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    sid: &str,
) -> Result<SearchJobStatus> {
    let url = format!("{}/services/search/jobs/{}", base_url, sid);

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")])
        .send()
        .await?;

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
        .map_err(|e| ClientError::InvalidResponse(format!("Failed to parse job: {}", e)))
}

/// List all search jobs.
pub async fn list_jobs(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    count: Option<u64>,
    offset: Option<u64>,
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

    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&query_params)
        .send()
        .await?;

    let status = response.status().as_u16();

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(ClientError::ApiError {
            status,
            message: body,
        });
    }

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
            disk_usage: 0,
            priority: None,
            label: None,
        })
        .collect())
}

/// Cancel a search job.
pub async fn cancel_job(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    sid: &str,
) -> Result<()> {
    let url = format!("{}/services/search/jobs/{}/control", base_url, sid);

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .form(&[("action", "cancel")])
        .send()
        .await?;

    let status = response.status().as_u16();

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(ClientError::ApiError {
            status,
            message: body,
        });
    }

    Ok(())
}

/// Delete a search job.
pub async fn delete_job(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    sid: &str,
) -> Result<()> {
    let url = format!("{}/services/search/jobs/{}", base_url, sid);

    let response = client
        .delete(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .send()
        .await?;

    let status = response.status().as_u16();

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(ClientError::ApiError {
            status,
            message: body,
        });
    }

    Ok(())
}
