//! Shared multi-profile aggregation workflows for CLI and TUI.
//!
//! Purpose:
//! - Centralize resource-summary fetching and multi-profile aggregation above the raw client.
//!
//! Responsibilities:
//! - Normalize and validate supported overview resource names.
//! - Build clients from persisted profile configs without borrowing frontend state.
//! - Fetch bounded-concurrency resource summaries from one or many Splunk profiles.
//! - Provide shared multi-instance result types and state-merging rules used by CLI and TUI.
//!
//! Scope:
//! - Shared aggregation models, cancellation checks, and fetch orchestration only.
//!
//! Usage:
//! - CLI list-all uses `fetch_multi_profile_overview`.
//! - TUI multi-instance uses `fetch_multi_instance_overview` and `merge_instance_update`.
//!
//! Invariants/Assumptions:
//! - Cancellation returns an error instead of partial top-level payloads.
//! - Per-resource and per-profile failures are captured in result payloads, not propagated.
//! - Profile configs are cloned before network I/O begins.

use anyhow::Result;
use futures::stream::{self, StreamExt};
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use splunk_config::constants::{
    DEFAULT_EXPIRY_BUFFER_SECS, DEFAULT_HEALTH_CHECK_INTERVAL_SECS, DEFAULT_MAX_RETRIES,
    DEFAULT_SESSION_TTL_SECS, DEFAULT_TIMEOUT_SECS,
};
use splunk_config::{
    AuthConfig as ConfigAuthConfig, AuthStrategy as ConfigAuthStrategy, Config, ConnectionConfig,
    ProfileConfig, SecureValue, default_circuit_breaker_enabled, default_circuit_failure_threshold,
    default_circuit_failure_window, default_circuit_half_open_requests,
    default_circuit_reset_timeout,
};
use std::collections::{HashMap, HashSet};
use std::time::Duration;

use crate::workflows::{CancellationProbe, ensure_not_cancelled};
use crate::{AuthStrategy, ClientError, SplunkClient};

const MAX_CONCURRENT_RESOURCE_FETCHES: usize = 5;
const MAX_CONCURRENT_PROFILE_FETCHES: usize = 4;
const LIST_LIMIT_1000: usize = 1000;
const LIST_LIMIT_100: usize = 100;
const INSTANCE_RETRY_ATTEMPTS: usize = 3;
const INSTANCE_RETRY_BASE_DELAY_MS: u64 = 250;

/// Valid resource types that shared overview aggregation supports.
pub const VALID_RESOURCES: &[&str] = &[
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

/// Shared per-resource summary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceSummary {
    pub resource_type: String,
    pub count: usize,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Shared per-profile output row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfileResult {
    pub profile_name: String,
    pub base_url: String,
    pub resources: Vec<ResourceSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Shared CLI/TUI multi-profile output.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ListAllMultiOutput {
    pub timestamp: String,
    pub profiles: Vec<ProfileResult>,
}

/// Shared instance health state for dashboards.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstanceStatus {
    Healthy,
    Cached,
    Failed,
    Loading,
}

/// Shared per-instance dashboard row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstanceOverview {
    pub profile_name: String,
    pub base_url: String,
    pub resources: Vec<ResourceSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub health_status: String,
    pub job_count: usize,
    pub status: InstanceStatus,
    pub last_success_at: Option<String>,
}

/// Shared multi-instance dashboard payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MultiInstanceOverviewData {
    pub timestamp: String,
    pub instances: Vec<InstanceOverview>,
}

/// Normalize and validate requested resource names.
pub fn normalize_and_validate_resources(
    resources_filter: Option<Vec<String>>,
) -> Result<Vec<String>> {
    let resources: Vec<String> = resources_filter
        .map(|resources| {
            let mut seen = HashSet::new();
            resources
                .into_iter()
                .map(|resource| resource.trim().to_lowercase())
                .filter(|resource| !resource.is_empty())
                .filter(|resource| seen.insert(resource.clone()))
                .collect()
        })
        .unwrap_or_else(|| {
            VALID_RESOURCES
                .iter()
                .map(|resource| resource.to_string())
                .collect()
        });

    for resource in &resources {
        if !VALID_RESOURCES.contains(&resource.as_str()) {
            anyhow::bail!(
                "Invalid resource type: {}. Valid types: {}",
                resource,
                VALID_RESOURCES.join(", ")
            );
        }
    }

    Ok(resources)
}

