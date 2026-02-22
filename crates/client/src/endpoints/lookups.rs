//! Purpose: Lookup table management endpoints.
//! Responsibilities: List, download, upload, and delete CSV lookup-table files via Splunk REST APIs.
//! Non-scope: KVStore lookup collections and lookup-transformation logic.
//! Invariants/Assumptions: List parsing tolerates Splunk variants that place ownership metadata in `acl` instead of `content`.

use reqwest::Client;
use reqwest::Url;

use crate::client::circuit_breaker::CircuitBreaker;
use crate::endpoints::encode_path_segment;
use crate::endpoints::send_request_with_retry;
use crate::error::{ClientError, Result};
use crate::metrics::MetricsCollector;
use crate::models::{LookupTable, UploadLookupParams};

/// List all lookup table files.
///
/// This endpoint returns CSV-based lookup files stored in Splunk.
/// KV store lookups are managed via a different endpoint.
#[allow(clippy::too_many_arguments)]
pub async fn list_lookup_tables(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    count: Option<usize>,
    offset: Option<usize>,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<Vec<LookupTable>> {
    let url = format!("{}/services/data/lookup-table-files", base_url);

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
        "/services/data/lookup-table-files",
        "GET",
        metrics,
        circuit_breaker,
    )
    .await?;

    let payload: serde_json::Value = response.json().await.map_err(|e| {
        ClientError::InvalidResponse(format!("Failed to parse lookup tables response: {}", e))
    })?;

    let entries = payload
        .get("entry")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| {
            ClientError::InvalidResponse(
                "Missing entry array in lookup tables response".to_string(),
            )
        })?;

    let lookups = entries.iter().map(parse_lookup_entry).collect();
    Ok(lookups)
}

fn parse_lookup_entry(entry: &serde_json::Value) -> LookupTable {
    let name = entry
        .get("name")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();
    let content = entry.get("content").unwrap_or(&serde_json::Value::Null);
    let acl = entry.get("acl").unwrap_or(&serde_json::Value::Null);

    let owner = content
        .get("owner")
        .and_then(serde_json::Value::as_str)
        .or_else(|| acl.get("owner").and_then(serde_json::Value::as_str))
        .or_else(|| entry.get("author").and_then(serde_json::Value::as_str))
        .unwrap_or_default()
        .to_string();

    let app = content
        .get("app")
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            content
                .get("eai:appName")
                .and_then(serde_json::Value::as_str)
        })
        .or_else(|| acl.get("app").and_then(serde_json::Value::as_str))
        .unwrap_or_default()
        .to_string();

    let sharing = content
        .get("sharing")
        .and_then(serde_json::Value::as_str)
        .or_else(|| acl.get("sharing").and_then(serde_json::Value::as_str))
        .unwrap_or("user")
        .to_string();

    let filename = content
        .get("filename")
        .and_then(serde_json::Value::as_str)
        .unwrap_or(&name)
        .to_string();

    let size = content
        .get("size")
        .and_then(size_from_json)
        .unwrap_or_default();

    LookupTable {
        name,
        filename,
        owner,
        app,
        sharing,
        size,
    }
}

fn size_from_json(value: &serde_json::Value) -> Option<usize> {
    match value {
        serde_json::Value::Number(n) => n.as_u64().and_then(|u| usize::try_from(u).ok()),
        serde_json::Value::String(s) => s.parse::<usize>().ok(),
        _ => None,
    }
}

/// Download a lookup table file as raw CSV content.
///
/// # Arguments
/// * `client` - The HTTP client
/// * `base_url` - The Splunk base URL
/// * `auth_token` - The authentication token
/// * `name` - The lookup name
/// * `app` - Optional app namespace (defaults to "search")
/// * `owner` - Optional owner namespace (defaults to "-" for all users)
/// * `max_retries` - Maximum number of retries for failed requests
/// * `metrics` - Optional metrics collector
///
/// # Returns
/// The raw CSV content as a string
#[allow(clippy::too_many_arguments)]
pub async fn download_lookup_table(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    name: &str,
    app: Option<&str>,
    owner: Option<&str>,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<String> {
    let ns_owner = owner.unwrap_or("-");
    let ns_app = app.unwrap_or("search");
    let encoded_owner = encode_path_segment(ns_owner);
    let encoded_app = encode_path_segment(ns_app);
    let encoded_name = encode_path_segment(name);
    let url = format!(
        "{}/servicesNS/{}/{}/data/lookup-table-files/{}",
        base_url, encoded_owner, encoded_app, encoded_name
    );

    let query_params = vec![("output_mode".to_string(), "raw".to_string())];

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&query_params);

    let response = send_request_with_retry(
        builder,
        max_retries,
        "/servicesNS/-/search/data/lookup-table-files",
        "GET",
        metrics,
        circuit_breaker,
    )
    .await?;

    // Response is raw CSV text, not JSON
    let content = response.text().await.map_err(|e| {
        ClientError::InvalidResponse(format!("Failed to read lookup file content: {}", e))
    })?;

    Ok(content)
}

