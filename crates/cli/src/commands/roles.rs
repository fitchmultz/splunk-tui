//! Roles and capabilities command implementation.
//!
//! Responsibilities:
//! - List roles with optional count limiting
//! - List available capabilities
//! - Create new roles with assigned capabilities and settings
//! - Modify existing role properties
//! - Delete roles with confirmation
//! - Format output via shared formatters
//!
//! Does NOT handle:
//! - Role assignment to users (see users module)
//! - Direct REST API calls (handled by client crate)
//! - Output formatting details (see formatters module)
//!
//! Invariants:
//! - Role names are validated as non-empty
//! - Delete operations require confirmation unless --force is used
//! - Capability lists are passed through without validation
//! - At least one field must be provided for update operations

use anyhow::{Context, Result};
use clap::Subcommand;
use tracing::info;

use crate::cancellation::Cancelled;
use crate::formatters::{OutputFormat, get_formatter, write_to_file};

#[derive(Debug, Subcommand)]
pub enum RolesCommand {
    /// List all roles (default)
    List {
        /// Maximum number of roles to list
        #[arg(short, long, default_value = "30")]
        count: usize,
    },
    /// List all available capabilities
    Capabilities,
    /// Create a new role
    Create {
        /// Role name (required)
        name: String,
        /// Capabilities to assign (comma-separated)
        #[arg(short, long, value_delimiter = ',')]
        capabilities: Vec<String>,
        /// Search indexes to allow (comma-separated)
        #[arg(short, long, value_delimiter = ',')]
        search_indexes: Vec<String>,
        /// Search filter to restrict results
        #[arg(long)]
        search_filter: Option<String>,
        /// Roles to import/inherit from (comma-separated)
        #[arg(short, long, value_delimiter = ',')]
        imported_roles: Vec<String>,
        /// Default app for the role
        #[arg(long)]
        default_app: Option<String>,
    },
    /// Modify an existing role
    Update {
        /// Role name (required)
        name: String,
        /// Capabilities to assign (comma-separated, replaces existing)
        #[arg(short, long, value_delimiter = ',')]
        capabilities: Option<Vec<String>>,
        /// Search indexes to allow (comma-separated, replaces existing)
        #[arg(short, long, value_delimiter = ',')]
        search_indexes: Option<Vec<String>>,
        /// Search filter to restrict results
        #[arg(long)]
        search_filter: Option<String>,
        /// Roles to import/inherit from (comma-separated, replaces existing)
        #[arg(short, long, value_delimiter = ',')]
        imported_roles: Option<Vec<String>>,
        /// Default app for the role
        #[arg(long)]
        default_app: Option<String>,
    },
    /// Delete a role
    Delete {
        /// Role name (required)
        name: String,
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
}

pub async fn run(
    config: splunk_config::Config,
    command: RolesCommand,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    match command {
        RolesCommand::List { count } => {
            run_list(config, count, output_format, output_file, cancel).await
        }
        RolesCommand::Capabilities => {
            run_capabilities(config, output_format, output_file, cancel).await
        }
        RolesCommand::Create {
            name,
            capabilities,
            search_indexes,
            search_filter,
            imported_roles,
            default_app,
        } => {
            run_create(
                config,
                &name,
                capabilities,
                search_indexes,
                search_filter,
                imported_roles,
                default_app,
                cancel,
            )
            .await
        }
        RolesCommand::Update {
            name,
            capabilities,
            search_indexes,
            search_filter,
            imported_roles,
            default_app,
        } => {
            run_update(
                config,
                &name,
                capabilities,
                search_indexes,
                search_filter,
                imported_roles,
                default_app,
                cancel,
            )
            .await
        }
        RolesCommand::Delete { name, force } => run_delete(config, &name, force, cancel).await,
    }
}

async fn run_list(
    config: splunk_config::Config,
    count: usize,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Listing roles");

    let mut client = crate::commands::build_client_from_config(&config)?;

    let roles = tokio::select! {
        res = client.list_roles(Some(count as u64), None) => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);

    let output = formatter.format_roles(&roles)?;
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

async fn run_capabilities(
    config: splunk_config::Config,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Listing capabilities");

    let mut client = crate::commands::build_client_from_config(&config)?;

    let capabilities = tokio::select! {
        res = client.list_capabilities() => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);

    let output = formatter.format_capabilities(&capabilities)?;
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

#[allow(clippy::too_many_arguments)]
async fn run_create(
    config: splunk_config::Config,
    name: &str,
    capabilities: Vec<String>,
    search_indexes: Vec<String>,
    search_filter: Option<String>,
    imported_roles: Vec<String>,
    default_app: Option<String>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Creating role: {}", name);

    let mut client = crate::commands::build_client_from_config(&config)?;

    let params = splunk_client::CreateRoleParams {
        name: name.to_string(),
        capabilities,
        search_indexes,
        search_filter,
        imported_roles,
        default_app,
    };

    tokio::select! {
        res = client.create_role(&params) => {
            let role = res?;
            println!("Role '{}' created successfully.", role.name);
            Ok(())
        }
        _ = cancel.cancelled() => Err(Cancelled.into()),
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_update(
    config: splunk_config::Config,
    name: &str,
    capabilities: Option<Vec<String>>,
    search_indexes: Option<Vec<String>>,
    search_filter: Option<String>,
    imported_roles: Option<Vec<String>>,
    default_app: Option<String>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Modifying role: {}", name);

    let mut client = crate::commands::build_client_from_config(&config)?;

    let params = splunk_client::ModifyRoleParams {
        capabilities,
        search_indexes,
        search_filter,
        imported_roles,
        default_app,
    };

    tokio::select! {
        res = client.modify_role(name, &params) => {
            let role = res?;
            println!("Role '{}' modified successfully.", role.name);
            Ok(())
        }
        _ = cancel.cancelled() => Err(Cancelled.into()),
    }
}

async fn run_delete(
    config: splunk_config::Config,
    name: &str,
    force: bool,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    if !force {
        print!("Are you sure you want to delete role '{}'? [y/N] ", name);
        use std::io::Write;
        std::io::stdout().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Delete cancelled.");
            return Ok(());
        }
    }

    info!("Deleting role: {}", name);

    let mut client = crate::commands::build_client_from_config(&config)?;

    tokio::select! {
        res = client.delete_role(name) => {
            res?;
            println!("Role '{}' deleted successfully.", name);
            Ok(())
        }
        _ = cancel.cancelled() => Err(Cancelled.into()),
    }
}
