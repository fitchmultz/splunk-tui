//! Index management endpoints.

use reqwest::{Client, Url};

use crate::client::circuit_breaker::CircuitBreaker;
use crate::endpoints::encode_path_segment;
use crate::endpoints::form_params;
use crate::endpoints::send_request_with_retry;
use crate::error::{ClientError, Result};
use crate::metrics::MetricsCollector;
use crate::models::{CreateIndexParams, Index, IndexListResponse, ModifyIndexParams};
use crate::name_merge::attach_entry_name;

/// List all indexes.
#[allow(clippy::too_many_arguments)]
pub async fn list_indexes(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    count: Option<usize>,
    offset: Option<usize>,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
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
    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/data/indexes",
        "GET",
        metrics,
        circuit_breaker,
    )
    .await?;

    let resp: IndexListResponse = response.json().await?;

    Ok(resp
        .entry
        .into_iter()
        .map(|e| attach_entry_name(e.name, e.content))
        .collect())
}

/// Get a specific index by name.
#[allow(clippy::too_many_arguments)]
pub async fn get_index(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    index_name: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<Index> {
    let encoded_index_name = encode_path_segment(index_name);
    let url = Url::parse(base_url)
        .map_err(|e| ClientError::InvalidUrl(format!("Invalid base URL: {}", e)))?
        .join(&format!("/services/data/indexes/{}", encoded_index_name))
        .map_err(|e| ClientError::InvalidUrl(format!("Invalid index name: {}", e)))?;

    let builder = client
        .get(url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")]);
    let response = send_request_with_retry(
        builder,
        max_retries,
        &format!("/services/data/indexes/{}", encoded_index_name),
        "GET",
        metrics,
        circuit_breaker,
    )
    .await?;

    let resp: IndexListResponse = response.json().await?;

    let entry = resp
        .entry
        .into_iter()
        .next()
        .ok_or_else(|| ClientError::NotFound(format!("Index '{}' not found", index_name)))?;

    Ok(attach_entry_name(entry.name, entry.content))
}

/// Create a new index.
#[allow(clippy::too_many_arguments)]
pub async fn create_index(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    params: &CreateIndexParams,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<Index> {
    let url = format!("{}/services/data/indexes", base_url);

    let mut form_params: Vec<(String, String)> =
        vec![("output_mode".to_string(), "json".to_string())];

    form_params! { form_params =>
        "name" => required_clone params.name,
        "maxTotalDataSizeMB" => params.max_data_size_mb,
        "maxHotBuckets" => params.max_hot_buckets,
        "maxWarmDBCount" => params.max_warm_db_count,
        "frozenTimePeriodInSecs" => params.frozen_time_period_in_secs,
        "homePath" => ref params.home_path,
        "coldDBPath" => ref params.cold_db_path,
        "thawedPath" => ref params.thawed_path,
        "coldToFrozenDir" => ref params.cold_to_frozen_dir,
    }

    let builder = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .form(&form_params);

    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/data/indexes",
        "POST",
        metrics,
        circuit_breaker,
    )
    .await?;

    let resp: serde_json::Value = response.json().await?;

    // Extract entry from response
    let entry = resp.get("entry").and_then(|e| e.get(0)).ok_or_else(|| {
        ClientError::InvalidResponse("Missing entry in create index response".to_string())
    })?;

    let entry_name = entry
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or(&params.name)
        .to_string();

    let content = entry.get("content").ok_or_else(|| {
        ClientError::InvalidResponse("Missing entry content in create index response".to_string())
    })?;

    let index: Index = serde_json::from_value(content.clone())
        .map_err(|e| ClientError::InvalidResponse(format!("Failed to parse index: {}", e)))?;

    Ok(attach_entry_name(entry_name, index))
}

/// Modify an existing index.
#[allow(clippy::too_many_arguments)]
pub async fn modify_index(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    index_name: &str,
    params: &ModifyIndexParams,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<Index> {
    let encoded_index_name = encode_path_segment(index_name);
    let url = Url::parse(base_url)
        .map_err(|e| ClientError::InvalidUrl(format!("Invalid base URL: {}", e)))?
        .join(&format!("/services/data/indexes/{}", encoded_index_name))
        .map_err(|e| ClientError::InvalidUrl(format!("Invalid index name: {}", e)))?;

    let mut form_params: Vec<(String, String)> =
        vec![("output_mode".to_string(), "json".to_string())];

    form_params! { form_params =>
        "maxTotalDataSizeMB" => params.max_data_size_mb,
        "maxHotBuckets" => params.max_hot_buckets,
        "maxWarmDBCount" => params.max_warm_db_count,
        "frozenTimePeriodInSecs" => params.frozen_time_period_in_secs,
        "homePath" => ref params.home_path,
        "coldDBPath" => ref params.cold_db_path,
        "thawedPath" => ref params.thawed_path,
        "coldToFrozenDir" => ref params.cold_to_frozen_dir,
    }

    let builder = client
        .post(url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .form(&form_params);

    let response = send_request_with_retry(
        builder,
        max_retries,
        &format!("/services/data/indexes/{}", encoded_index_name),
        "POST",
        metrics,
        circuit_breaker,
    )
    .await?;

    let resp: serde_json::Value = response.json().await?;

    // Extract entry from response
    let entry = resp.get("entry").and_then(|e| e.get(0)).ok_or_else(|| {
        ClientError::InvalidResponse(format!(
            "Missing entry in modify index response for '{}'",
            index_name
        ))
    })?;

    let entry_name = entry
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or(index_name)
        .to_string();

    let content = entry.get("content").ok_or_else(|| {
        ClientError::InvalidResponse(format!(
            "Missing entry content in modify index response for '{}'",
            index_name
        ))
    })?;

    let index: Index = serde_json::from_value(content.clone())
        .map_err(|e| ClientError::InvalidResponse(format!("Failed to parse index: {}", e)))?;

    Ok(attach_entry_name(entry_name, index))
}

/// Delete an index by name.
#[allow(clippy::too_many_arguments)]
pub async fn delete_index(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    index_name: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<()> {
    let encoded_index_name = encode_path_segment(index_name);
    let url = Url::parse(base_url)
        .map_err(|e| ClientError::InvalidUrl(format!("Invalid base URL: {}", e)))?
        .join(&format!("/services/data/indexes/{}", encoded_index_name))
        .map_err(|e| ClientError::InvalidUrl(format!("Invalid index name: {}", e)))?;

    let builder = client
        .delete(url)
        .header("Authorization", format!("Bearer {}", auth_token));

    let _response = send_request_with_retry(
        builder,
        max_retries,
        &format!("/services/data/indexes/{}", encoded_index_name),
        "DELETE",
        metrics,
        circuit_breaker,
    )
    .await?;

    Ok(())
}
