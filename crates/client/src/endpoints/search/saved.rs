//! Saved search operations.
//!
//! This module provides endpoints for managing saved searches.
//!
//! # What this module handles:
//! - Listing saved searches
//! - Creating saved searches
//! - Updating saved searches
//! - Deleting saved searches
//! - Getting a single saved search by name
//!
//! # What this module does NOT handle:
//! - Search job execution (see [`super::jobs`])
//! - SPL validation (see [`super::validate`])

use reqwest::Client;
use tracing::debug;

use crate::endpoints::send_request_with_retry;
use crate::error::{ClientError, Result};
use crate::metrics::MetricsCollector;
use crate::models::SavedSearchListResponse;
use crate::name_merge::attach_entry_name;

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

/// Parameters for updating a saved search.
///
/// Only provided fields are updated; omitted fields retain their current values.
#[derive(Debug, Clone, Default)]
pub struct SavedSearchUpdateParams<'a> {
    /// New search query (SPL)
    pub search: Option<&'a str>,
    /// New description
    pub description: Option<&'a str>,
    /// Enable/disable flag
    pub disabled: Option<bool>,
}

/// Update an existing saved search.
///
/// This endpoint uses POST to `/services/saved/searches/{name}` to update
/// an existing saved search. Only provided fields are updated; omitted
/// fields retain their current values.
///
/// # Arguments
/// * `client` - The reqwest client
/// * `base_url` - The Splunk base URL
/// * `auth_token` - Authentication token
/// * `name` - The name of the saved search to update
/// * `params` - Update parameters (search, description, disabled)
/// * `max_retries` - Maximum number of retries for transient failures
/// * `metrics` - Optional metrics collector
///
/// # Returns
/// Ok(()) on success, or `ClientError::NotFound` if the saved search doesn't exist.
pub async fn update_saved_search(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    name: &str,
    params: &SavedSearchUpdateParams<'_>,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<()> {
    debug!("Updating saved search: {}", name);

    let url = format!("{}/services/saved/searches/{}", base_url, name);

    let mut form_params: Vec<(&str, String)> = Vec::new();

    if let Some(s) = params.search {
        form_params.push(("search", s.to_string()));
    }
    if let Some(d) = params.description {
        form_params.push(("description", d.to_string()));
    }
    if let Some(disabled_flag) = params.disabled {
        form_params.push(("disabled", disabled_flag.to_string()));
    }

    let builder = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")])
        .form(&form_params);

    match send_request_with_retry(
        builder,
        max_retries,
        "/services/saved/searches/{name}",
        "POST",
        metrics,
    )
    .await
    {
        Ok(_) => Ok(()),
        Err(ClientError::ApiError { status: 404, .. }) => Err(ClientError::NotFound(format!(
            "Saved search '{}' not found",
            name
        ))),
        Err(e) => Err(e),
    }
}
