//! License command implementation.
//!
//! Responsibilities:
//! - Fetch license usage, pools, stacks, and installed licenses from Splunk.
//! - Install license files.
//! - Manage license pools (create, modify, delete).
//! - Activate/deactivate licenses.
//! - Format and display license information.
//!
//! Does NOT handle:
//! - License file validation (handled by Splunk server).
//!
//! Invariants:
//! - Requires an authenticated Splunk client.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Subcommand;
use tracing::info;

use crate::cancellation::Cancelled;
use crate::formatters::{
    LicenseActivationOutput, LicenseInfoOutput, LicenseInstallOutput, LicensePoolOperationOutput,
    OutputFormat, get_formatter, write_to_file,
};

#[derive(Subcommand)]
pub enum LicenseCommand {
    /// Show license information (default)
    #[command(alias = "info")]
    Show,

    /// List installed licenses
    List,

    /// Install a license file
    Install {
        /// Path to the .sla license file
        #[arg(value_name = "FILE")]
        file_path: PathBuf,
    },

    /// Manage license pools
    #[command(subcommand)]
    Pool(PoolCommands),

    /// Activate a license
    Activate {
        /// License name to activate
        #[arg(value_name = "NAME")]
        license_name: String,
    },

    /// Deactivate a license
    Deactivate {
        /// License name to deactivate
        #[arg(value_name = "NAME")]
        license_name: String,
    },
}

#[derive(Subcommand)]
pub enum PoolCommands {
    /// List all license pools
    List,

    /// Create a new license pool
    Create {
        /// Pool name
        #[arg(value_name = "NAME")]
        name: String,
        /// Stack ID to associate with
        #[arg(value_name = "STACK_ID")]
        stack_id: String,
        /// Quota in bytes
        #[arg(short, long)]
        quota: Option<u64>,
        /// Pool description
        #[arg(short, long)]
        description: Option<String>,
    },

    /// Delete a license pool
    Delete {
        /// Pool name to delete
        #[arg(value_name = "NAME")]
        name: String,

        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },

    /// Modify an existing license pool
    Modify {
        /// Pool name to modify
        #[arg(value_name = "NAME")]
        name: String,
        /// New quota in bytes
        #[arg(short, long)]
        quota: Option<u64>,
        /// New description
        #[arg(short, long)]
        description: Option<String>,
    },
}

pub async fn run(
    config: splunk_config::Config,
    command: LicenseCommand,
    output_format: &str,
    output_file: Option<PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    match command {
        LicenseCommand::Show => run_show(config, output_format, output_file.clone(), cancel).await,
        LicenseCommand::List => run_list(config, output_format, output_file.clone(), cancel).await,
        LicenseCommand::Install { file_path } => {
            run_install(
                config,
                &file_path,
                output_format,
                output_file.clone(),
                cancel,
            )
            .await
        }
        LicenseCommand::Pool(pool_cmd) => {
            run_pool(config, pool_cmd, output_format, output_file.clone(), cancel).await
        }
        LicenseCommand::Activate { license_name } => {
            run_activate(config, &license_name, cancel).await
        }
        LicenseCommand::Deactivate { license_name } => {
            run_deactivate(config, &license_name, cancel).await
        }
    }
}

async fn run_show(
    config: splunk_config::Config,
    output_format: &str,
    output_file: Option<PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Fetching license information...");

    let client = crate::commands::build_client_from_config(&config)?;

    let usage = tokio::select! {
        res = client.get_license_usage() => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };
    let pools = tokio::select! {
        res = client.list_license_pools() => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };
    let stacks = tokio::select! {
        res = client.list_license_stacks() => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    let output = LicenseInfoOutput {
        usage,
        pools,
        stacks,
    };

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);
    let formatted = formatter.format_license(&output)?;

    if let Some(ref path) = output_file {
        write_to_file(&formatted, path)
            .with_context(|| format!("Failed to write output to {}", path.display()))?;
        eprintln!(
            "Results written to {} ({:?} format)",
            path.display(),
            format
        );
    } else {
        println!("{}", formatted);
    }

    Ok(())
}

async fn run_list(
    config: splunk_config::Config,
    output_format: &str,
    output_file: Option<PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Listing installed licenses...");

    let client = crate::commands::build_client_from_config(&config)?;

    let licenses = tokio::select! {
        res = client.list_installed_licenses() => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);
    let formatted = formatter.format_installed_licenses(&licenses)?;

    if let Some(ref path) = output_file {
        write_to_file(&formatted, path)
            .with_context(|| format!("Failed to write output to {}", path.display()))?;
        eprintln!(
            "Results written to {} ({:?} format)",
            path.display(),
            format
        );
    } else {
        println!("{}", formatted);
    }

    Ok(())
}

async fn run_install(
    config: splunk_config::Config,
    file_path: &std::path::Path,
    output_format: &str,
    output_file: Option<PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    // Validate file exists before attempting upload
    if !file_path.exists() {
        return Err(anyhow::anyhow!(
            "License file not found: {}",
            file_path.display()
        ));
    }

    info!("Installing license from: {}", file_path.display());

    let client = crate::commands::build_client_from_config(&config)?;

    let result = tokio::select! {
        res = client.install_license(file_path) => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    let output = LicenseInstallOutput {
        success: result.success,
        message: result.message,
        license_name: result.license_name,
    };

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);
    let formatted = formatter.format_license_install(&output)?;

    if let Some(ref path) = output_file {
        write_to_file(&formatted, path)
            .with_context(|| format!("Failed to write output to {}", path.display()))?;
        eprintln!(
            "Results written to {} ({:?} format)",
            path.display(),
            format
        );
    } else {
        println!("{}", formatted);
    }

    Ok(())
}

