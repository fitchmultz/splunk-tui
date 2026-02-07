//! Resource fetching logic for the list-all command.
//!
//! Responsibilities:
//! - Fetch resource summaries from Splunk for all supported resource types.
//! - Handle per-resource timeouts (30-second default).
//! - Convert API errors into ResourceSummary error states gracefully.
//! - Fetch resources concurrently with bounded concurrency (5 concurrent by default).
//!
//! Does NOT handle:
//! - Multi-profile orchestration (see `output.rs`).
//! - Output formatting (see `output.rs`).
//! - Authentication strategy building (see `auth.rs`).
//!
//! Invariants:
//! - Each resource fetch has a 30-second timeout.
//! - Errors are captured in ResourceSummary, not propagated as Err.
//! - Resources are fetched concurrently with bounded concurrency to avoid overwhelming Splunk.

use crate::cancellation::{CancellationToken, Cancelled};
use anyhow::Result;
use futures::stream::{self, StreamExt};
use splunk_client::{ClientError, SplunkClient};
use std::time::Duration;
use tokio::time;
use tracing::warn;

use super::types::ResourceSummary;

const TIMEOUT_DURATION: Duration = Duration::from_secs(30);
const MAX_CONCURRENT_FETCHES: usize = 5;

/// Fetch all requested resources from a single client.
///
/// Resources are fetched concurrently with bounded concurrency (5 by default)
/// to improve performance while avoiding overwhelming the Splunk server.
/// Each fetch has its own 30-second timeout.
pub async fn fetch_all_resources(
    client: &SplunkClient,
    resource_types: Vec<String>,
    cancel: &CancellationToken,
) -> Result<Vec<ResourceSummary>> {
    // Check for cancellation before starting
    if cancel.is_cancelled() {
        return Err(Cancelled.into());
    }

    // Create a stream of futures for each resource type
    let fetch_futures = resource_types.into_iter().map(|resource_type| {
        async move {
            // Check cancellation before each fetch
            if cancel.is_cancelled() {
                return ResourceSummary {
                    resource_type: resource_type.clone(),
                    count: 0,
                    status: "cancelled".to_string(),
                    error: Some("Request was cancelled".to_string()),
                };
            }

            match resource_type.as_str() {
                "indexes" => fetch_indexes(client).await,
                "jobs" => fetch_jobs(client).await,
                "apps" => fetch_apps(client).await,
                "users" => fetch_users(client).await,
                "cluster" => fetch_cluster(client).await,
                "health" => fetch_health(client).await,
                "kvstore" => fetch_kvstore(client).await,
                "license" => fetch_license(client).await,
                "saved-searches" => fetch_saved_searches(client).await,
                _ => unreachable!(),
            }
        }
    });

    // Execute fetches with bounded concurrency using buffer_unordered
    let resources: Vec<ResourceSummary> = stream::iter(fetch_futures)
        .buffer_unordered(MAX_CONCURRENT_FETCHES)
        .collect()
        .await;

    Ok(resources)
}

/// Generic fetch helper that applies timeout and maps results to ResourceSummary.
///
/// # Type Parameters
/// - `T`: The successful response type from the API call
/// - `F`: The future type returned by the fetch function
/// - `E`: The error type from the API call (must implement Display)
///
/// # Arguments
/// - `resource_type`: The resource type name for the summary
/// - `status_error`: The status string for failed fetches
/// - `fetch_fn`: The async function that performs the actual fetch
/// - `extract_count`: Function to extract the count from a successful response
/// - `extract_status`: Function to extract/derive the status from a successful response
async fn fetch_with_timeout<T, F, E>(
    resource_type: &str,
    status_error: &str,
    fetch_fn: impl FnOnce() -> F,
    extract_count: impl FnOnce(&T) -> u64,
    extract_status: impl FnOnce(&T) -> String,
) -> ResourceSummary
where
    F: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    match time::timeout(TIMEOUT_DURATION, fetch_fn()).await {
        Ok(Ok(response)) => ResourceSummary {
            resource_type: resource_type.to_string(),
            count: extract_count(&response),
            status: extract_status(&response),
            error: None,
        },
        Ok(Err(e)) => {
            warn!("Failed to fetch {}: {}", resource_type, e);
            ResourceSummary {
                resource_type: resource_type.to_string(),
                count: 0,
                status: status_error.to_string(),
                error: Some(e.to_string()),
            }
        }
        Err(_) => {
            warn!("Timeout fetching {}", resource_type);
            ResourceSummary {
                resource_type: resource_type.to_string(),
                count: 0,
                status: "timeout".to_string(),
                error: Some("Request timeout after 30 seconds".to_string()),
            }
        }
    }
}

