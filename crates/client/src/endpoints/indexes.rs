//! Index management endpoints.

use reqwest::Client;

use crate::endpoints::send_request_with_retry;
use crate::error::{ClientError, Result};
use crate::models::{Index, IndexListResponse};

/// List all indexes.
pub async fn list_indexes(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    count: Option<u64>,
    offset: Option<u64>,
    max_retries: usize,
) -> Result<Vec<Index>> {
    let url = format!("{}/services/data/indexes", base_url);

    let mut query_params: Vec<(String, String)> = vec![
        ("output_mode".to_string(), "json".to_string()),
        ("count".to_string(), count.unwrap_or(30).to_string()),
    ];

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

    let resp: IndexListResponse = response.json().await?;

    Ok(resp.entry.into_iter().map(|e| e.content).collect())
}
