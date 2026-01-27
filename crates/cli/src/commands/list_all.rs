//! List-all command implementation - minimal working version.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use splunk_client::SplunkClient;
use std::time::Duration;
use tokio::time;
use tracing::{info, warn};

use crate::cancellation::Cancelled;
use crate::commands::convert_auth_strategy;
use crate::formatters::{OutputFormat, get_formatter, write_to_file};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSummary {
    pub resource_type: String,
    pub count: u64,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListAllOutput {
    pub timestamp: String,
    pub resources: Vec<ResourceSummary>,
}

const VALID_RESOURCES: &[&str] = &[
    "indexes",
    "jobs",
    "apps",
    "users",
    "cluster",
    "health",
    "kvstore",
    "license",
    "saved-searches",
];

fn format_timestamp() -> String {
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    let nsecs = duration.subsec_nanos();
    format!("{}.{:09}Z", secs, nsecs)
}

pub async fn run(
    config: splunk_config::Config,
    resources_filter: Option<Vec<String>>,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    _cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Listing all Splunk resources");

    let auth_strategy = convert_auth_strategy(&config.auth.strategy);

    let mut client = SplunkClient::builder()
        .base_url(config.connection.base_url)
        .auth_strategy(auth_strategy)
        .skip_verify(config.connection.skip_verify)
        .timeout(config.connection.timeout)
        .build()?;

    let resources_to_fetch =
        resources_filter.unwrap_or_else(|| VALID_RESOURCES.iter().map(|s| s.to_string()).collect());

    for resource in &resources_to_fetch {
        if !VALID_RESOURCES.contains(&resource.as_str()) {
            anyhow::bail!(
                "Invalid resource type: {}. Valid types: {}",
                resource,
                VALID_RESOURCES.join(", ")
            );
        }
    }

    let resources = fetch_all_resources(&mut client, resources_to_fetch, _cancel).await?;

    let output = ListAllOutput {
        timestamp: format_timestamp(),
        resources,
    };

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);

    let formatted = formatter.format_list_all(&output)?;
    if let Some(ref path) = output_file {
        write_to_file(&formatted, path)
            .with_context(|| format!("Failed to write output to {}", path.display()))?;
        eprintln!(
            "Results written to {} ({:?} format)",
            path.display(),
            format
        );
    } else {
        print!("{}", formatted);
    }

    Ok(())
}

async fn fetch_all_resources(
    client: &mut SplunkClient,
    resource_types: Vec<String>,
    _cancel: &crate::cancellation::CancellationToken,
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
            _ = _cancel.cancelled() => return Err(Cancelled.into()),
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
        Ok(Err(e)) => {
            let error_msg = e.to_string();
            if error_msg.contains("cluster")
                || error_msg.contains("404")
                || error_msg.contains("not configured")
            {
                ResourceSummary {
                    resource_type: "cluster".to_string(),
                    count: 0,
                    status: "not clustered".to_string(),
                    error: None,
                }
            } else {
                warn!("Failed to fetch cluster info: {}", e);
                ResourceSummary {
                    resource_type: "cluster".to_string(),
                    count: 0,
                    status: "error".to_string(),
                    error: Some(e.to_string()),
                }
            }
        }
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

    match time::timeout(timeout_duration, client.list_saved_searches()).await {
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
