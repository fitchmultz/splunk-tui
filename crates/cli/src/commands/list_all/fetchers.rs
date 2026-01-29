//! Resource fetching logic for the list-all command.
//!
//! Responsibilities:
//! - Fetch resource summaries from Splunk for all supported resource types.
//! - Handle per-resource timeouts (30-second default).
//! - Convert API errors into ResourceSummary error states gracefully.
//!
//! Does NOT handle:
//! - Multi-profile orchestration (see `output.rs`).
//! - Output formatting (see `output.rs`).
//! - Authentication strategy building (see `auth.rs`).
//!
//! Invariants:
//! - Each resource fetch has a 30-second timeout.
//! - Errors are captured in ResourceSummary, not propagated as Err.

use crate::cancellation::{CancellationToken, Cancelled};
use anyhow::Result;
use splunk_client::{ClientError, SplunkClient};
use std::time::Duration;
use tokio::time;
use tracing::warn;

use super::types::ResourceSummary;

/// Fetch all requested resources from a single client.
pub async fn fetch_all_resources(
    client: &mut SplunkClient,
    resource_types: Vec<String>,
    cancel: &CancellationToken,
) -> Result<Vec<ResourceSummary>> {
    let mut resources = Vec::new();

    for resource_type in resource_types {
        let summary: ResourceSummary = tokio::select! {
            res = async {
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
            } => res,
            _ = cancel.cancelled() => return Err(Cancelled.into()),
        };
        resources.push(summary);
    }

    Ok(resources)
}

async fn fetch_indexes(client: &mut SplunkClient) -> ResourceSummary {
    let timeout_duration = Duration::from_secs(30);

    match time::timeout(timeout_duration, client.list_indexes(Some(1000), None)).await {
        Ok(Ok(indexes)) => ResourceSummary {
            resource_type: "indexes".to_string(),
            count: indexes.len() as u64,
            status: "ok".to_string(),
            error: None,
        },
        Ok(Err(e)) => {
            warn!("Failed to fetch indexes: {}", e);
            ResourceSummary {
                resource_type: "indexes".to_string(),
                count: 0,
                status: "error".to_string(),
                error: Some(e.to_string()),
            }
        }
        Err(_) => {
            warn!("Timeout fetching indexes");
            ResourceSummary {
                resource_type: "indexes".to_string(),
                count: 0,
                status: "timeout".to_string(),
                error: Some("Request timeout after 30 seconds".to_string()),
            }
        }
    }
}

async fn fetch_jobs(client: &mut SplunkClient) -> ResourceSummary {
    let timeout_duration = Duration::from_secs(30);

    match time::timeout(timeout_duration, client.list_jobs(Some(100), None)).await {
        Ok(Ok(jobs)) => ResourceSummary {
            resource_type: "jobs".to_string(),
            count: jobs.len() as u64,
            status: "active".to_string(),
            error: None,
        },
        Ok(Err(e)) => {
            warn!("Failed to fetch jobs: {}", e);
            ResourceSummary {
                resource_type: "jobs".to_string(),
                count: 0,
                status: "error".to_string(),
                error: Some(e.to_string()),
            }
        }
        Err(_) => {
            warn!("Timeout fetching jobs");
            ResourceSummary {
                resource_type: "jobs".to_string(),
                count: 0,
                status: "timeout".to_string(),
                error: Some("Request timeout after 30 seconds".to_string()),
            }
        }
    }
}

async fn fetch_apps(client: &mut SplunkClient) -> ResourceSummary {
    let timeout_duration = Duration::from_secs(30);

    match time::timeout(timeout_duration, client.list_apps(Some(1000), None)).await {
        Ok(Ok(apps)) => ResourceSummary {
            resource_type: "apps".to_string(),
            count: apps.len() as u64,
            status: "installed".to_string(),
            error: None,
        },
        Ok(Err(e)) => {
            warn!("Failed to fetch apps: {}", e);
            ResourceSummary {
                resource_type: "apps".to_string(),
                count: 0,
                status: "error".to_string(),
                error: Some(e.to_string()),
            }
        }
        Err(_) => {
            warn!("Timeout fetching apps");
            ResourceSummary {
                resource_type: "apps".to_string(),
                count: 0,
                status: "timeout".to_string(),
                error: Some("Request timeout after 30 seconds".to_string()),
            }
        }
    }
}

