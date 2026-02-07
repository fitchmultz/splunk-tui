//! Lookup table management endpoints.

use reqwest::Client;
use reqwest::Url;

use crate::endpoints::send_request_with_retry;
use crate::error::{ClientError, Result};
use crate::metrics::MetricsCollector;
use crate::models::{LookupTable, LookupTableListResponse, UploadLookupParams};

/// List all lookup table files.
///
/// This endpoint returns CSV-based lookup files stored in Splunk.
/// KV store lookups are managed via a different endpoint.
pub async fn list_lookup_tables(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    count: Option<usize>,
    offset: Option<usize>,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
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
    )
    .await?;

    let resp: LookupTableListResponse = response.json().await.map_err(|e| {
        ClientError::InvalidResponse(format!("Failed to parse lookup tables response: {}", e))
    })?;

    Ok(resp.entry.into_iter().map(|e| e.content).collect())
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
) -> Result<String> {
    let ns_owner = owner.unwrap_or("-");
    let ns_app = app.unwrap_or("search");
    let url = format!(
        "{}/servicesNS/{}/{}/data/lookup-table-files/{}",
        base_url, ns_owner, ns_app, name
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
pub async fn upload_lookup_table(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    params: &UploadLookupParams,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<LookupTable> {
    let ns_owner = params.owner.as_deref().unwrap_or("-");
    let ns_app = params.app.as_deref().unwrap_or("search");
    let url = format!(
        "{}/servicesNS/{}/{}/data/lookup-table-files",
        base_url, ns_owner, ns_app
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
) -> Result<()> {
    let ns_owner = owner.unwrap_or("-");
    let ns_app = app.unwrap_or("search");

    let url = Url::parse(base_url)
        .map_err(|e| ClientError::InvalidUrl(format!("Invalid base URL: {}", e)))?
        .join(&format!(
            "/servicesNS/{}/{}/data/lookup-table-files/{}",
            ns_owner, ns_app, name
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
    )
    .await?;

    Ok(())
}
