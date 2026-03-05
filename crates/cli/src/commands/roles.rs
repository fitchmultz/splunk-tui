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

use anyhow::Result;
use clap::Subcommand;
use tracing::info;

use crate::formatters::{OutputFormat, get_formatter, output_result};
use splunk_config::constants::*;

#[derive(Debug, Subcommand)]
pub enum RolesCommand {
    /// List all roles (default)
    List {
        /// Maximum number of roles to list
        #[arg(short, long, default_value_t = DEFAULT_LIST_PAGE_SIZE)]
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
    no_cache: bool,
) -> Result<()> {
    match command {
        RolesCommand::List { count } => {
            run_list(config, count, output_format, output_file, cancel, no_cache).await
        }
        RolesCommand::Capabilities => {
            run_capabilities(config, output_format, output_file, cancel, no_cache).await
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
                no_cache,
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
                no_cache,
            )
            .await
        }
        RolesCommand::Delete { name, force } => {
            run_delete(config, &name, force, cancel, no_cache).await
        }
    }
}

async fn run_list(
    config: splunk_config::Config,
    count: usize,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
    no_cache: bool,
) -> Result<()> {
    info!("Listing roles");

    let client = crate::commands::build_client_from_config(&config, Some(no_cache))?;

    let roles = cancellable!(client.list_roles(Some(count), None), cancel)?;

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);

    let output = formatter.format_roles(&roles)?;
    output_result(&output, format, output_file.as_ref())?;

    Ok(())
}

async fn run_capabilities(
    config: splunk_config::Config,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
    no_cache: bool,
) -> Result<()> {
    info!("Listing capabilities");

    let client = crate::commands::build_client_from_config(&config, Some(no_cache))?;

    let capabilities = cancellable!(client.list_capabilities(), cancel)?;

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);

    let output = formatter.format_capabilities(&capabilities)?;
    output_result(&output, format, output_file.as_ref())?;

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
    no_cache: bool,
) -> Result<()> {
    info!("Creating role: {}", name);

    let client = crate::commands::build_client_from_config(&config, Some(no_cache))?;

    let params = splunk_client::CreateRoleParams {
        name: name.to_string(),
        capabilities,
        search_indexes,
        search_filter,
        imported_roles,
        default_app,
    };

    cancellable_with!(client.create_role(&params), cancel, |role| {
        println!("Role '{}' created successfully.", role.name);
        Ok(())
    })
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
    no_cache: bool,
) -> Result<()> {
    info!("Modifying role: {}", name);

    let client = crate::commands::build_client_from_config(&config, Some(no_cache))?;

    let params = splunk_client::ModifyRoleParams {
        capabilities,
        search_indexes,
        search_filter,
        imported_roles,
        default_app,
    };

    cancellable_with!(client.modify_role(name, &params), cancel, |role| {
        println!("Role '{}' modified successfully.", role.name);
        Ok(())
    })
}

async fn run_delete(
    config: splunk_config::Config,
    name: &str,
    force: bool,
    cancel: &crate::cancellation::CancellationToken,
    no_cache: bool,
) -> Result<()> {
    if !force && !crate::interactive::confirm_delete(name, "role")? {
        return Ok(());
    }

    info!("Deleting role: {}", name);

    let client = crate::commands::build_client_from_config(&config, Some(no_cache))?;

    cancellable_with!(client.delete_role(name), cancel, |_res| {
        println!("Role '{}' deleted successfully.", name);
        Ok(())
    })
}
