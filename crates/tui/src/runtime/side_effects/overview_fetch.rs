//! Shared overview resource fetch helpers.
//!
//! This module provides fetch helpers used by both the Overview and Multi-Instance
//! screens to avoid code duplication.

use crate::action::OverviewResource;
use splunk_client::ClientError;

/// Create an OverviewResource representing a failed fetch.
pub fn resource_error(resource_type: &str, error: ClientError) -> OverviewResource {
    OverviewResource {
        resource_type: resource_type.to_string(),
        count: 0,
        status: "error".to_string(),
        error: Some(error.to_string()),
    }
}

/// Standard timeout for overview resource fetches.
pub const FETCH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// List limit for indexes, apps, and users.
pub const LIST_LIMIT_1000: u64 = 1000;

/// List limit for jobs.
pub const LIST_LIMIT_100: u64 = 100;

/// Fetch indexes with timeout.
pub async fn fetch_indexes(
    client: &mut splunk_client::SplunkClient,
) -> Result<OverviewResource, ClientError> {
    match tokio::time::timeout(
        FETCH_TIMEOUT,
        client.list_indexes(Some(LIST_LIMIT_1000), None),
    )
    .await
    {
        Ok(Ok(indexes)) => Ok(OverviewResource {
            resource_type: "indexes".to_string(),
            count: indexes.len() as u64,
            status: "ok".to_string(),
            error: None,
        }),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(ClientError::Timeout(FETCH_TIMEOUT)),
    }
}

/// Fetch jobs with timeout.
pub async fn fetch_jobs(
    client: &mut splunk_client::SplunkClient,
) -> Result<OverviewResource, ClientError> {
    match tokio::time::timeout(FETCH_TIMEOUT, client.list_jobs(Some(LIST_LIMIT_100), None)).await {
        Ok(Ok(jobs)) => Ok(OverviewResource {
            resource_type: "jobs".to_string(),
            count: jobs.len() as u64,
            status: "active".to_string(),
            error: None,
        }),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(ClientError::Timeout(FETCH_TIMEOUT)),
    }
}

/// Fetch apps with timeout.
pub async fn fetch_apps(
    client: &mut splunk_client::SplunkClient,
) -> Result<OverviewResource, ClientError> {
    match tokio::time::timeout(FETCH_TIMEOUT, client.list_apps(Some(LIST_LIMIT_1000), None)).await {
        Ok(Ok(apps)) => Ok(OverviewResource {
            resource_type: "apps".to_string(),
            count: apps.len() as u64,
            status: "installed".to_string(),
            error: None,
        }),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(ClientError::Timeout(FETCH_TIMEOUT)),
    }
}

/// Fetch users with timeout.
pub async fn fetch_users(
    client: &mut splunk_client::SplunkClient,
) -> Result<OverviewResource, ClientError> {
    match tokio::time::timeout(
        FETCH_TIMEOUT,
        client.list_users(Some(LIST_LIMIT_1000), None),
    )
    .await
    {
        Ok(Ok(users)) => Ok(OverviewResource {
            resource_type: "users".to_string(),
            count: users.len() as u64,
            status: "active".to_string(),
            error: None,
        }),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(ClientError::Timeout(FETCH_TIMEOUT)),
    }
}

/// Fetch cluster info with timeout.
pub async fn fetch_cluster(
    client: &mut splunk_client::SplunkClient,
) -> Result<OverviewResource, ClientError> {
    match tokio::time::timeout(FETCH_TIMEOUT, client.get_cluster_info()).await {
        Ok(Ok(cluster)) => Ok(OverviewResource {
            resource_type: "cluster".to_string(),
            count: 1,
            status: cluster.mode,
            error: None,
        }),
        Ok(Err(e)) => match e {
            ClientError::ApiError { status: 404, .. } => Ok(OverviewResource {
                resource_type: "cluster".to_string(),
                count: 0,
                status: "not clustered".to_string(),
                error: None,
            }),
            ClientError::NotFound(_) => Ok(OverviewResource {
                resource_type: "cluster".to_string(),
                count: 0,
                status: "not clustered".to_string(),
                error: None,
            }),
            _ if e.to_string().to_lowercase().contains("cluster") => Ok(OverviewResource {
                resource_type: "cluster".to_string(),
                count: 0,
                status: "not clustered".to_string(),
                error: None,
            }),
            _ => Err(e),
        },
        Err(_) => Err(ClientError::Timeout(FETCH_TIMEOUT)),
    }
}

/// Fetch health with timeout.
pub async fn fetch_health(
    client: &mut splunk_client::SplunkClient,
) -> Result<OverviewResource, ClientError> {
    match tokio::time::timeout(FETCH_TIMEOUT, client.get_health()).await {
        Ok(Ok(health)) => Ok(OverviewResource {
            resource_type: "health".to_string(),
            count: 1,
            status: health.health.clone(),
            error: None,
        }),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(ClientError::Timeout(FETCH_TIMEOUT)),
    }
}

/// Fetch KVStore status with timeout.
pub async fn fetch_kvstore(
    client: &mut splunk_client::SplunkClient,
) -> Result<OverviewResource, ClientError> {
    match tokio::time::timeout(FETCH_TIMEOUT, client.get_kvstore_status()).await {
        Ok(Ok(status)) => Ok(OverviewResource {
            resource_type: "kvstore".to_string(),
            count: 1,
            status: status.current_member.status.clone(),
            error: None,
        }),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(ClientError::Timeout(FETCH_TIMEOUT)),
    }
}

/// Fetch license usage with timeout.
pub async fn fetch_license(
    client: &mut splunk_client::SplunkClient,
) -> Result<OverviewResource, ClientError> {
    match tokio::time::timeout(FETCH_TIMEOUT, client.get_license_usage()).await {
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

            Ok(OverviewResource {
                resource_type: "license".to_string(),
                count: usage.len() as u64,
                status: pct.to_string(),
                error: None,
            })
        }
        Ok(Err(e)) => Err(e),
        Err(_) => Err(ClientError::Timeout(FETCH_TIMEOUT)),
    }
}

/// Fetch saved searches with timeout.
pub async fn fetch_saved_searches(
    client: &mut splunk_client::SplunkClient,
) -> Result<OverviewResource, ClientError> {
    match tokio::time::timeout(FETCH_TIMEOUT, client.list_saved_searches(None, None)).await {
        Ok(Ok(saved_searches)) => Ok(OverviewResource {
            resource_type: "saved-searches".to_string(),
            count: saved_searches.len() as u64,
            status: "available".to_string(),
            error: None,
        }),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(ClientError::Timeout(FETCH_TIMEOUT)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_error_creates_correct_resource() {
        let error = ClientError::Timeout(std::time::Duration::from_secs(30));
        let resource = resource_error("indexes", error);

        assert_eq!(resource.resource_type, "indexes");
        assert_eq!(resource.count, 0);
        assert_eq!(resource.status, "error");
        assert!(resource.error.is_some());
        // Error message format is "Request timed out after ..." (lowercase "timed out")
        assert!(resource.error.unwrap().to_lowercase().contains("timed out"));
    }

    #[test]
    fn test_timeout_duration_constant() {
        // Verify the timeout constant is 30 seconds as expected
        assert_eq!(FETCH_TIMEOUT, std::time::Duration::from_secs(30));
    }

    #[test]
    fn test_list_limits() {
        assert_eq!(LIST_LIMIT_1000, 1000);
        assert_eq!(LIST_LIMIT_100, 100);
    }
}
