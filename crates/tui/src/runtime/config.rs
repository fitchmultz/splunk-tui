//! Configuration loading and persistence for the TUI.
//!
//! Responsibilities:
//! - Load configuration with CLI and environment variable overrides.
//! - Apply search defaults with environment variable overrides.
//! - Save persisted state on application exit.
//!
//! Does NOT handle:
//! - Creating the Splunk client (see `runtime::client`).
//! - Terminal state management (see `runtime::terminal`).
//! - Async API calls (see `runtime::side_effects`).
//!
//! Invariants:
//! - Configuration precedence: CLI args > env vars > profile config > defaults.
//! - `load_dotenv()` is called before loading configuration.
//! - ConfigManager is wrapped in Arc<Mutex<>> for thread-safe access.

use crate::app::App;
use anyhow::{Result, anyhow};
use splunk_config::{
    Config, ConfigLoader, ConfigManager, InternalLogsDefaults, SearchDefaultConfig,
};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::cli::Cli;

/// Load configuration, search defaults, and internal logs defaults from CLI args, environment variables, and profile.
///
/// This function returns the main Config along with SearchDefaultConfig and InternalLogsDefaults so that
/// defaults with environment variable overrides can be applied to the App state.
///
/// Configuration precedence (highest to lowest):
/// 1. CLI arguments (e.g., --profile, --config-path)
/// 2. Environment variables (e.g., SPLUNK_PROFILE, SPLUNK_BASE_URL)
/// 3. Profile configuration (from config.json)
/// 4. Default values
///
/// # Arguments
///
/// * `cli` - The parsed CLI arguments
///
/// # Errors
///
/// Returns an error if configuration loading fails (e.g., profile not found,
/// missing required fields like base_url or auth credentials).
/// Load configuration, search defaults, and internal logs defaults from CLI args, environment variables, and profile.
///
/// This function returns the main Config along with SearchDefaultConfig and InternalLogsDefaults so that
/// defaults with environment variable overrides can be applied to the App state.
///
/// Configuration precedence (highest to lowest):
/// 1. CLI arguments (e.g., --profile, --config-path)
/// 2. Environment variables (e.g., SPLUNK_PROFILE, SPLUNK_BASE_URL)
/// 3. Profile configuration (from config.json)
/// 4. Default values
///
/// # Arguments
///
/// * `cli` - The parsed CLI arguments
///
/// # Returns
///
/// Returns a tuple of (SearchDefaultConfig, InternalLogsDefaults, Config, Option<String>)
/// where the Option<String> is the resolved profile name (from CLI or env var).
///
/// # Errors
///
/// Returns an error if configuration loading fails (e.g., profile not found,
/// missing required fields like base_url or auth credentials).
pub fn load_config_with_defaults(
    cli: &Cli,
) -> Result<(
    SearchDefaultConfig,
    InternalLogsDefaults,
    Config,
    Option<String>,
)> {
    let mut loader = ConfigLoader::new().load_dotenv()?;

    // Apply config path from CLI if provided (highest precedence)
    if let Some(config_path) = &cli.config_path {
        loader = loader.with_config_path(config_path.clone());
    }

    // Apply profile from CLI if provided (highest precedence)
    if let Some(profile) = &cli.profile {
        loader = loader.with_profile_name(profile.clone());
    }

    // Apply environment variables (including SPLUNK_CONFIG_PATH and SPLUNK_PROFILE
    // if not already set via CLI args). Env vars override profile values.
    let mut loader = loader.from_env()?;

    // Load from profile if profile_name is now set (from CLI or env var)
    if loader.profile_name().is_some() {
        loader = loader.from_profile()?;
    }

    // Capture the resolved profile name before building
    let resolved_profile_name = loader.profile_name().cloned();

    // Build search defaults and internal logs defaults with env var overrides
    // (pass None for now, will merge with persisted later)
    let search_defaults = loader.build_search_defaults(None);
    let internal_logs_defaults = loader.build_internal_logs_defaults(None);

    let config = loader
        .build()
        .map_err(|e| anyhow!("Failed to load config: {}", e))?;

    Ok((
        search_defaults,
        internal_logs_defaults,
        config,
        resolved_profile_name,
    ))
}

/// Save persisted state and prepare to quit.
///
/// This function should be called before exiting the event loop to ensure
/// user preferences and UI state are persisted to disk.
///
/// # Arguments
///
/// * `app` - The application state containing persisted settings
/// * `config_manager` - The configuration manager for saving state
///
/// # Errors
///
/// Returns an error if saving the persisted state fails.
pub async fn save_and_quit(app: &App, config_manager: &Arc<Mutex<ConfigManager>>) -> Result<()> {
    let state = app.get_persisted_state();
    let mut cm = config_manager.lock().await;
    cm.save(&state)?;
    Ok(())
}
