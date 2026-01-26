//! Apps command implementation.

use anyhow::{Context, Result};
use clap::Subcommand;
use splunk_client::SplunkClient;
use tracing::info;

use crate::commands::convert_auth_strategy;
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
) -> Result<()> {
    match command {
        AppsCommand::List { count } => {
            run_list(config, count, output_format, output_file.clone()).await
        }
        AppsCommand::Info { app_name } => {
            run_info(config, &app_name, output_format, output_file.clone()).await
        }
        AppsCommand::Enable { app_name } => run_enable(config, &app_name).await,
        AppsCommand::Disable { app_name } => run_disable(config, &app_name).await,
    }
}

async fn run_list(
    config: splunk_config::Config,
    count: usize,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
) -> Result<()> {
    info!("Listing installed apps (count: {})", count);

    let auth_strategy = convert_auth_strategy(&config.auth.strategy);
    let mut client = SplunkClient::builder()
        .base_url(config.connection.base_url)
        .auth_strategy(auth_strategy)
        .skip_verify(config.connection.skip_verify)
        .timeout(config.connection.timeout)
        .build()?;

    let apps = client.list_apps(Some(count as u64), None).await?;

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
) -> Result<()> {
    info!("Getting app info for: {}", app_name);

    let auth_strategy = convert_auth_strategy(&config.auth.strategy);
    let mut client = SplunkClient::builder()
        .base_url(config.connection.base_url)
        .auth_strategy(auth_strategy)
        .skip_verify(config.connection.skip_verify)
        .timeout(config.connection.timeout)
        .build()?;

    let app = client
        .get_app(app_name)
        .await
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

async fn run_enable(config: splunk_config::Config, app_name: &str) -> Result<()> {
    info!("Enabling app: {}", app_name);

    let auth_strategy = convert_auth_strategy(&config.auth.strategy);
    let mut client = SplunkClient::builder()
        .base_url(config.connection.base_url)
        .auth_strategy(auth_strategy)
        .skip_verify(config.connection.skip_verify)
        .timeout(config.connection.timeout)
        .build()?;

    client
        .enable_app(app_name)
        .await
        .with_context(|| format!("Failed to enable app '{}'", app_name))?;

    println!("App '{}' enabled successfully.", app_name);

    Ok(())
}

async fn run_disable(config: splunk_config::Config, app_name: &str) -> Result<()> {
    info!("Disabling app: {}", app_name);

    let auth_strategy = convert_auth_strategy(&config.auth.strategy);
    let mut client = SplunkClient::builder()
        .base_url(config.connection.base_url)
        .auth_strategy(auth_strategy)
        .skip_verify(config.connection.skip_verify)
        .timeout(config.connection.timeout)
        .build()?;

    client
        .disable_app(app_name)
        .await
        .with_context(|| format!("Failed to disable app '{}'", app_name))?;

    println!("App '{}' disabled successfully.", app_name);

    Ok(())
}
