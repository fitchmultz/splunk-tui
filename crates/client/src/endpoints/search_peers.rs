//! Search peer management endpoints.
//!
//! This module provides low-level HTTP endpoint functions for interacting
//! with the Splunk distributed search peers API.
//!
//! # What this module handles:
//! - HTTP GET requests to list distributed search peers
//! - Query parameter construction for pagination
//!
//! # What this module does NOT handle:
//! - Authentication retry logic (handled by [`crate::client`])
//! - High-level client operations (see [`crate::client::search_peers`])
//! - Response deserialization (delegated to models)

use reqwest::Client;

use crate::endpoints::send_request_with_retry;
use crate::error::Result;
use crate::metrics::MetricsCollector;
use crate::models::{SearchPeer, SearchPeerListResponse};
use crate::name_merge::attach_entry_name;

/// List all distributed search peers.
///
/// Retrieves a list of search peers configured on the search head.
/// Supports pagination via `count` and `offset` parameters.
///
/// # Arguments
///
/// * `client` - The HTTP client to use for the request
/// * `base_url` - The base URL of the Splunk server
/// * `auth_token` - The authentication token for the request
/// * `count` - Maximum number of results to return (default: 30)
/// * `offset` - Offset for pagination
/// * `max_retries` - Maximum number of retry attempts for failed requests
/// * `metrics` - Optional metrics collector for request tracking
///
/// # Returns
///
/// A `Result` containing a vector of `SearchPeer` structs on success.
///
/// # Errors
///
/// Returns a `ClientError` if the request fails or the response cannot be parsed.
pub async fn list_search_peers(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    count: Option<u64>,
    offset: Option<u64>,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<Vec<SearchPeer>> {
    let url = format!("{}/services/search/distributed/peers", base_url);

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
        "/services/search/distributed/peers",
        "GET",
        metrics,
    )
    .await?;

    let resp: SearchPeerListResponse = response.json().await?;

    Ok(resp
        .entry
        .into_iter()
        .map(|e| attach_entry_name(e.name, e.content))
        .collect())
}