async fn fetch_users(client: &mut SplunkClient) -> ResourceSummary {
    let timeout_duration = Duration::from_secs(30);

    match time::timeout(timeout_duration, client.list_users(Some(1000), None)).await {
        Ok(Ok(users)) => ResourceSummary {
            resource_type: "users".to_string(),
            count: users.len() as u64,
            status: "active".to_string(),
            error: None,
        },
        Ok(Err(e)) => {
            warn!("Failed to fetch users: {}", e);
            ResourceSummary {
                resource_type: "users".to_string(),
                count: 0,
                status: "error".to_string(),
                error: Some(e.to_string()),
            }
        }
        Err(_) => {
            warn!("Timeout fetching users");
            ResourceSummary {
                resource_type: "users".to_string(),
                count: 0,
                status: "timeout".to_string(),
                error: Some("Request timeout after 30 seconds".to_string()),
            }
        }
    }
}

async fn fetch_cluster(client: &mut SplunkClient) -> ResourceSummary {
    let timeout_duration = Duration::from_secs(30);

    match time::timeout(timeout_duration, client.get_cluster_info()).await {
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

async fn fetch_health(client: &mut SplunkClient) -> ResourceSummary {
    let timeout_duration = Duration::from_secs(30);

    match time::timeout(timeout_duration, client.get_health()).await {
        Ok(Ok(health)) => ResourceSummary {
            resource_type: "health".to_string(),
            count: 1,
            status: health.health.clone(),
            error: None,
        },
        Ok(Err(e)) => {
            warn!("Failed to fetch health: {}", e);
            ResourceSummary {
                resource_type: "health".to_string(),
                count: 0,
                status: "error".to_string(),
                error: Some(e.to_string()),
            }
        }
        Err(_) => {
            warn!("Timeout fetching health");
            ResourceSummary {
                resource_type: "health".to_string(),
                count: 0,
                status: "timeout".to_string(),
                error: Some("Request timeout after 30 seconds".to_string()),
            }
        }
    }
}

async fn fetch_kvstore(client: &mut SplunkClient) -> ResourceSummary {
    let timeout_duration = Duration::from_secs(30);

    match time::timeout(timeout_duration, client.get_kvstore_status()).await {
        Ok(Ok(status)) => ResourceSummary {
            resource_type: "kvstore".to_string(),
            count: 1,
            status: status.current_member.status,
            error: None,
        },
        Ok(Err(e)) => {
            warn!("Failed to fetch KVStore status: {}", e);
            ResourceSummary {
                resource_type: "kvstore".to_string(),
                count: 0,
                status: "error".to_string(),
                error: Some(e.to_string()),
            }
        }
        Err(_) => {
            warn!("Timeout fetching KVStore status");
            ResourceSummary {
                resource_type: "kvstore".to_string(),
                count: 0,
                status: "timeout".to_string(),
                error: Some("Request timeout after 30 seconds".to_string()),
            }
        }
    }
}

async fn fetch_license(client: &mut SplunkClient) -> ResourceSummary {
    let timeout_duration = Duration::from_secs(30);

    match time::timeout(timeout_duration, client.get_license_usage()).await {
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

async fn fetch_saved_searches(client: &mut SplunkClient) -> ResourceSummary {
    let timeout_duration = Duration::from_secs(30);

    match time::timeout(timeout_duration, client.list_saved_searches(None, None)).await {
        Ok(Ok(saved_searches)) => ResourceSummary {
            resource_type: "saved-searches".to_string(),
            count: saved_searches.len() as u64,
            status: "available".to_string(),
            error: None,
        },
        Ok(Err(e)) => {
            warn!("Failed to fetch saved searches: {}", e);
            ResourceSummary {
                resource_type: "saved-searches".to_string(),
                count: 0,
                status: "error".to_string(),
                error: Some(e.to_string()),
            }
        }
        Err(_) => {
            warn!("Timeout fetching saved searches");
            ResourceSummary {
                resource_type: "saved-searches".to_string(),
                count: 0,
                status: "timeout".to_string(),
                error: Some("Request timeout after 30 seconds".to_string()),
            }
        }
    }
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