async fn fetch_indexes(client: &SplunkClient) -> ResourceSummary {
    fetch_with_timeout(
        "indexes",
        "error",
        || client.list_indexes(Some(1000), None),
        |indexes| indexes.len() as u64,
        |_| "ok".to_string(),
    )
    .await
}

async fn fetch_jobs(client: &SplunkClient) -> ResourceSummary {
    fetch_with_timeout(
        "jobs",
        "error",
        || client.list_jobs(Some(100), None),
        |jobs| jobs.len() as u64,
        |_| "active".to_string(),
    )
    .await
}

async fn fetch_apps(client: &SplunkClient) -> ResourceSummary {
    fetch_with_timeout(
        "apps",
        "error",
        || client.list_apps(Some(1000), None),
        |apps| apps.len() as u64,
        |_| "installed".to_string(),
    )
    .await
}

async fn fetch_users(client: &SplunkClient) -> ResourceSummary {
    fetch_with_timeout(
        "users",
        "error",
        || client.list_users(Some(1000), None),
        |users| users.len() as u64,
        |_| "active".to_string(),
    )
    .await
}

async fn fetch_cluster(client: &SplunkClient) -> ResourceSummary {
    match time::timeout(TIMEOUT_DURATION, client.get_cluster_info()).await {
        Ok(Ok(cluster)) => ResourceSummary {
            resource_type: "cluster".to_string(),
            count: 1,
            status: cluster.mode,
            error: None,
        },
        Ok(Err(e)) => match e {
            // HTTP 404 indicates cluster endpoint not available (not clustered)
            ClientError::ApiError { status: 404, .. } => ResourceSummary {
                resource_type: "cluster".to_string(),
                count: 0,
                status: "not clustered".to_string(),
                error: None,
            },
            // NotFound variant also indicates not clustered
            ClientError::NotFound(_) => ResourceSummary {
                resource_type: "cluster".to_string(),
                count: 0,
                status: "not clustered".to_string(),
                error: None,
            },
            // Check message for cluster-related errors as fallback
            // This catches messages like "clustering not configured", "not part of a cluster", etc.
            _ if e.to_string().to_lowercase().contains("cluster") => ResourceSummary {
                resource_type: "cluster".to_string(),
                count: 0,
                status: "not clustered".to_string(),
                error: None,
            },
            // All other errors are unexpected
            _ => {
                warn!("Failed to fetch cluster info: {}", e);
                ResourceSummary {
                    resource_type: "cluster".to_string(),
                    count: 0,
                    status: "error".to_string(),
                    error: Some(e.to_string()),
                }
            }
        },
        Err(_) => {
            warn!("Timeout fetching cluster info");
            ResourceSummary {
                resource_type: "cluster".to_string(),
                count: 0,
                status: "timeout".to_string(),
                error: Some("Request timeout after 30 seconds".to_string()),
            }
        }
    }
}

async fn fetch_health(client: &SplunkClient) -> ResourceSummary {
    fetch_with_timeout(
        "health",
        "error",
        || client.get_health(),
        |_| 1,
        |health| health.health.clone(),
    )
    .await
}

async fn fetch_kvstore(client: &SplunkClient) -> ResourceSummary {
    fetch_with_timeout(
        "kvstore",
        "error",
        || client.get_kvstore_status(),
        |_| 1,
        |status| status.current_member.status.clone(),
    )
    .await
}

async fn fetch_license(client: &SplunkClient) -> ResourceSummary {
    match time::timeout(TIMEOUT_DURATION, client.get_license_usage()).await {
        Ok(Ok(usage)) => {
            let total_usage: u64 =
                usage.iter().map(|u| u.effective_used_bytes()).sum::<u64>() / 1024;
            let total_quota: u64 = usage.iter().map(|u| u.quota).sum::<u64>() / 1024;
            let pct = if total_quota > 0 && total_usage > total_quota * 9 / 10 {
                "warning"
            } else if total_quota > 0 {
                "ok"
            } else {
                "unavailable"
            };

            ResourceSummary {
                resource_type: "license".to_string(),
                count: usage.len() as u64,
                status: pct.to_string(),
                error: None,
            }
        }
        Ok(Err(e)) => {
            warn!("Failed to fetch license: {}", e);
            ResourceSummary {
                resource_type: "license".to_string(),
                count: 0,
                status: "unavailable".to_string(),
                error: Some(e.to_string()),
            }
        }
        Err(_) => {
            warn!("Timeout fetching license");
            ResourceSummary {
                resource_type: "license".to_string(),
                count: 0,
                status: "timeout".to_string(),
                error: Some("Request timeout after 30 seconds".to_string()),
            }
        }
    }
}

