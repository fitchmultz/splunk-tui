//! Multi-instance dashboard side effect handler.
//!
//! Responsibilities:
//! - Fetch overview data from all configured profiles in parallel.
//! - Aggregate results into MultiInstanceOverviewData for the dashboard.
//! - Handle per-profile errors gracefully without failing the entire dashboard.
//!
//! Does NOT handle:
//! - Direct state modification (sends actions for that).
//! - UI rendering.
//!
//! Invariants:
//! - Profile-level errors are captured in InstanceOverview, not propagated.
//! - Timestamp is always RFC3339 format.
//! - All futures are joined for concurrent execution.

use crate::action::{Action, InstanceOverview, MultiInstanceOverviewData};
use splunk_config::ConfigManager;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc::Sender;

use super::overview_fetch;

/// Handle loading multi-instance overview from all configured profiles.
pub async fn handle_load_multi_instance_overview(
    config_manager: Arc<Mutex<ConfigManager>>,
    tx: Sender<Action>,
) {
    let _ = tx.send(Action::Loading(true)).await;

    tokio::spawn(async move {
        let cm = config_manager.lock().await;
        let profiles = cm.list_profiles().clone();
        drop(cm); // Release lock before async operations

        let timestamp = chrono::Utc::now().to_rfc3339();
        let mut futures = Vec::new();

        for (profile_name, profile_config) in profiles {
            let future = fetch_single_instance(profile_name, profile_config);
            futures.push(future);
        }

        let results = futures_util::future::join_all(futures).await;
        let instances: Vec<InstanceOverview> = results.into_iter().collect();

        let data = MultiInstanceOverviewData {
            timestamp,
            instances,
        };

        let _ = tx.send(Action::MultiInstanceOverviewLoaded(data)).await;
    });
}

/// Fetch overview data from a single profile.
async fn fetch_single_instance(
    profile_name: String,
    profile_config: splunk_config::types::ProfileConfig,
) -> InstanceOverview {
    let base_url = profile_config.base_url.clone().unwrap_or_default();

    // Build auth strategy
    let auth_strategy = match build_auth_strategy(&profile_config) {
        Ok(strategy) => strategy,
        Err(error_msg) => {
            return InstanceOverview {
                profile_name,
                base_url,
                resources: vec![],
                error: Some(error_msg),
                health_status: "error".to_string(),
                job_count: 0,
            };
        }
    };

    // Build Splunk client
    let mut client = match splunk_client::SplunkClient::builder()
        .base_url(base_url.clone())
        .auth_strategy(auth_strategy)
        .skip_verify(profile_config.skip_verify.unwrap_or(false))
        .timeout(std::time::Duration::from_secs(
            profile_config.timeout_seconds.unwrap_or(30),
        ))
        .session_ttl_seconds(profile_config.session_ttl_seconds.unwrap_or(3600))
        .session_expiry_buffer_seconds(profile_config.session_expiry_buffer_seconds.unwrap_or(60))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return InstanceOverview {
                profile_name,
                base_url,
                resources: vec![],
                error: Some(format!("Failed to build client: {}", e)),
                health_status: "error".to_string(),
                job_count: 0,
            };
        }
    };

    // Fetch all resources
    let mut resources = Vec::new();
    let mut health_status = "unknown".to_string();
    let mut job_count = 0u64;

    // Fetch health
    match overview_fetch::fetch_health(&mut client).await {
        Ok(r) => {
            health_status = r.status.clone();
            resources.push(r);
        }
        Err(e) => {
            resources.push(overview_fetch::resource_error("health", e));
        }
    }

    // Fetch jobs
    match overview_fetch::fetch_jobs(&mut client).await {
        Ok(r) => {
            job_count = r.count;
            resources.push(r);
        }
        Err(e) => {
            resources.push(overview_fetch::resource_error("jobs", e));
        }
    }

    // Fetch indexes
    match overview_fetch::fetch_indexes(&mut client).await {
        Ok(r) => resources.push(r),
        Err(e) => resources.push(overview_fetch::resource_error("indexes", e)),
    }

    // Fetch apps
    match overview_fetch::fetch_apps(&mut client).await {
        Ok(r) => resources.push(r),
        Err(e) => resources.push(overview_fetch::resource_error("apps", e)),
    }

    // Fetch users
    match overview_fetch::fetch_users(&mut client).await {
        Ok(r) => resources.push(r),
        Err(e) => resources.push(overview_fetch::resource_error("users", e)),
    }

    // Fetch cluster
    match overview_fetch::fetch_cluster(&mut client).await {
        Ok(r) => resources.push(r),
        Err(e) => resources.push(overview_fetch::resource_error("cluster", e)),
    }

    // Fetch kvstore
    match overview_fetch::fetch_kvstore(&mut client).await {
        Ok(r) => resources.push(r),
        Err(e) => resources.push(overview_fetch::resource_error("kvstore", e)),
    }

    // Fetch license
    match overview_fetch::fetch_license(&mut client).await {
        Ok(r) => resources.push(r),
        Err(e) => resources.push(overview_fetch::resource_error("license", e)),
    }

    // Fetch saved searches
    match overview_fetch::fetch_saved_searches(&mut client).await {
        Ok(r) => resources.push(r),
        Err(e) => resources.push(overview_fetch::resource_error("saved-searches", e)),
    }

    InstanceOverview {
        profile_name,
        base_url,
        resources,
        error: None,
        health_status,
        job_count,
    }
}

/// Build authentication strategy from profile configuration.
fn build_auth_strategy(
    profile_config: &splunk_config::types::ProfileConfig,
) -> Result<splunk_client::AuthStrategy, String> {
    // Check for API token first
    if let Some(ref token_secure) = profile_config.api_token {
        match token_secure.resolve() {
            Ok(token) => {
                return Ok(splunk_client::AuthStrategy::ApiToken { token });
            }
            Err(e) => {
                return Err(format!("Failed to resolve API token from keyring: {}", e));
            }
        }
    }

    // Fall back to username/password
    if let (Some(username), Some(password_secure)) =
        (&profile_config.username, &profile_config.password)
    {
        match password_secure.resolve() {
            Ok(password) => {
                return Ok(splunk_client::AuthStrategy::SessionToken {
                    username: username.clone(),
                    password,
                });
            }
            Err(e) => {
                return Err(format!("Failed to resolve password from keyring: {}", e));
            }
        }
    }

    Err("No authentication credentials found in profile".to_string())
}