/// Upload or replace a lookup table file.
///
/// Note: This operation uses multipart/form-data which cannot be retried
/// due to the request body being consumed on the first attempt. The
/// send_request_with_retry function handles this gracefully by attempting
/// a single request when the body cannot be cloned.
///
/// # Arguments
/// * `client` - The HTTP client
/// * `base_url` - The Splunk base URL
/// * `auth_token` - The authentication token
/// * `params` - Upload parameters including name, filename, and content
/// * `max_retries` - Maximum number of retries (ignored for multipart uploads)
/// * `metrics` - Optional metrics collector
///
/// # Returns
/// The created/updated lookup table metadata
#[allow(clippy::too_many_arguments)]
pub async fn upload_lookup_table(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    params: &UploadLookupParams,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<LookupTable> {
    let ns_owner = params.owner.as_deref().unwrap_or("-");
    let ns_app = params.app.as_deref().unwrap_or("search");
    let encoded_owner = encode_path_segment(ns_owner);
    let encoded_app = encode_path_segment(ns_app);
    let url = format!(
        "{}/servicesNS/{}/{}/data/lookup-table-files",
        base_url, encoded_owner, encoded_app
    );

    // Build multipart form
    let mut form = reqwest::multipart::Form::new()
        .text("name", params.name.clone())
        .text("filename", params.filename.clone())
        .part(
            "file",
            reqwest::multipart::Part::bytes(params.content.clone())
                .file_name(params.filename.clone())
                .mime_str("text/csv")
                .map_err(|e| ClientError::InvalidRequest(format!("Invalid MIME type: {}", e)))?,
        );

    // Add optional sharing parameter
    if let Some(sharing) = &params.sharing {
        form = form.text("sharing", sharing.clone());
    }

    let builder = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .multipart(form);

    // Use send_request_with_retry - it will handle the non-cloneable body gracefully
    // by attempting a single request when try_clone() returns None
    let response = send_request_with_retry(
        builder,
        max_retries,
        "/servicesNS/-/search/data/lookup-table-files",
        "POST",
        metrics,
        circuit_breaker,
    )
    .await?;

    // Parse response to get the created/updated lookup metadata
    let resp: serde_json::Value = response.json().await.map_err(|e| {
        ClientError::InvalidResponse(format!("Failed to parse upload response: {}", e))
    })?;

    let entry = resp.get("entry").and_then(|e| e.get(0)).ok_or_else(|| {
        ClientError::InvalidResponse("Missing entry in upload response".to_string())
    })?;

    let content = entry.get("content").ok_or_else(|| {
        ClientError::InvalidResponse("Missing entry content in upload response".to_string())
    })?;

    let lookup: LookupTable = serde_json::from_value(content.clone())
        .map_err(|e| ClientError::InvalidResponse(format!("Failed to parse lookup: {}", e)))?;

    Ok(lookup)
}

/// Delete a lookup table file.
///
/// # Arguments
/// * `client` - The HTTP client
/// * `base_url` - The Splunk base URL
/// * `auth_token` - The authentication token
/// * `name` - The lookup name to delete
/// * `app` - Optional app namespace (defaults to "search")
/// * `owner` - Optional owner namespace (defaults to "-" for all users)
/// * `max_retries` - Maximum number of retries for failed requests
/// * `metrics` - Optional metrics collector
#[allow(clippy::too_many_arguments)]
pub async fn delete_lookup_table(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    name: &str,
    app: Option<&str>,
    owner: Option<&str>,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<()> {
    let ns_owner = owner.unwrap_or("-");
    let ns_app = app.unwrap_or("search");
    let encoded_owner = encode_path_segment(ns_owner);
    let encoded_app = encode_path_segment(ns_app);
    let encoded_name = encode_path_segment(name);

    let url = Url::parse(base_url)
        .map_err(|e| ClientError::InvalidUrl(format!("Invalid base URL: {}", e)))?
        .join(&format!(
            "/servicesNS/{}/{}/data/lookup-table-files/{}",
            encoded_owner, encoded_app, encoded_name
        ))
        .map_err(|e| ClientError::InvalidUrl(format!("Invalid lookup name: {}", e)))?;

    let builder = client
        .delete(url)
        .header("Authorization", format!("Bearer {}", auth_token));

    let _response = send_request_with_retry(
        builder,
        max_retries,
        "/servicesNS/-/search/data/lookup-table-files",
        "DELETE",
        metrics,
        circuit_breaker,
    )
    .await?;

    Ok(())
}
