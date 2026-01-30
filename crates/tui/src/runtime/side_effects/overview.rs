//! Overview screen side effect handlers.
//!
//! Responsibilities:
//! - Handle async API calls for overview resource aggregation.
//! - Fetch all resource types and compile into OverviewData.
//!
//! Does NOT handle:
//! - Direct state modification (sends actions for that).
//! - UI rendering.

use crate::action::{Action, OverviewData, OverviewResource};
use splunk_client::ClientError;
use tokio::sync::mpsc::Sender;

use super::SharedClient;

/// Handle loading overview information from all resource endpoints.
pub async fn handle_load_overview(client: SharedClient, tx: Sender<Action>) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut c = client.lock().await;
        let mut resources = Vec::new();

        // Fetch each resource type with timeout
        // Individual failures are converted to error entries rather than failing the entire overview
        // Follow the same pattern as CLI fetchers.rs

        // indexes
        match fetch_indexes(&mut c).await {
            Ok(r) => resources.push(r),
            Err(e) => resources.push(resource_error("indexes", e)),
        }

        // jobs
        match fetch_jobs(&mut c).await {
            Ok(r) => resources.push(r),
            Err(e) => resources.push(resource_error("jobs", e)),
        }

        // apps
        match fetch_apps(&mut c).await {
            Ok(r) => resources.push(r),
            Err(e) => resources.push(resource_error("apps", e)),
        }

        // users
        match fetch_users(&mut c).await {
            Ok(r) => resources.push(r),
            Err(e) => resources.push(resource_error("users", e)),
        }

        // cluster
        match fetch_cluster(&mut c).await {
            Ok(r) => resources.push(r),
            Err(e) => resources.push(resource_error("cluster", e)),
        }

        // health
        match fetch_health(&mut c).await {
            Ok(r) => resources.push(r),
            Err(e) => resources.push(resource_error("health", e)),
        }

        // kvstore
        match fetch_kvstore(&mut c).await {
            Ok(r) => resources.push(r),
            Err(e) => resources.push(resource_error("kvstore", e)),
        }

        // license
        match fetch_license(&mut c).await {
            Ok(r) => resources.push(r),
            Err(e) => resources.push(resource_error("license", e)),
        }

        // saved-searches
        match fetch_saved_searches(&mut c).await {
            Ok(r) => resources.push(r),
            Err(e) => resources.push(resource_error("saved-searches", e)),
        }

        let overview_data = OverviewData { resources };
        let _ = tx.send(Action::OverviewLoaded(overview_data)).await;
    });
}

/// Create an OverviewResource representing a failed fetch.
fn resource_error(resource_type: &str, error: ClientError) -> OverviewResource {
    OverviewResource {
        resource_type: resource_type.to_string(),
        count: 0,
        status: "error".to_string(),
        error: Some(error.to_string()),
    }
}

async fn fetch_indexes(
    client: &mut splunk_client::SplunkClient,
) -> Result<OverviewResource, ClientError> {
    match tokio::time::timeout(
        std::time::Duration::from_secs(30),
        client.list_indexes(Some(1000), None),
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
        Err(_) => Err(ClientError::Timeout(std::time::Duration::from_secs(30))),
    }
}

async fn fetch_jobs(
    client: &mut splunk_client::SplunkClient,
) -> Result<OverviewResource, ClientError> {
    match tokio::time::timeout(
        std::time::Duration::from_secs(30),
        client.list_jobs(Some(100), None),
    )
    .await
    {
        Ok(Ok(jobs)) => Ok(OverviewResource {
            resource_type: "jobs".to_string(),
            count: jobs.len() as u64,
            status: "active".to_string(),
            error: None,
        }),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(ClientError::Timeout(std::time::Duration::from_secs(30))),
    }
}

async fn fetch_apps(
    client: &mut splunk_client::SplunkClient,
) -> Result<OverviewResource, ClientError> {
    match tokio::time::timeout(
        std::time::Duration::from_secs(30),
        client.list_apps(Some(1000), None),
    )
    .await
    {
        Ok(Ok(apps)) => Ok(OverviewResource {
            resource_type: "apps".to_string(),
            count: apps.len() as u64,
            status: "installed".to_string(),
            error: None,
        }),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(ClientError::Timeout(std::time::Duration::from_secs(30))),
    }
}

async fn fetch_users(
    client: &mut splunk_client::SplunkClient,
) -> Result<OverviewResource, ClientError> {
    match tokio::time::timeout(
        std::time::Duration::from_secs(30),
        client.list_users(Some(1000), None),
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
        Err(_) => Err(ClientError::Timeout(std::time::Duration::from_secs(30))),
    }
}

async fn fetch_cluster(
    client: &mut splunk_client::SplunkClient,
) -> Result<OverviewResource, ClientError> {
    match tokio::time::timeout(
        std::time::Duration::from_secs(30),
        client.get_cluster_info(),
    )
    .await
    {
        Ok(Ok(cluster)) => Ok(OverviewResource {
            resource_type: "cluster".to_string(),
            count: 1,
            status: cluster.mode,
            error: None,
        }),
        Ok(Err(e)) => match e {
            // HTTP 404 indicates cluster endpoint not available (not clustered)
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
            // Check message for cluster-related errors as fallback
            _ if e.to_string().to_lowercase().contains("cluster") => Ok(OverviewResource {
                resource_type: "cluster".to_string(),
                count: 0,
                status: "not clustered".to_string(),
                error: None,
            }),
            _ => Err(e),
        },
        Err(_) => Err(ClientError::Timeout(std::time::Duration::from_secs(30))),
    }
}

async fn fetch_health(
    client: &mut splunk_client::SplunkClient,
) -> Result<OverviewResource, ClientError> {
    match tokio::time::timeout(std::time::Duration::from_secs(30), client.get_health()).await {
        Ok(Ok(health)) => Ok(OverviewResource {
            resource_type: "health".to_string(),
            count: 1,
            status: health.health.clone(),
            error: None,
        }),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(ClientError::Timeout(std::time::Duration::from_secs(30))),
    }
}

async fn fetch_kvstore(
    client: &mut splunk_client::SplunkClient,
) -> Result<OverviewResource, ClientError> {
    match tokio::time::timeout(
        std::time::Duration::from_secs(30),
        client.get_kvstore_status(),
    )
    .await
    {
        Ok(Ok(status)) => Ok(OverviewResource {
            resource_type: "kvstore".to_string(),
            count: 1,
            status: status.current_member.status.clone(),
            error: None,
        }),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(ClientError::Timeout(std::time::Duration::from_secs(30))),
    }
}

async fn fetch_license(
    client: &mut splunk_client::SplunkClient,
) -> Result<OverviewResource, ClientError> {
    match tokio::time::timeout(
        std::time::Duration::from_secs(30),
        client.get_license_usage(),
    )
    .await
    {
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
        Err(_) => Err(ClientError::Timeout(std::time::Duration::from_secs(30))),
    }
}

async fn fetch_saved_searches(
    client: &mut splunk_client::SplunkClient,
) -> Result<OverviewResource, ClientError> {
    match tokio::time::timeout(
        std::time::Duration::from_secs(30),
        client.list_saved_searches(None, None),
    )
    .await
    {
        Ok(Ok(saved_searches)) => Ok(OverviewResource {
            resource_type: "saved-searches".to_string(),
            count: saved_searches.len() as u64,
            status: "available".to_string(),
            error: None,
        }),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(ClientError::Timeout(std::time::Duration::from_secs(30))),
    }
}