async fn run_pool(
    config: splunk_config::Config,
    command: PoolCommands,
    output_format: &str,
    output_file: Option<PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    match command {
        PoolCommands::List => run_pool_list(config, output_format, output_file, cancel).await,
        PoolCommands::Create {
            name,
            stack_id,
            quota,
            description,
        } => {
            run_pool_create(
                config,
                &name,
                &stack_id,
                quota,
                description,
                output_format,
                output_file,
                cancel,
            )
            .await
        }
        PoolCommands::Delete { name, force } => run_pool_delete(config, &name, force, cancel).await,
        PoolCommands::Modify {
            name,
            quota,
            description,
        } => {
            run_pool_modify(
                config,
                &name,
                quota,
                description,
                output_format,
                output_file,
                cancel,
            )
            .await
        }
    }
}

async fn run_pool_list(
    config: splunk_config::Config,
    output_format: &str,
    output_file: Option<PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Listing license pools...");

    let client = crate::commands::build_client_from_config(&config)?;

    let pools = tokio::select! {
        res = client.list_license_pools() => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);
    let formatted = formatter.format_license_pools(&pools)?;

    if let Some(ref path) = output_file {
        write_to_file(&formatted, path)
            .with_context(|| format!("Failed to write output to {}", path.display()))?;
        eprintln!(
            "Results written to {} ({:?} format)",
            path.display(),
            format
        );
    } else {
        println!("{}", formatted);
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn run_pool_create(
    config: splunk_config::Config,
    name: &str,
    stack_id: &str,
    quota: Option<u64>,
    description: Option<String>,
    output_format: &str,
    output_file: Option<PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Creating license pool: {} (stack: {})", name, stack_id);

    let client = crate::commands::build_client_from_config(&config)?;

    let params = splunk_client::CreatePoolParams {
        name: name.to_string(),
        stack_id: stack_id.to_string(),
        quota_bytes: quota,
        description,
    };

    let pool = tokio::select! {
        res = client.create_license_pool(&params) => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    let output = LicensePoolOperationOutput {
        operation: "create".to_string(),
        pool_name: pool.name,
        success: true,
        message: format!("Pool '{}' created successfully", name),
    };

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);
    let formatted = formatter.format_license_pool_operation(&output)?;

    if let Some(ref path) = output_file {
        write_to_file(&formatted, path)
            .with_context(|| format!("Failed to write output to {}", path.display()))?;
        eprintln!(
            "Results written to {} ({:?} format)",
            path.display(),
            format
        );
    } else {
        println!("{}", formatted);
    }

    Ok(())
}

async fn run_pool_delete(
    config: splunk_config::Config,
    name: &str,
    force: bool,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    // Prompt for confirmation unless --force is used
    if !force {
        eprint!("Delete license pool '{}' ? [y/N] ", name);
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancelled.");
            return Ok(());
        }
    }

    info!("Deleting license pool: {}", name);

    let client = crate::commands::build_client_from_config(&config)?;

    tokio::select! {
        res = client.delete_license_pool(name) => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    println!("License pool '{}' deleted successfully.", name);

    Ok(())
}

async fn run_pool_modify(
    config: splunk_config::Config,
    name: &str,
    quota: Option<u64>,
    description: Option<String>,
    output_format: &str,
    output_file: Option<PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Modifying license pool: {}", name);

    let client = crate::commands::build_client_from_config(&config)?;

    let params = splunk_client::ModifyPoolParams {
        quota_bytes: quota,
        description,
    };

    let pool = tokio::select! {
        res = client.modify_license_pool(name, &params) => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    let output = LicensePoolOperationOutput {
        operation: "modify".to_string(),
        pool_name: pool.name,
        success: true,
        message: format!("Pool '{}' modified successfully", name),
    };

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);
    let formatted = formatter.format_license_pool_operation(&output)?;

    if let Some(ref path) = output_file {
        write_to_file(&formatted, path)
            .with_context(|| format!("Failed to write output to {}", path.display()))?;
        eprintln!(
            "Results written to {} ({:?} format)",
            path.display(),
            format
        );
    } else {
        println!("{}", formatted);
    }

    Ok(())
}

async fn run_activate(
    config: splunk_config::Config,
    license_name: &str,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Activating license: {}", license_name);

    let client = crate::commands::build_client_from_config(&config)?;

    let result = tokio::select! {
        res = client.activate_license(license_name) => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    let output = LicenseActivationOutput {
        operation: "activate".to_string(),
        license_name: license_name.to_string(),
        success: result.success,
        message: result.message,
    };

    println!("{}", output.message);

    Ok(())
}

async fn run_deactivate(
    config: splunk_config::Config,
    license_name: &str,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Deactivating license: {}", license_name);

    let client = crate::commands::build_client_from_config(&config)?;

    let result = tokio::select! {
        res = client.deactivate_license(license_name) => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    let output = LicenseActivationOutput {
        operation: "deactivate".to_string(),
        license_name: license_name.to_string(),
        success: result.success,
        message: result.message,
    };

    println!("{}", output.message);

    Ok(())
}
