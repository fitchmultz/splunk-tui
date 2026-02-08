//! Apps command implementation.
//!
//! Responsibilities:
//! - List installed apps with optional count limiting
//! - Show detailed information about specific apps
//! - Enable/disable apps by name
//! - Install apps from .spl package files
//! - Remove (uninstall) apps with confirmation
//! - Format output via shared formatters
//!
//! Does NOT handle:
//! - App development or packaging (use Splunk SDK)
//! - Direct REST API calls (handled by client crate)
//! - Output formatting details (see formatters module)
//!
//! Invariants:
//! - File paths are validated before upload operations
//! - Confirmation prompts are shown unless --force is used
//! - App names are passed through without modification

use anyhow::{Context, Result};
use clap::Subcommand;
use tracing::info;

use splunk_config::constants::*;

use crate::cancellation::Cancelled;
use crate::formatters::{OutputFormat, get_formatter, output_result};

#[derive(Subcommand)]
pub enum AppsCommand {
    /// List installed apps
    List {
        /// Maximum number of apps to list
        #[arg(short, long, default_value_t = DEFAULT_LIST_PAGE_SIZE)]
        count: usize,
    },
    /// Show detailed information about an app
    Info {
        /// App name (e.g., 'search', 'launcher')
        #[arg(value_name = "APP_NAME")]
        app_name: String,
    },
    /// Enable an app by name
    Enable {
        /// App name to enable
        #[arg(value_name = "APP_NAME")]
        app_name: String,
    },
    /// Disable an app by name
    Disable {
        /// App name to disable
        #[arg(value_name = "APP_NAME")]
        app_name: String,
    },
    /// Install an app from a .spl file
    Install {
        /// Path to the .spl package file
        #[arg(value_name = "FILE_PATH")]
        file_path: std::path::PathBuf,
    },
    /// Remove (uninstall) an app by name
    Remove {
        /// App name to remove
        #[arg(value_name = "APP_NAME")]
        app_name: String,

        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
}

pub async fn run(
    config: splunk_config::Config,
    command: AppsCommand,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    match command {
        AppsCommand::List { count } => {
            run_list(config, count, output_format, output_file.clone(), cancel).await
        }
        AppsCommand::Info { app_name } => {
            run_info(
                config,
                &app_name,
                output_format,
                output_file.clone(),
                cancel,
            )
            .await
        }
        AppsCommand::Enable { app_name } => run_enable(config, &app_name, cancel).await,
        AppsCommand::Disable { app_name } => run_disable(config, &app_name, cancel).await,
        AppsCommand::Install { file_path } => {
            run_install(
                config,
                &file_path,
                output_format,
                output_file.clone(),
                cancel,
            )
            .await
        }
        AppsCommand::Remove { app_name, force } => {
            run_remove(config, &app_name, force, cancel).await
        }
    }
}

async fn run_list(
    config: splunk_config::Config,
    count: usize,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Listing installed apps (count: {})", count);

    let client = crate::commands::build_client_from_config(&config)?;

    let apps = tokio::select! {
        res = client.list_apps(Some(count), None) => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);
    let output = formatter.format_apps(&apps)?;
    output_result(&output, format, output_file.as_ref())?;

    Ok(())
}

async fn run_info(
    config: splunk_config::Config,
    app_name: &str,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Getting app info for: {}", app_name);

    let client = crate::commands::build_client_from_config(&config)?;

    let app = tokio::select! {
        res = client.get_app(app_name) => res,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    }
    .with_context(|| format!("Failed to get app information for '{}'", app_name))?;

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);
    let output = formatter.format_app_info(&app)?;
    output_result(&output, format, output_file.as_ref())?;

    Ok(())
}

async fn run_enable(
    config: splunk_config::Config,
    app_name: &str,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Enabling app: {}", app_name);

    let client = crate::commands::build_client_from_config(&config)?;

    tokio::select! {
        res = client.enable_app(app_name) => res,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    }
    .with_context(|| format!("Failed to enable app '{}'", app_name))?;

    println!("App '{}' enabled successfully.", app_name);

    Ok(())
}

async fn run_disable(
    config: splunk_config::Config,
    app_name: &str,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Disabling app: {}", app_name);

    let client = crate::commands::build_client_from_config(&config)?;

    tokio::select! {
        res = client.disable_app(app_name) => res,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    }
    .with_context(|| format!("Failed to disable app '{}'", app_name))?;

    println!("App '{}' disabled successfully.", app_name);

    Ok(())
}

async fn run_install(
    config: splunk_config::Config,
    file_path: &std::path::Path,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    // Validate file exists before attempting upload
    if !file_path.exists() {
        return Err(anyhow::anyhow!(
            "App package file not found: {}",
            file_path.display()
        ));
    }

    info!("Installing app from: {}", file_path.display());

    let client = crate::commands::build_client_from_config(&config)?;

    let app = tokio::select! {
        res = client.install_app(file_path) => res,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    }
    .with_context(|| format!("Failed to install app from '{}'", file_path.display()))?;

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);
    let output = formatter.format_app_info(&app)?;
    output_result(&output, format, output_file.as_ref())?;

    Ok(())
}

async fn run_remove(
    config: splunk_config::Config,
    app_name: &str,
    force: bool,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    if !force && !crate::interactive::confirm_delete(app_name, "app")? {
        return Ok(());
    }

    info!("Removing app: {}", app_name);

    let client = crate::commands::build_client_from_config(&config)?;

    tokio::select! {
        res = client.remove_app(app_name) => res,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    }
    .with_context(|| format!("Failed to remove app '{}'", app_name))?;

    println!("App '{}' removed successfully.", app_name);

    Ok(())
}
