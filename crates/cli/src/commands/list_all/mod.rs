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

use anyhow::{Context, Result};
use tracing::info;

use crate::cancellation::CancellationToken;
use crate::commands::build_client_from_config;
use crate::formatters::{OutputFormat, write_to_file};

pub use types::{ListAllMultiOutput, ListAllOutput, ProfileResult, VALID_RESOURCES};

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
    cancel: &CancellationToken,
) -> Result<()> {
    info!("Listing all Splunk resources");

    // Normalize and validate resource types (trim, lowercase, dedupe, preserve order)
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

        output::fetch_multi_profile_resources(cm, target_profiles, resources_to_fetch, cancel)
            .await?
    } else {
        // Single-profile mode (backward compatible)
        let mut client = build_client_from_config(&config)?;

        let resources =
            fetchers::fetch_all_resources(&mut client, resources_to_fetch, cancel).await?;

        ListAllMultiOutput {
            timestamp: output::format_timestamp(),
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
    let formatted = output::format_multi_profile_output(&results, format)?;

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
