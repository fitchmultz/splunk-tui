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
//! Invariants:
//! - `load_dotenv()` is called BEFORE CLI parsing to allow `.env` to provide clap defaults.
//! - Global options (like `--base-url`) are applied consistently across all subcommands.

mod args;
#[macro_use]
mod cancellation;
mod commands;
mod config_context;
mod dispatch;
mod dynamic_complete;
mod error;
mod formatters;
mod interactive;
mod progress;

use args::Cli;
use cancellation::{CancellationToken, is_cancelled_error, print_cancelled_message};
use clap::Parser;
use config_context::ConfigCommandContext;
use dispatch::run_command;
use error::{ExitCode, ExitCodeExt};
use splunk_config::ConfigLoader;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[tokio::main]
async fn main() {
    // Load .env file BEFORE CLI parsing so clap env defaults can read .env values
    if let Err(e) = ConfigLoader::new().load_dotenv() {
        eprintln!("Failed to load environment: {}", e);
        std::process::exit(ExitCode::GeneralError.as_i32());
    }

    let cli = Cli::parse();

    // Initialize OpenTelemetry tracing if OTLP endpoint is configured
    let _tracing_guard = if let Some(ref endpoint) = cli.otlp_endpoint {
        let service_name = cli
            .otel_service_name
            .clone()
            .unwrap_or_else(|| "splunk-cli".to_string());

        let config = splunk_client::TracingConfig::new()
            .with_otlp_endpoint(endpoint)
            .with_service_name(service_name)
            .with_stdout(true);

        match config.init() {
            Ok(guard) => Some(guard),
            Err(e) => {
                eprintln!("Failed to initialize OpenTelemetry tracing: {}", e);
                std::process::exit(ExitCode::GeneralError.as_i32());
            }
        }
    } else {
        // Fallback to standard logging
        tracing_subscriber::registry()
            .with(EnvFilter::from_default_env())
            .with(fmt::layer())
            .init();
        None
    };

    // Initialize metrics exporter if --metrics-bind is provided
    let _metrics_exporter = if let Some(ref bind_addr) = cli.metrics_bind {
        match splunk_client::MetricsExporter::install(bind_addr) {
            Ok(exporter) => {
                tracing::info!("Metrics exporter started on http://{}/metrics", bind_addr);
                Some(exporter)
            }
            Err(e) => {
                eprintln!("Failed to start metrics exporter: {}", e);
                std::process::exit(ExitCode::GeneralError.as_i32());
            }
        }
    } else {
        None
    };

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
        args::Commands::Config { .. }
            | args::Commands::Hec { .. }
            | args::Commands::Completions { .. }
            | args::Commands::Complete { .. }
            | args::Commands::Man
    ) && !is_multi_profile_list_all;

    // Build configuration only if needed
    let config = if needs_real_config {
        let mut loader = ConfigLoader::new();

        // Apply custom config path if provided via CLI (highest priority)
        // Blank/whitespace-only values are ignored to allow fallback to env var
        if let Some(ref path) = cli.config_path {
            let path_str = path.to_string_lossy();
            if !path_str.trim().is_empty() {
                loader = loader.with_config_path(path.clone());
            }
        }

        // Apply profile from CLI if provided (highest priority)
        if let Some(ref profile_name) = cli.profile {
            loader = loader.with_profile_name(profile_name.clone());
        }

        // Apply environment variables (including SPLUNK_CONFIG_PATH and SPLUNK_PROFILE
        // if not already set via CLI args). Env vars override profile values.
        loader = match loader.from_env() {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Failed to load configuration from environment: {:#}", e);
                std::process::exit(ExitCode::GeneralError.as_i32());
            }
        };

        // Load from profile if profile_name is now set (from CLI or env var)
        if loader.profile_name().is_some() {
            loader = match loader.from_profile() {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("Failed to load configuration from profile: {:#}", e);
                    std::process::exit(ExitCode::GeneralError.as_i32());
                }
            };
        }

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

        // Apply circuit breaker overrides
        if cli.no_circuit_breaker {
            loader = loader.with_circuit_breaker_enabled(false);
        }
        if let Some(threshold) = cli.circuit_failure_threshold {
            loader = loader.with_circuit_failure_threshold(threshold);
        }
        if let Some(window) = cli.circuit_failure_window {
            loader = loader.with_circuit_failure_window_seconds(window);
        }
        if let Some(timeout) = cli.circuit_reset_timeout {
            loader = loader.with_circuit_reset_timeout_seconds(timeout);
        }
        if let Some(requests) = cli.circuit_half_open_requests {
            loader = loader.with_circuit_half_open_requests(requests);
        }

        // Build search defaults with env var overrides (matching TUI behavior)
        // Must be done before loader.build() since build() consumes the loader
        let search_defaults = loader.build_search_defaults(None);

        let config = match loader.build() {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Failed to build configuration: {:#}", e);
                std::process::exit(ExitCode::GeneralError.as_i32());
            }
        };

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
        ConfigCommandContext::Real(Box::new(config), search_defaults, cli.no_cache)
    } else {
        ConfigCommandContext::Placeholder
    };

    // Execute command
    let exit_code = match run_command(cli, config_context, &cancel).await {
        Ok(()) => ExitCode::Success,
        Err(e) if is_cancelled_error(&e) => {
            print_cancelled_message();
            ExitCode::Interrupted
        }
        Err(e) => {
            // Print the error message
            eprintln!("{:#}", e);

            // Return structured exit code
            e.exit_code()
        }
    };

    // Shutdown tracing to ensure all spans are flushed
    if let Some(guard) = _tracing_guard {
        guard.shutdown();
    }

    std::process::exit(exit_code.as_i32());
}