/// Merge a newly fetched instance update with existing dashboard state.
///
/// Successful updates replace previous state. Failed updates retain the last healthy payload and
/// transition the status to `Cached` so the UI remains honest about stale-but-usable data.
pub fn merge_instance_update(
    existing: Option<&InstanceOverview>,
    mut new_instance: InstanceOverview,
) -> InstanceOverview {
    match existing {
        Some(existing)
            if new_instance.error.is_some() && existing.status == InstanceStatus::Healthy =>
        {
            let mut cached = existing.clone();
            cached.status = InstanceStatus::Cached;
            cached.error = new_instance.error.take();
            cached
        }
        _ if new_instance.error.is_none() => {
            new_instance.status = InstanceStatus::Healthy;
            new_instance.last_success_at = Some(chrono::Utc::now().to_rfc3339());
            new_instance
        }
        _ => {
            new_instance.status = InstanceStatus::Failed;
            new_instance
        }
    }
}

/// Fetch bounded-concurrency resource summaries for a single client.
pub async fn fetch_resource_summaries(
    client: &SplunkClient,
    resource_types: Vec<String>,
    cancel: Option<&dyn CancellationProbe>,
) -> Result<Vec<ResourceSummary>> {
    ensure_not_cancelled(cancel)?;

    stream::iter(resource_types.into_iter().map(|resource_type| async move {
        ensure_not_cancelled(cancel)?;

        let summary = match resource_type.as_str() {
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
        };

        ensure_not_cancelled(cancel)?;
        Ok::<_, anyhow::Error>(summary)
    }))
    .buffer_unordered(MAX_CONCURRENT_RESOURCE_FETCHES)
    .collect::<Vec<_>>()
    .await
    .into_iter()
    .collect()
}

/// Fetch shared multi-profile overview output.
pub async fn fetch_multi_profile_overview(
    profiles: Vec<(String, ProfileConfig)>,
    resource_types: Vec<String>,
    cancel: Option<&dyn CancellationProbe>,
) -> Result<ListAllMultiOutput> {
    ensure_not_cancelled(cancel)?;

    let results = stream::iter(profiles.into_iter().map(|(profile_name, profile_config)| {
        let resource_types = resource_types.clone();
        async move {
            ensure_not_cancelled(cancel)?;
            fetch_profile_result(profile_name, profile_config, resource_types, cancel).await
        }
    }))
    .buffer_unordered(MAX_CONCURRENT_PROFILE_FETCHES)
    .collect::<Vec<_>>()
    .await;

    let mut profiles: Vec<ProfileResult> = results.into_iter().collect::<Result<Vec<_>>>()?;
    profiles.sort_by(|left, right| left.profile_name.cmp(&right.profile_name));

    Ok(ListAllMultiOutput {
        timestamp: chrono::Utc::now().to_rfc3339(),
        profiles,
    })
}

/// Fetch shared dashboard overview for every configured profile.
pub async fn fetch_multi_instance_overview(
    profiles: Vec<(String, ProfileConfig)>,
    cancel: Option<&dyn CancellationProbe>,
) -> Result<MultiInstanceOverviewData> {
    ensure_not_cancelled(cancel)?;

    let results = stream::iter(profiles.into_iter().map(
        |(profile_name, profile_config)| async move {
            ensure_not_cancelled(cancel)?;
            fetch_instance_overview(profile_name, profile_config, cancel).await
        },
    ))
    .buffer_unordered(MAX_CONCURRENT_PROFILE_FETCHES)
    .collect::<Vec<_>>()
    .await;

    let mut instances: Vec<InstanceOverview> = results.into_iter().collect::<Result<Vec<_>>>()?;
    instances.sort_by(|left, right| left.profile_name.cmp(&right.profile_name));

    Ok(MultiInstanceOverviewData {
        timestamp: chrono::Utc::now().to_rfc3339(),
        instances,
    })
}

