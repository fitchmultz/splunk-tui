//! Splunk CLI - Command-line interface for Splunk Enterprise.
//!
//! Responsibilities:
//! - Parse command-line arguments and environment variables.
//! - Execute Splunk REST API commands via the shared client library.
//! - Format and display results in various output formats (table, JSON, etc.).
//!
//! Does NOT handle:
//! - Core business logic or REST API implementation (see `crates/client`).
//! - Long-term persistence of search results.
//!
//! Invariants / Assumptions:
//! - `load_dotenv()` is called BEFORE CLI parsing to allow `.env` to provide clap defaults.
//! - Global options (like `--base-url`) are applied consistently across all subcommands.

mod args;
mod cancellation;
mod commands;
mod config_context;
mod dispatch;
mod formatters;
mod progress;

use anyhow::Result;
use args::{Cli, resolve_config_path};
use cancellation::{
    CancellationToken, SIGINT_EXIT_CODE, is_cancelled_error, print_cancelled_message,
};
use clap::Parser;
use config_context::ConfigCommandContext;
use dispatch::run_command;
use splunk_config::ConfigLoader;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file BEFORE CLI parsing so clap env defaults can read .env values
    ConfigLoader::new().load_dotenv()?;

    let mut cli = Cli::parse();

    // Resolve config path immediately after parsing to ensure blank values are ignored.
    // This handles both blank env vars and blank CLI flags consistently.
    cli.config_path = resolve_config_path(cli.config_path.take());

    // Initialize logging
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(fmt::layer())
        .init();

    // Determine if we need a real config or can use a placeholder
    // Config commands, multi-profile list-all, and HEC commands don't need standard connection details
    let is_multi_profile_list_all = matches!(
        cli.command,
        args::Commands::ListAll {
            all_profiles: true,
            ..
        } | args::Commands::ListAll {
            profiles: Some(_),
            ..
        }
    );
    let needs_real_config = !matches!(
        cli.command,
        args::Commands::Config { .. } | args::Commands::Hec { .. }
    ) && !is_multi_profile_list_all;

    // Build configuration only if needed
    let config = if needs_real_config {
        let mut loader = ConfigLoader::new();

        // Apply custom config path if provided (highest priority for loader setup)
        if let Some(ref path) = cli.config_path {
            loader = loader.with_config_path(path.clone());
        }

        // Load from profile if specified (lowest priority)
        if let Some(ref profile_name) = cli.profile {
            loader = loader
                .with_profile_name(profile_name.clone())
                .from_profile()?;
        }

        // Apply environment variable overrides (medium priority)
        loader = loader.from_env()?;

        // Apply CLI overrides (highest priority)
        if let Some(ref url) = cli.base_url {
            loader = loader.with_base_url(url.clone());
        }
        if let Some(ref username) = cli.username {
            loader = loader.with_username(username.clone());
        }
        if let Some(ref password) = cli.password {
            loader = loader.with_password(password.clone());
        }
        if let Some(ref token) = cli.api_token {
            loader = loader.with_api_token(token.clone());
        }
        if let Some(timeout_secs) = cli.timeout {
            loader = loader.with_timeout(std::time::Duration::from_secs(timeout_secs));
        }
        if let Some(retries) = cli.max_retries {
            loader = loader.with_max_retries(retries);
        }
        if cli.skip_verify {
            loader = loader.with_skip_verify(true);
        }

        // Build search defaults with env var overrides (matching TUI behavior)
        // Must be done before loader.build() since build() consumes the loader
        let search_defaults = loader.build_search_defaults(None);

        let config = loader.build()?;

        // Warn if using default credentials (security check)
        if config.is_using_default_credentials() {
            tracing::warn!(
                "Using default Splunk credentials (admin/changeme). \
                 These are for local development only - change before production use."
            );
        }

        Some((config, search_defaults))
    } else {
        None
    };

    // Create cancellation token and set up signal handling
    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();

    tokio::spawn(async move {
        if let Err(e) = tokio::signal::ctrl_c().await {
            eprintln!("Failed to listen for Ctrl+C: {}", e);
            return;
        }
        cancel_clone.cancel();
    });

    // Wrap config in appropriate context based on command type
    let config_context = if let Some((config, search_defaults)) = config {
        ConfigCommandContext::Real(config, search_defaults)
    } else {
        ConfigCommandContext::Placeholder
    };

    // Execute command
    match run_command(cli, config_context, &cancel).await {
        Ok(()) => Ok(()),
        Err(e) if is_cancelled_error(&e) => {
            print_cancelled_message();
            std::process::exit(SIGINT_EXIT_CODE as i32);
        }
        Err(e) => Err(e),
    }
}