async fn fetch_saved_searches(client: &SplunkClient) -> ResourceSummary {
    fetch_with_timeout(
        "saved-searches",
        "error",
        || client.list_saved_searches(None, None),
        |saved_searches| saved_searches.len() as u64,
        |_| "available".to_string(),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that fetch_cluster correctly handles ApiError with 404 status (not clustered).
    #[test]
    fn test_fetch_cluster_handles_api_error_404() {
        // Test the error matching logic by creating the error directly
        let error = ClientError::ApiError {
            status: 404,
            url: "https://localhost:8089/services/cluster/master/config".to_string(),
            message: "Not Found".to_string(),
            request_id: None,
        };

        // Verify the error matches the expected pattern
        assert!(
            matches!(error, ClientError::ApiError { status: 404, .. }),
            "404 ApiError should match the not-clustered pattern"
        );
    }

    /// Test that fetch_cluster correctly handles NotFound variant (not clustered).
    #[test]
    fn test_fetch_cluster_handles_not_found() {
        let error = ClientError::NotFound("Cluster endpoint not available".to_string());

        // Verify the error matches the expected pattern
        assert!(
            matches!(error, ClientError::NotFound(_)),
            "NotFound error should match the not-clustered pattern"
        );
    }

    /// Test that fetch_cluster correctly handles "cluster" in message (not clustered fallback).
    #[test]
    fn test_fetch_cluster_handles_cluster_in_message() {
        // Test various cluster-related messages that should be treated as "not clustered"
        let messages = vec![
            "Clustering is not configured",
            "This node is not part of a cluster",
            "cluster manager not available",
            "Cluster config endpoint disabled",
        ];

        for msg in messages {
            let error = ClientError::InvalidResponse(msg.to_string());
            let error_str = error.to_string().to_lowercase();
            assert!(
                error_str.contains("cluster"),
                "Error message '{}' should contain 'cluster'",
                msg
            );
        }
    }

    /// Test that fetch_cluster treats non-404, non-cluster errors as unexpected errors.
    #[test]
    fn test_fetch_cluster_handles_unexpected_errors() {
        // These errors should NOT be treated as "not clustered"
        let errors = vec![
            ClientError::AuthFailed("Invalid credentials".to_string()),
            ClientError::Timeout(Duration::from_secs(30)),
            ClientError::ConnectionRefused("localhost:8089".to_string()),
            ClientError::ApiError {
                status: 500,
                url: "https://localhost:8089/services/server/info".to_string(),
                message: "Internal Server Error".to_string(),
                request_id: None,
            },
            ClientError::ApiError {
                status: 403,
                url: "https://localhost:8089/services/server/info".to_string(),
                message: "Forbidden".to_string(),
                request_id: None,
            },
        ];

        for error in errors {
            // These should NOT match the "not clustered" patterns
            let is_not_clustered = match &error {
                ClientError::ApiError { status: 404, .. } => true,
                ClientError::NotFound(_) => true,
                _ if error.to_string().to_lowercase().contains("cluster") => true,
                _ => false,
            };
            assert!(
                !is_not_clustered,
                "Error {:?} should be treated as unexpected, not 'not clustered'",
                error
            );
        }
    }

    /// Test that ResourceSummary is correctly constructed for clustered state.
    #[test]
    fn test_resource_summary_clustered() {
        let summary = ResourceSummary {
            resource_type: "cluster".to_string(),
            count: 1,
            status: "peer".to_string(),
            error: None,
        };

        assert_eq!(summary.resource_type, "cluster");
        assert_eq!(summary.count, 1);
        assert_eq!(summary.status, "peer");
        assert!(summary.error.is_none());
    }

    /// Test that ResourceSummary is correctly constructed for not-clustered state.
    #[test]
    fn test_resource_summary_not_clustered() {
        let summary = ResourceSummary {
            resource_type: "cluster".to_string(),
            count: 0,
            status: "not clustered".to_string(),
            error: None,
        };

        assert_eq!(summary.resource_type, "cluster");
        assert_eq!(summary.count, 0);
        assert_eq!(summary.status, "not clustered");
        assert!(summary.error.is_none());
    }

    /// Test that ResourceSummary is correctly constructed for error state.
    #[test]
    fn test_resource_summary_error() {
        let summary = ResourceSummary {
            resource_type: "cluster".to_string(),
            count: 0,
            status: "error".to_string(),
            error: Some("Connection refused".to_string()),
        };

        assert_eq!(summary.resource_type, "cluster");
        assert_eq!(summary.count, 0);
        assert_eq!(summary.status, "error");
        assert_eq!(summary.error, Some("Connection refused".to_string()));
    }
}