/// Fetch a single instance overview for targeted retry flows.
pub async fn fetch_instance_overview(
    profile_name: String,
    profile_config: ProfileConfig,
    cancel: Option<&dyn CancellationProbe>,
) -> Result<InstanceOverview> {
    ensure_not_cancelled(cancel)?;

    let base_url = profile_config.base_url.clone().unwrap_or_default();
    let timeout = profile_timeout(&profile_config);
    let resources_to_fetch: Vec<String> = VALID_RESOURCES
        .iter()
        .map(|resource| resource.to_string())
        .collect();

    let client = match build_client_from_profile(&profile_config) {
        Ok(client) => client,
        Err(error) => {
            return Ok(InstanceOverview {
                profile_name,
                base_url,
                resources: Vec::new(),
                error: Some(error),
                health_status: "error".to_string(),
                job_count: 0,
                status: InstanceStatus::Failed,
                last_success_at: None,
            });
        }
    };

    let resources = fetch_resource_summaries_with_retries(
        &client,
        resources_to_fetch,
        timeout,
        cancel,
        INSTANCE_RETRY_ATTEMPTS,
    )
    .await?;

    let health_status = resources
        .iter()
        .find(|resource| resource.resource_type == "health")
        .map(|resource| resource.status.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let job_count = resources
        .iter()
        .find(|resource| resource.resource_type == "jobs")
        .map(|resource| resource.count)
        .unwrap_or_default();
    let has_errors = resources.iter().any(|resource| resource.error.is_some());
    let error = has_errors.then(|| "One or more resources failed to refresh".to_string());

    Ok(InstanceOverview {
        profile_name,
        base_url,
        resources,
        error,
        health_status,
        job_count,
        status: if has_errors {
            InstanceStatus::Failed
        } else {
            InstanceStatus::Healthy
        },
        last_success_at: (!has_errors).then(|| chrono::Utc::now().to_rfc3339()),
    })
}

/// Clone profiles from a config manager map without borrowing frontend locks across I/O.
pub fn clone_profiles(
    profiles: &HashMap<String, ProfileConfig>,
    profile_names: &[String],
) -> Vec<(String, ProfileConfig)> {
    profile_names
        .iter()
        .map(|profile_name| {
            (
                profile_name.clone(),
                profiles.get(profile_name).cloned().unwrap_or_default(),
            )
        })
        .collect()
}

async fn fetch_profile_result(
    profile_name: String,
    profile_config: ProfileConfig,
    resource_types: Vec<String>,
    cancel: Option<&dyn CancellationProbe>,
) -> Result<ProfileResult> {
    ensure_not_cancelled(cancel)?;

    let base_url = profile_config.base_url.clone().unwrap_or_default();
    let timeout = profile_timeout(&profile_config);

    let client = match build_client_from_profile(&profile_config) {
        Ok(client) => client,
        Err(error) => {
            return Ok(ProfileResult {
                profile_name,
                base_url,
                resources: Vec::new(),
                error: Some(error),
            });
        }
    };

    let resources =
        fetch_resource_summaries_with_retries(&client, resource_types, timeout, cancel, 1).await?;

    Ok(ProfileResult {
        profile_name,
        base_url,
        resources,
        error: None,
    })
}

async fn fetch_resource_summaries_with_retries(
    client: &SplunkClient,
    resource_types: Vec<String>,
    timeout: Duration,
    cancel: Option<&dyn CancellationProbe>,
    max_attempts: usize,
) -> Result<Vec<ResourceSummary>> {
    let mut attempt = 0usize;

    loop {
        ensure_not_cancelled(cancel)?;
        let resources = fetch_resource_summaries(client, resource_types.clone(), cancel).await?;
        let has_success = resources.iter().any(|resource| resource.error.is_none());
        let has_failure = resources.iter().any(|resource| resource.error.is_some());

        if !has_failure || max_attempts <= 1 || attempt + 1 >= max_attempts || !has_success {
            return Ok(resources);
        }

        attempt += 1;
        tokio::time::sleep(backoff_delay(attempt, timeout)).await;
    }
}

fn backoff_delay(attempt: usize, timeout: Duration) -> Duration {
    let multiplier = 1u64 << attempt.saturating_sub(1).min(4);
    let base = Duration::from_millis(INSTANCE_RETRY_BASE_DELAY_MS * multiplier);
    base.min(timeout / 2)
}

fn profile_timeout(profile_config: &ProfileConfig) -> Duration {
    Duration::from_secs(
        profile_config
            .timeout_seconds
            .unwrap_or(DEFAULT_TIMEOUT_SECS),
    )
}

fn build_client_from_profile(
    profile_config: &ProfileConfig,
) -> std::result::Result<SplunkClient, String> {
    let config = config_from_profile(profile_config)?;
    SplunkClient::builder()
        .from_config(&config)
        .build()
        .map_err(|error| format!("Failed to build client: {error}"))
}

fn config_from_profile(profile_config: &ProfileConfig) -> std::result::Result<Config, String> {
    let auth_strategy = build_config_auth_strategy(profile_config)?;

    Ok(Config {
        connection: ConnectionConfig {
            base_url: profile_config.base_url.clone().unwrap_or_default(),
            skip_verify: profile_config.skip_verify.unwrap_or(false),
            timeout: profile_timeout(profile_config),
            max_retries: profile_config.max_retries.unwrap_or(DEFAULT_MAX_RETRIES),
            session_expiry_buffer_seconds: profile_config
                .session_expiry_buffer_seconds
                .unwrap_or(DEFAULT_EXPIRY_BUFFER_SECS),
            session_ttl_seconds: profile_config
                .session_ttl_seconds
                .unwrap_or(DEFAULT_SESSION_TTL_SECS),
            health_check_interval_seconds: profile_config
                .health_check_interval_seconds
                .unwrap_or(DEFAULT_HEALTH_CHECK_INTERVAL_SECS),
            circuit_breaker_enabled: default_circuit_breaker_enabled(),
            circuit_failure_threshold: default_circuit_failure_threshold(),
            circuit_failure_window_seconds: default_circuit_failure_window(),
            circuit_reset_timeout_seconds: default_circuit_reset_timeout(),
            circuit_half_open_requests: default_circuit_half_open_requests(),
        },
        auth: ConfigAuthConfig {
            strategy: auth_strategy,
        },
    })
}

fn build_config_auth_strategy(
    profile_config: &ProfileConfig,
) -> std::result::Result<ConfigAuthStrategy, String> {
    if let Some(token_secure) = &profile_config.api_token {
        return resolve_secure_value(token_secure, "API token")
            .map(|token| ConfigAuthStrategy::ApiToken { token });
    }

    match (&profile_config.username, &profile_config.password) {
        (Some(username), Some(password_secure)) => {
            resolve_secure_value(password_secure, "password").map(|password| {
                ConfigAuthStrategy::SessionToken {
                    username: username.clone(),
                    password,
                }
            })
        }
        (Some(_), None) => Err("Username configured but password is missing".to_string()),
        (None, Some(_)) => Err("Password configured but username is missing".to_string()),
        (None, None) => {
            Err("No credentials configured (expected api_token or username/password)".to_string())
        }
    }
}

fn resolve_secure_value(
    value: &SecureValue,
    label: &str,
) -> std::result::Result<SecretString, String> {
    value
        .resolve()
        .map_err(|error| format!("Failed to resolve {label} from keyring: {error}"))
}

#[allow(dead_code)]
fn build_auth_strategy(
    profile_config: &ProfileConfig,
) -> std::result::Result<AuthStrategy, String> {
    match build_config_auth_strategy(profile_config)? {
        ConfigAuthStrategy::ApiToken { token } => Ok(AuthStrategy::ApiToken { token }),
        ConfigAuthStrategy::SessionToken { username, password } => {
            Ok(AuthStrategy::SessionToken { username, password })
        }
    }
}

async fn fetch_indexes(client: &SplunkClient) -> ResourceSummary {
    fetch_with_timeout(
        "indexes",
        client_timeout(client),
        || client.list_indexes(Some(LIST_LIMIT_1000), None),
        |indexes| indexes.len(),
        |_| "ok".to_string(),
    )
    .await
}

async fn fetch_jobs(client: &SplunkClient) -> ResourceSummary {
    fetch_with_timeout(
        "jobs",
        client_timeout(client),
        || client.list_jobs(Some(LIST_LIMIT_100), None),
        |jobs| jobs.len(),
        |_| "active".to_string(),
    )
    .await
}

async fn fetch_apps(client: &SplunkClient) -> ResourceSummary {
    fetch_with_timeout(
        "apps",
        client_timeout(client),
        || client.list_apps(Some(LIST_LIMIT_1000), None),
        |apps| apps.len(),
        |_| "installed".to_string(),
    )
    .await
}

async fn fetch_users(client: &SplunkClient) -> ResourceSummary {
    fetch_with_timeout(
        "users",
        client_timeout(client),
        || client.list_users(Some(LIST_LIMIT_1000), None),
        |users| users.len(),
        |_| "active".to_string(),
    )
    .await
}

async fn fetch_cluster(client: &SplunkClient) -> ResourceSummary {
    match tokio::time::timeout(client_timeout(client), client.get_cluster_info()).await {
        Ok(Ok(cluster)) => ResourceSummary {
            resource_type: "cluster".to_string(),
            count: 1,
            status: cluster.mode.to_string(),
            error: None,
        },
        Ok(Err(error)) => match error {
            ClientError::ApiError { status: 404, .. } | ClientError::NotFound(_) => {
                ResourceSummary {
                    resource_type: "cluster".to_string(),
                    count: 0,
                    status: "not clustered".to_string(),
                    error: None,
                }
            }
            _ if error.to_string().to_lowercase().contains("cluster") => ResourceSummary {
                resource_type: "cluster".to_string(),
                count: 0,
                status: "not clustered".to_string(),
                error: None,
            },
            _ => resource_error("cluster", error),
        },
        Err(_) => timeout_resource("cluster", "fetch_cluster", client_timeout(client)),
    }
}

async fn fetch_health(client: &SplunkClient) -> ResourceSummary {
    match tokio::time::timeout(client_timeout(client), client.get_health()).await {
        Ok(Ok(health)) => ResourceSummary {
            resource_type: "health".to_string(),
            count: 1,
            status: health.health.to_string(),
            error: None,
        },
        Ok(Err(error)) => resource_error("health", error),
        Err(_) => timeout_resource("health", "fetch_health", client_timeout(client)),
    }
}

async fn fetch_kvstore(client: &SplunkClient) -> ResourceSummary {
    match tokio::time::timeout(client_timeout(client), client.get_kvstore_status()).await {
        Ok(Ok(status)) => ResourceSummary {
            resource_type: "kvstore".to_string(),
            count: 1,
            status: status.current_member.status.to_string(),
            error: None,
        },
        Ok(Err(error)) => resource_error("kvstore", error),
        Err(_) => timeout_resource("kvstore", "fetch_kvstore", client_timeout(client)),
    }
}

async fn fetch_license(client: &SplunkClient) -> ResourceSummary {
    match tokio::time::timeout(client_timeout(client), client.get_license_usage()).await {
        Ok(Ok(usage)) => {
            let total_usage: usize = usage
                .iter()
                .map(|item| item.effective_used_bytes())
                .sum::<usize>()
                / 1024;
            let total_quota: usize = usage.iter().map(|item| item.quota).sum::<usize>() / 1024;
            let status = if total_quota > 0 && total_usage > total_quota * 9 / 10 {
                "warning"
            } else if total_quota > 0 {
                "ok"
            } else {
                "unavailable"
            };

            ResourceSummary {
                resource_type: "license".to_string(),
                count: usage.len(),
                status: status.to_string(),
                error: None,
            }
        }
        Ok(Err(error)) => resource_error("license", error),
        Err(_) => timeout_resource("license", "fetch_license", client_timeout(client)),
    }
}

async fn fetch_saved_searches(client: &SplunkClient) -> ResourceSummary {
    fetch_with_timeout(
        "saved-searches",
        client_timeout(client),
        || client.list_saved_searches(Some(LIST_LIMIT_1000), None),
        |searches| searches.len(),
        |_| "ok".to_string(),
    )
    .await
}

async fn fetch_with_timeout<T, F, E>(
    resource_type: &str,
    timeout: Duration,
    fetch_fn: impl FnOnce() -> F,
    extract_count: impl FnOnce(&T) -> usize,
    extract_status: impl FnOnce(&T) -> String,
) -> ResourceSummary
where
    F: std::future::Future<Output = std::result::Result<T, E>>,
    E: std::fmt::Display,
{
    match tokio::time::timeout(timeout, fetch_fn()).await {
        Ok(Ok(response)) => ResourceSummary {
            resource_type: resource_type.to_string(),
            count: extract_count(&response),
            status: extract_status(&response),
            error: None,
        },
        Ok(Err(error)) => ResourceSummary {
            resource_type: resource_type.to_string(),
            count: 0,
            status: "error".to_string(),
            error: Some(error.to_string()),
        },
        Err(_) => timeout_resource(resource_type, "fetch_resource", timeout),
    }
}

fn client_timeout(client: &SplunkClient) -> Duration {
    client.request_timeout
}

fn timeout_resource(
    resource_type: &str,
    operation: &'static str,
    timeout: Duration,
) -> ResourceSummary {
    resource_error(
        resource_type,
        ClientError::OperationTimeout { operation, timeout },
    )
}

fn resource_error(resource_type: &str, error: ClientError) -> ResourceSummary {
    ResourceSummary {
        resource_type: resource_type.to_string(),
        count: 0,
        status: "error".to_string(),
        error: Some(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::load_fixture;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[derive(Debug)]
    struct TestCancel(bool);

    impl CancellationProbe for TestCancel {
        fn is_cancelled(&self) -> bool {
            self.0
        }
    }

    fn session_profile() -> ProfileConfig {
        ProfileConfig {
            base_url: Some("https://splunk.example.com:8089".to_string()),
            username: Some("admin".to_string()),
            password: Some(SecureValue::Plain(SecretString::new("password".into()))),
            api_token: None,
            skip_verify: Some(true),
            timeout_seconds: Some(45),
            max_retries: Some(7),
            session_expiry_buffer_seconds: Some(90),
            session_ttl_seconds: Some(7200),
            health_check_interval_seconds: Some(30),
        }
    }

    #[test]
    fn normalizes_and_deduplicates_resources() {
        let resources = normalize_and_validate_resources(Some(vec![
            " indexes ".to_string(),
            "jobs".to_string(),
            "indexes".to_string(),
        ]))
        .unwrap();

        assert_eq!(resources, vec!["indexes".to_string(), "jobs".to_string()]);
    }

    #[test]
    fn rejects_unknown_resources() {
        let error =
            normalize_and_validate_resources(Some(vec!["unknown".to_string()])).unwrap_err();
        assert!(error.to_string().contains("Invalid resource type"));
    }

    #[test]
    fn merge_preserves_cached_data_on_failure() {
        let existing = InstanceOverview {
            profile_name: "prod".to_string(),
            base_url: "https://prod".to_string(),
            resources: vec![ResourceSummary {
                resource_type: "jobs".to_string(),
                count: 4,
                status: "active".to_string(),
                error: None,
            }],
            error: None,
            health_status: "green".to_string(),
            job_count: 4,
            status: InstanceStatus::Healthy,
            last_success_at: Some("2026-03-11T10:00:00Z".to_string()),
        };
        let incoming = InstanceOverview {
            profile_name: "prod".to_string(),
            base_url: "https://prod".to_string(),
            resources: Vec::new(),
            error: Some("boom".to_string()),
            health_status: "error".to_string(),
            job_count: 0,
            status: InstanceStatus::Failed,
            last_success_at: None,
        };

        let merged = merge_instance_update(Some(&existing), incoming);

        assert_eq!(merged.status, InstanceStatus::Cached);
        assert_eq!(merged.job_count, 4);
        assert_eq!(merged.error.as_deref(), Some("boom"));
    }

    #[test]
    fn clone_profiles_returns_requested_entries() {
        let mut profiles = HashMap::new();
        profiles.insert("prod".to_string(), session_profile());

        let cloned = clone_profiles(&profiles, &["prod".to_string()]);
        assert_eq!(cloned.len(), 1);
        assert_eq!(cloned[0].0, "prod");
        assert_eq!(
            cloned[0].1.base_url.as_deref(),
            Some("https://splunk.example.com:8089")
        );
    }

    #[test]
    fn config_from_profile_preserves_profile_connection_settings() {
        let config = config_from_profile(&session_profile()).unwrap();

        assert_eq!(config.connection.max_retries, 7);
        assert_eq!(config.connection.timeout, Duration::from_secs(45));
        assert_eq!(config.connection.session_ttl_seconds, 7200);
        assert_eq!(config.connection.session_expiry_buffer_seconds, 90);
        assert!(matches!(
            config.auth.strategy,
            ConfigAuthStrategy::SessionToken { .. }
        ));
    }

    #[tokio::test]
    async fn multi_profile_fetch_stops_when_cancelled() {
        let cancel = TestCancel(true);
        let error = fetch_multi_profile_overview(
            vec![("prod".to_string(), session_profile())],
            vec!["jobs".to_string()],
            Some(&cancel),
        )
        .await
        .unwrap_err();

        assert!(error.to_string().contains("workflow cancelled"));
    }

    #[tokio::test]
    async fn fetch_instance_overview_stops_when_cancelled() {
        let cancel = TestCancel(true);
        let error = fetch_instance_overview("prod".to_string(), session_profile(), Some(&cancel))
            .await
            .unwrap_err();

        assert!(error.to_string().contains("workflow cancelled"));
    }

    #[tokio::test]
    async fn fetch_cluster_treats_404_as_not_clustered() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/services/cluster/master/config"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "messages": [{"type": "ERROR", "text": "Not clustered"}]
            })))
            .mount(&mock_server)
            .await;

        let client = SplunkClient::builder()
            .base_url(mock_server.uri())
            .auth_strategy(AuthStrategy::ApiToken {
                token: SecretString::new("test-token".to_string().into()),
            })
            .skip_verify(true)
            .build()
            .expect("client should build");

        let summary = fetch_cluster(&client).await;

        assert_eq!(summary.resource_type, "cluster");
        assert_eq!(summary.status, "not clustered");
        assert!(summary.error.is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn fetch_with_timeout_returns_timeout_resource() {
        let handle = tokio::spawn(async {
            fetch_with_timeout(
                "jobs",
                Duration::from_millis(50),
                || async {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    Ok::<Vec<String>, anyhow::Error>(vec!["never".to_string()])
                },
                |rows| rows.len(),
                |_| "ok".to_string(),
            )
            .await
        });

        tokio::time::advance(Duration::from_secs(5)).await;
        let summary = handle.await.expect("task should complete");

        assert_eq!(summary.resource_type, "jobs");
        assert_eq!(summary.status, "error");
        assert!(
            summary
                .error
                .as_deref()
                .is_some_and(|error| error.contains("timed out"))
        );
    }

    #[tokio::test]
    async fn fetch_instance_overview_retries_targeted_profile_only() {
        let mock_server = MockServer::start().await;
        let cluster_calls = Arc::new(AtomicUsize::new(0));
        let cluster_calls_clone = cluster_calls.clone();

        Mock::given(method("GET"))
            .and(path("/services/data/indexes"))
            .and(query_param("output_mode", "json"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(load_fixture("indexes/list_indexes.json")),
            )
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/services/search/jobs"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(load_fixture("jobs/list_jobs.json")),
            )
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/services/apps/local"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(load_fixture("apps/list_apps.json")),
            )
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/services/authentication/users"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(load_fixture("users/list_users.json")),
            )
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/services/cluster/master/config"))
            .respond_with(move |_request: &wiremock::Request| {
                let attempt = cluster_calls_clone.fetch_add(1, Ordering::SeqCst);
                if attempt == 0 {
                    ResponseTemplate::new(503).set_body_json(serde_json::json!({
                        "messages": [{"type": "ERROR", "text": "cluster unavailable"}]
                    }))
                } else {
                    ResponseTemplate::new(200)
                        .set_body_json(load_fixture("cluster/get_cluster_info.json"))
                }
            })
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/services/server/health/splunkd"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(load_fixture("server/get_health.json")),
            )
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/services/kvstore/status"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(load_fixture("kvstore/status.json")),
            )
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/services/licenser/usage"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(load_fixture("license/get_usage.json")),
            )
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/services/saved/searches"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(load_fixture("search/list_saved_searches.json")),
            )
            .mount(&mock_server)
            .await;

        let mut profile = session_profile();
        profile.base_url = Some(mock_server.uri());
        profile.api_token = Some(SecureValue::Plain(SecretString::new(
            "test-token".to_string().into(),
        )));
        profile.username = None;
        profile.password = None;
        profile.timeout_seconds = Some(2);

        let overview = fetch_instance_overview("prod".to_string(), profile, None)
            .await
            .expect("instance overview should succeed");

        assert_eq!(overview.profile_name, "prod");
        assert_eq!(overview.status, InstanceStatus::Healthy);
        assert_eq!(overview.health_status, "green");
        assert_eq!(overview.job_count, 2);
        assert!(overview.error.is_none());
        assert!(overview.last_success_at.is_some());
        assert_eq!(cluster_calls.load(Ordering::SeqCst), 2);
    }
}
