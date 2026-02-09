//! List-all command implementation with multi-profile aggregation.
//!
//! Responsibilities:
//! - Fetch resource summaries from single or multiple Splunk profiles.
//! - Aggregate results across profiles for distributed visibility.
//! - Handle per-profile errors gracefully without failing the entire command.
//!
//! Does NOT handle:
//! - Direct REST API implementation (see `crates/client`).
//! - Output formatting details (see `output.rs`).
//! - Resource fetching implementation (see `fetchers.rs`).
//!
//! Invariants:
//! - Individual resource fetches have a 30-second timeout.
//! - Profile-level errors are captured and reported but don't stop other profiles.
//! - Timestamp is always RFC3339 format.

pub mod auth;
pub mod fetchers;
pub mod output;
pub mod types;

use anyhow::Result;
use tracing::info;

use crate::cancellation::CancellationToken;
use crate::commands::build_client_from_config;
use crate::formatters::{OutputFormat, output_result};

pub use types::{ListAllMultiOutput, ProfileResult, VALID_RESOURCES};

/// Normalize and validate resource types for fetching.
///
/// This helper:
/// - Trims, lowercases, and deduplicates resource names (preserving order)
/// - Defaults to all valid resources if none specified
/// - Validates that all resources are valid types
fn normalize_and_validate_resources(resources_filter: Option<Vec<String>>) -> Result<Vec<String>> {
    // Normalize resource types (trim, lowercase, dedupe, preserve order)
    let resources_to_fetch: Vec<String> = resources_filter
        .map(|resources| {
            let mut seen = std::collections::HashSet::new();
            resources
                .into_iter()
                .map(|r| r.trim().to_lowercase())
                .filter(|r| !r.is_empty())
                .filter(|r| seen.insert(r.clone()))
                .collect()
        })
        .unwrap_or_else(|| VALID_RESOURCES.iter().map(|s| s.to_string()).collect());

    // Validate all resources are valid types
    for resource in &resources_to_fetch {
        if !VALID_RESOURCES.contains(&resource.as_str()) {
            anyhow::bail!(
                "Invalid resource type: {}. Valid types: {}",
                resource,
                VALID_RESOURCES.join(", ")
            );
        }
    }

    Ok(resources_to_fetch)
}

/// Write formatted output to stdout or file.
async fn write_output(
    formatted: &str,
    output_file: Option<std::path::PathBuf>,
    format: OutputFormat,
) -> Result<()> {
    output_result(formatted, format, output_file.as_ref())?;
    Ok(())
}

/// Run list-all in single-profile mode.
///
/// This entrypoint requires a real config and is used for backward-compatible
/// single-profile operations. The config is used to build a Splunk client
/// and fetch resources directly.
pub async fn run_single_profile(
    config: splunk_config::Config,
    resources_filter: Option<Vec<String>>,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &CancellationToken,
    no_cache: bool,
) -> Result<()> {
    info!("Listing all Splunk resources (single-profile mode)");

    // Normalize and validate resource types
    let resources_to_fetch = normalize_and_validate_resources(resources_filter)?;

    // Build client and fetch resources
    let client = build_client_from_config(&config, Some(no_cache))?;
    let resources = fetchers::fetch_all_resources(&client, resources_to_fetch, cancel).await?;

    // Build output structure
    let results = ListAllMultiOutput {
        timestamp: output::format_timestamp(),
        profiles: vec![ProfileResult {
            profile_name: "default".to_string(),
            base_url: config.connection.base_url,
            resources,
            error: None,
        }],
    };

    // Format and output results
    let format = OutputFormat::from_str(output_format)?;
    let formatted = output::format_multi_profile_output(&results, format)?;
    write_output(&formatted, output_file, format).await?;

    Ok(())
}

/// Run list-all in multi-profile mode.
///
/// This entrypoint requires a ConfigManager and is used for querying multiple
/// profiles. No single Config is needed since each profile's config is loaded
/// from the ConfigManager.
///
/// Either `profile_names` or `all_profiles` must be specified to determine
/// which profiles to query.
pub async fn run_multi_profile(
    config_manager: splunk_config::ConfigManager,
    resources_filter: Option<Vec<String>>,
    profile_names: Option<Vec<String>>,
    all_profiles: bool,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &CancellationToken,
) -> Result<()> {
    info!("Listing all Splunk resources (multi-profile mode)");

    // Normalize and validate resource types
    let resources_to_fetch = normalize_and_validate_resources(resources_filter)?;

    // Determine which profiles to query
    let target_profiles: Vec<String> = if all_profiles {
        // Query all profiles from config file
        config_manager.list_profiles().keys().cloned().collect()
    } else {
        // Query specified profiles - trim and dedupe (preserve case, preserve order)
        profile_names
            .map(|profiles| {
                let mut seen = std::collections::HashSet::new();
                profiles
                    .into_iter()
                    .map(|p| p.trim().to_string())
                    .filter(|p| !p.is_empty())
                    .filter(|p| seen.insert(p.clone()))
                    .collect()
            })
            .unwrap_or_default()
    };

    if target_profiles.is_empty() {
        anyhow::bail!(
            "Failed to list profiles: No profiles configured. Use 'splunk-cli config set <profile>' to add one."
        );
    }

    // Validate that specified profiles exist
    let available_profiles = config_manager.list_profiles();
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

    // Fetch resources from all target profiles
    let results = output::fetch_multi_profile_resources(
        config_manager,
        target_profiles,
        resources_to_fetch,
        cancel,
    )
    .await?;

    // Format and output results
    let format = OutputFormat::from_str(output_format)?;
    let formatted = output::format_multi_profile_output(&results, format)?;
    write_output(&formatted, output_file, format).await?;

    Ok(())
}
