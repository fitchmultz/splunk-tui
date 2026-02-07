//! Index management endpoints.

use reqwest::{Client, Url};

use crate::endpoints::send_request_with_retry;
use crate::error::{ClientError, Result};
use crate::metrics::MetricsCollector;
use crate::models::{CreateIndexParams, Index, IndexListResponse, ModifyIndexParams};
use crate::name_merge::attach_entry_name;

/// List all indexes.
pub async fn list_indexes(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    count: Option<usize>,
    offset: Option<usize>,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
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
    )
    .await?;

    let resp: IndexListResponse = response.json().await?;

    Ok(resp
        .entry
        .into_iter()
        .map(|e| attach_entry_name(e.name, e.content))
        .collect())
}

/// Create a new index.
pub async fn create_index(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    params: &CreateIndexParams,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<Index> {
    let url = format!("{}/services/data/indexes", base_url);

    let mut form_params: Vec<(String, String)> = vec![
        ("name".to_string(), params.name.clone()),
        ("output_mode".to_string(), "json".to_string()),
    ];

    if let Some(max_data_size) = params.max_data_size_mb {
        form_params.push(("maxTotalDataSizeMB".to_string(), max_data_size.to_string()));
    }
    if let Some(max_hot) = params.max_hot_buckets {
        form_params.push(("maxHotBuckets".to_string(), max_hot.to_string()));
    }
    if let Some(max_warm) = params.max_warm_db_count {
        form_params.push(("maxWarmDBCount".to_string(), max_warm.to_string()));
    }
    if let Some(frozen_time) = params.frozen_time_period_in_secs {
        form_params.push((
            "frozenTimePeriodInSecs".to_string(),
            frozen_time.to_string(),
        ));
    }
    if let Some(ref home_path) = params.home_path {
        form_params.push(("homePath".to_string(), home_path.clone()));
    }
    if let Some(ref cold_db_path) = params.cold_db_path {
        form_params.push(("coldDBPath".to_string(), cold_db_path.clone()));
    }
    if let Some(ref thawed_path) = params.thawed_path {
        form_params.push(("thawedPath".to_string(), thawed_path.clone()));
    }
    if let Some(ref cold_to_frozen) = params.cold_to_frozen_dir {
        form_params.push(("coldToFrozenDir".to_string(), cold_to_frozen.clone()));
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
pub async fn modify_index(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    index_name: &str,
    params: &ModifyIndexParams,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<Index> {
    let url = Url::parse(base_url)
        .map_err(|e| ClientError::InvalidUrl(format!("Invalid base URL: {}", e)))?
        .join(&format!("/services/data/indexes/{}", index_name))
        .map_err(|e| ClientError::InvalidUrl(format!("Invalid index name: {}", e)))?;

    let mut form_params: Vec<(String, String)> =
        vec![("output_mode".to_string(), "json".to_string())];

    if let Some(max_data_size) = params.max_data_size_mb {
        form_params.push(("maxTotalDataSizeMB".to_string(), max_data_size.to_string()));
    }
    if let Some(max_hot) = params.max_hot_buckets {
        form_params.push(("maxHotBuckets".to_string(), max_hot.to_string()));
    }
    if let Some(max_warm) = params.max_warm_db_count {
        form_params.push(("maxWarmDBCount".to_string(), max_warm.to_string()));
    }
    if let Some(frozen_time) = params.frozen_time_period_in_secs {
        form_params.push((
            "frozenTimePeriodInSecs".to_string(),
            frozen_time.to_string(),
        ));
    }
    if let Some(ref home_path) = params.home_path {
        form_params.push(("homePath".to_string(), home_path.clone()));
    }
    if let Some(ref cold_db_path) = params.cold_db_path {
        form_params.push(("coldDBPath".to_string(), cold_db_path.clone()));
    }
    if let Some(ref thawed_path) = params.thawed_path {
        form_params.push(("thawedPath".to_string(), thawed_path.clone()));
    }
    if let Some(ref cold_to_frozen) = params.cold_to_frozen_dir {
        form_params.push(("coldToFrozenDir".to_string(), cold_to_frozen.clone()));
    }

    let builder = client
        .post(url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .form(&form_params);

    let response = send_request_with_retry(
        builder,
        max_retries,
        &format!("/services/data/indexes/{}", index_name),
        "POST",
        metrics,
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
pub async fn delete_index(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    index_name: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<()> {
    let url = Url::parse(base_url)
        .map_err(|e| ClientError::InvalidUrl(format!("Invalid base URL: {}", e)))?
        .join(&format!("/services/data/indexes/{}", index_name))
        .map_err(|e| ClientError::InvalidUrl(format!("Invalid index name: {}", e)))?;

    let builder = client
        .delete(url)
        .header("Authorization", format!("Bearer {}", auth_token));

    let _response = send_request_with_retry(
        builder,
        max_retries,
        &format!("/services/data/indexes/{}", index_name),
        "DELETE",
        metrics,
    )
    .await?;

    Ok(())
}
