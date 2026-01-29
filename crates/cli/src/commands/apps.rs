//! Apps command implementation.

use anyhow::{Context, Result};
use clap::Subcommand;
use tracing::info;

use crate::cancellation::Cancelled;
use crate::formatters::{OutputFormat, get_formatter, write_to_file};

#[derive(Subcommand)]
pub enum AppsCommand {
    /// List installed apps
    List {
        /// Maximum number of apps to list
        #[arg(short, long, default_value = "30")]
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

    let mut client = crate::commands::build_client_from_config(&config)?;

    let apps = tokio::select! {
        res = client.list_apps(Some(count as u64), None) => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);
    let output = formatter.format_apps(&apps)?;
    if let Some(ref path) = output_file {
        write_to_file(&output, path)
            .with_context(|| format!("Failed to write output to {}", path.display()))?;
        eprintln!(
            "Results written to {} ({:?} format)",
            path.display(),
            format
        );
    } else {
        print!("{}", output);
    }

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

    let mut client = crate::commands::build_client_from_config(&config)?;

    let app = tokio::select! {
        res = client.get_app(app_name) => res,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    }
    .with_context(|| format!("Failed to get app information for '{}'", app_name))?;

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);
    let output = formatter.format_app_info(&app)?;
    if let Some(ref path) = output_file {
        write_to_file(&output, path)
            .with_context(|| format!("Failed to write output to {}", path.display()))?;
        eprintln!(
            "Results written to {} ({:?} format)",
            path.display(),
            format
        );
    } else {
        print!("{}", output);
    }

    Ok(())
}

async fn run_enable(
    config: splunk_config::Config,
    app_name: &str,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Enabling app: {}", app_name);

    let mut client = crate::commands::build_client_from_config(&config)?;

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

    let mut client = crate::commands::build_client_from_config(&config)?;

    tokio::select! {
        res = client.disable_app(app_name) => res,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    }
    .with_context(|| format!("Failed to disable app '{}'", app_name))?;

    println!("App '{}' disabled successfully.", app_name);

    Ok(())
}
