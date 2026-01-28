//! List-all command implementation with multi-profile aggregation.
//!
//! Responsibilities:
//! - Fetch resource summaries from single or multiple Splunk profiles.
//! - Aggregate results across profiles for distributed visibility.
//! - Handle per-profile errors gracefully without failing the entire command.
//!
//! Does NOT handle:
//! - Direct REST API implementation (see `crates/client`).
//! - Output formatting (see `crate::formatters`).
//!
//! Invariants / Assumptions:
//! - Individual resource fetches have a 30-second timeout.
//! - Profile-level errors are captured and reported but don't stop other profiles.
//! - Timestamp is always RFC3339 format.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use splunk_client::SplunkClient;
use std::collections::HashSet;
use std::time::Duration;
use tokio::time;
use tracing::{info, warn};

use crate::cancellation::Cancelled;
use crate::commands::convert_auth_strategy;
use crate::formatters::{OutputFormat, write_to_file};

/// Per-resource summary for a single resource type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSummary {
    pub resource_type: String,
    pub count: u64,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Single-profile list-all output structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ListAllOutput {
    pub timestamp: String,
    pub resources: Vec<ResourceSummary>,
}

/// Per-profile resource summary for multi-profile aggregation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileResult {
    pub profile_name: String,
    pub base_url: String,
    pub resources: Vec<ResourceSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Multi-profile list-all output structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListAllMultiOutput {
    pub timestamp: String,
    pub profiles: Vec<ProfileResult>,
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

/// Returns the current timestamp in RFC3339 format.
fn format_timestamp() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Main entry point for the list-all command.
///
/// Supports single-profile and multi-profile modes:
/// - Single-profile: Uses the provided config directly (backward compatible)
/// - Multi-profile: Uses ConfigManager to enumerate and query multiple profiles
#[allow(clippy::too_many_arguments)]
pub async fn run(
    config: splunk_config::Config,
    resources_filter: Option<Vec<String>>,
    profile_names: Option<Vec<String>>,
    all_profiles: bool,
    config_manager: Option<splunk_config::ConfigManager>,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Listing all Splunk resources");

    // Normalize and validate resource types (trim, lowercase, dedupe)
    let resources_to_fetch: Vec<String> = resources_filter
        .map(|resources| {
            resources
                .into_iter()
                .map(|r| r.trim().to_lowercase())
                .filter(|r| !r.is_empty())
                .collect::<HashSet<_>>()
                .into_iter()
                .collect()
        })
        .unwrap_or_else(|| VALID_RESOURCES.iter().map(|s| s.to_string()).collect());

    for resource in &resources_to_fetch {
        if !VALID_RESOURCES.contains(&resource.as_str()) {
            anyhow::bail!(
                "Invalid resource type: {}. Valid types: {}",
                resource,
                VALID_RESOURCES.join(", ")
            );
        }
    }

    // Determine which profiles to query
    let is_multi_profile = all_profiles || profile_names.is_some();

    let results = if is_multi_profile {
        // Multi-profile mode
        let cm = config_manager.context("ConfigManager required for multi-profile mode")?;

        let target_profiles: Vec<String> = if all_profiles {
            // Query all profiles from config file
            cm.list_profiles().keys().cloned().collect()
        } else {
            // Query specified profiles - trim and dedupe (preserve case)
            profile_names
                .map(|profiles| {
                    profiles
                        .into_iter()
                        .map(|p| p.trim().to_string())
                        .filter(|p| !p.is_empty())
                        .collect::<HashSet<_>>()
                        .into_iter()
                        .collect()
                })
                .unwrap_or_default()
        };

        if target_profiles.is_empty() {
            anyhow::bail!(
                "No profiles configured. Use 'splunk-cli config set <profile>' to add one."
            );
        }

        // Validate that specified profiles exist
        let available_profiles = cm.list_profiles();
        for profile_name in &target_profiles {
            if !available_profiles.contains_key(profile_name) {
                anyhow::bail!(
                    "Profile '{}' not found. Available profiles: {}",
                    profile_name,
                    available_profiles
                        .keys()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
        }

        fetch_multi_profile_resources(cm, target_profiles, resources_to_fetch, cancel).await?
    } else {
        // Single-profile mode (backward compatible)
        let auth_strategy = convert_auth_strategy(&config.auth.strategy);

        let mut client = SplunkClient::builder()
            .base_url(config.connection.base_url.clone())
            .auth_strategy(auth_strategy)
            .skip_verify(config.connection.skip_verify)
            .timeout(config.connection.timeout)
            .session_ttl_seconds(config.connection.session_ttl_seconds)
            .session_expiry_buffer_seconds(config.connection.session_expiry_buffer_seconds)
            .build()?;

        let resources = fetch_all_resources(&mut client, resources_to_fetch, cancel).await?;

        ListAllMultiOutput {
            timestamp: format_timestamp(),
            profiles: vec![ProfileResult {
                profile_name: "default".to_string(),
                base_url: config.connection.base_url,
                resources,
                error: None,
            }],
        }
    };

    // Format and output results
    let format = OutputFormat::from_str(output_format)?;
    let formatted = format_multi_profile_output(&results, format)?;

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

/// Format multi-profile output based on the selected format.
fn format_multi_profile_output(
    output: &ListAllMultiOutput,
    format: OutputFormat,
) -> Result<String> {
    match format {
        OutputFormat::Json => Ok(serde_json::to_string_pretty(output)?),
        OutputFormat::Table => format_multi_profile_table(output),
        OutputFormat::Csv => format_multi_profile_csv(output),
        OutputFormat::Xml => format_multi_profile_xml(output),
    }
}

/// Format multi-profile output as a table.
fn format_multi_profile_table(output: &ListAllMultiOutput) -> Result<String> {
    let mut out = String::new();

    out.push_str(&format!("Timestamp: {}\n", output.timestamp));
    out.push('\n');

    if output.profiles.is_empty() {
        out.push_str("No profiles found.\n");
        return Ok(out);
    }

    for profile in &output.profiles {
        out.push_str(&format!(
            "=== Profile: {} ({}) ===\n",
            profile.profile_name, profile.base_url
        ));

        if let Some(ref error) = profile.error {
            out.push_str(&format!("Error: {}\n", error));
            out.push('\n');
            continue;
        }

        if profile.resources.is_empty() {
            out.push_str("No resources found.\n");
        } else {
            let header = format!(
                "{:<20} {:<10} {:<15} {}",
                "Resource Type", "Count", "Status", "Error"
            );
            out.push_str(&header);
            out.push('\n');

            let separator = format!("{:<20} {:<10} {:<15} {}", "====", "=====", "=====", "=====");
            out.push_str(&separator);
            out.push('\n');

            for resource in &profile.resources {
                let error = resource.error.as_deref().unwrap_or("");
                out.push_str(&format!(
                    "{:<20} {:<10} {:<15} {}\n",
                    resource.resource_type, resource.count, resource.status, error
                ));
            }
        }
        out.push('\n');
    }

    Ok(out)
}

/// Format multi-profile output as CSV.
fn format_multi_profile_csv(output: &ListAllMultiOutput) -> Result<String> {
    let mut csv = String::new();

    csv.push_str("profile_name,base_url,timestamp,resource_type,count,status,error\n");

    for profile in &output.profiles {
        if let Some(ref error) = profile.error {
            // Profile-level error
            csv.push_str(&format!(
                "{},{},{},,,,{}\n",
                escape_csv(&profile.profile_name),
                escape_csv(&profile.base_url),
                escape_csv(&output.timestamp),
                escape_csv(error)
            ));
        } else {
            for resource in &profile.resources {
                let error = resource.error.as_deref().unwrap_or("");
                csv.push_str(&format!(
                    "{},{},{},{},{},{},{}\n",
                    escape_csv(&profile.profile_name),
                    escape_csv(&profile.base_url),
                    escape_csv(&output.timestamp),
                    escape_csv(&resource.resource_type),
                    resource.count,
                    escape_csv(&resource.status),
                    escape_csv(error)
                ));
            }
        }
    }

    Ok(csv)
}

/// Escape a string for CSV output.
fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Format multi-profile output as XML.
fn format_multi_profile_xml(output: &ListAllMultiOutput) -> Result<String> {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<list_all_multi>\n");
    xml.push_str(&format!(
        "  <timestamp>{}</timestamp>\n",
        escape_xml(&output.timestamp)
    ));
    xml.push_str("  <profiles>\n");

    for profile in &output.profiles {
        xml.push_str("    <profile>\n");
        xml.push_str(&format!(
            "      <name>{}</name>\n",
            escape_xml(&profile.profile_name)
        ));
        xml.push_str(&format!(
            "      <base_url>{}</base_url>\n",
            escape_xml(&profile.base_url)
        ));

        if let Some(ref error) = profile.error {
            xml.push_str(&format!("      <error>{}</error>\n", escape_xml(error)));
        } else {
            xml.push_str("      <resources>\n");
            for resource in &profile.resources {
                xml.push_str("        <resource>\n");
                xml.push_str(&format!(
                    "          <type>{}</type>\n",
                    escape_xml(&resource.resource_type)
                ));
                xml.push_str(&format!("          <count>{}</count>\n", resource.count));
                xml.push_str(&format!(
                    "          <status>{}</status>\n",
                    escape_xml(&resource.status)
                ));
                if let Some(ref error) = resource.error {
                    xml.push_str(&format!("          <error>{}</error>\n", escape_xml(error)));
                }
                xml.push_str("        </resource>\n");
            }
            xml.push_str("      </resources>\n");
        }

        xml.push_str("    </profile>\n");
    }

    xml.push_str("  </profiles>\n");
    xml.push_str("</list_all_multi>");
    Ok(xml)
}

/// Escape special XML characters.
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Fetch resources from multiple profiles in parallel.
async fn fetch_multi_profile_resources(
    config_manager: splunk_config::ConfigManager,
    profile_names: Vec<String>,
    resource_types: Vec<String>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<ListAllMultiOutput> {
    let timestamp = format_timestamp();
    let mut profile_results = Vec::new();

    // Get all profile configs first
    let profiles_map = config_manager.list_profiles().clone();

    // Create futures for each profile
    let mut futures = Vec::new();
    for profile_name in &profile_names {
        let profile_config = profiles_map.get(profile_name).cloned();
        let resource_types = resource_types.clone();
        let profile_name = profile_name.clone();

        let future = async move {
            if let Some(config) = profile_config {
                fetch_single_profile_resources(profile_name, config, resource_types, cancel).await
            } else {
                ProfileResult {
                    profile_name,
                    base_url: String::new(),
                    resources: vec![],
                    error: Some("Profile configuration not found".to_string()),
                }
            }
        };
        futures.push(future);
    }

    // Execute all futures concurrently
    let results = futures::future::join_all(futures).await;
    profile_results.extend(results);

    Ok(ListAllMultiOutput {
        timestamp,
        profiles: profile_results,
    })
}

/// Fetch resources from a single profile.
async fn fetch_single_profile_resources(
    profile_name: String,
    profile_config: splunk_config::types::ProfileConfig,
    resource_types: Vec<String>,
    cancel: &crate::cancellation::CancellationToken,
) -> ProfileResult {
    // Build config from profile
    let base_url = profile_config.base_url.clone().unwrap_or_default();

    // Build auth strategy - fail fast if credentials are missing/invalid
    let auth_strategy = match build_auth_strategy_from_profile(&profile_config) {
        Ok(strategy) => strategy,
        Err(error_msg) => {
            return ProfileResult {
                profile_name,
                base_url,
                resources: vec![],
                error: Some(error_msg),
            };
        }
    };

    // Build Splunk client
    let mut client = match SplunkClient::builder()
        .base_url(base_url.clone())
        .auth_strategy(auth_strategy)
        .skip_verify(profile_config.skip_verify.unwrap_or(false))
        .timeout(Duration::from_secs(
            profile_config.timeout_seconds.unwrap_or(30),
        ))
        .session_ttl_seconds(profile_config.session_ttl_seconds.unwrap_or(3600))
        .session_expiry_buffer_seconds(profile_config.session_expiry_buffer_seconds.unwrap_or(60))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return ProfileResult {
                profile_name,
                base_url,
                resources: vec![],
                error: Some(format!("Failed to build client: {}", e)),
            };
        }
    };

    // Fetch resources
    match fetch_all_resources(&mut client, resource_types, cancel).await {
        Ok(resources) => ProfileResult {
            profile_name,
            base_url,
            resources,
            error: None,
        },
        Err(e) => ProfileResult {
            profile_name,
            base_url,
            resources: vec![],
            error: Some(e.to_string()),
        },
    }
}

/// Build authentication strategy from profile configuration.
///
/// Returns `Ok(AuthStrategy)` when credentials are successfully resolved,
/// or `Err(String)` with a descriptive error message when credentials are
/// missing or fail to resolve.
fn build_auth_strategy_from_profile(
    profile: &splunk_config::types::ProfileConfig,
) -> Result<splunk_client::AuthStrategy, String> {
    use secrecy::{ExposeSecret, SecretString};
    use splunk_client::AuthStrategy;

    // Prefer API token if available
    if let Some(ref token) = profile.api_token {
        match token.resolve() {
            Ok(resolved) => {
                return Ok(AuthStrategy::ApiToken {
                    token: SecretString::from(resolved.expose_secret()),
                });
            }
            Err(e) => {
                return Err(format!("Failed to resolve API token from keyring: {}", e));
            }
        }
    }

    // Check for partial username/password configuration
    match (&profile.username, &profile.password) {
        (Some(username), Some(password)) => match password.resolve() {
            Ok(resolved) => Ok(AuthStrategy::SessionToken {
                username: username.clone(),
                password: SecretString::from(resolved.expose_secret()),
            }),
            Err(e) => Err(format!("Failed to resolve password from keyring: {}", e)),
        },
        (Some(_), None) => Err("Username configured but password is missing".to_string()),
        (None, Some(_)) => Err("Password configured but username is missing".to_string()),
        (None, None) => {
            Err("No credentials configured (expected api_token or username/password)".to_string())
        }
    }
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
