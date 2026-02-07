//! SHC command implementation.
//!
//! Responsibilities:
//! - Fetch and display SHC information, members, and captain
//! - Manage SHC members (add, remove)
//! - SHC administrative operations (rolling restart, set captain)
//!
//! Does NOT handle:
//! - Low-level SHC API calls (handled by client crate)
//! - Output formatting details (see formatters module)
//!
//! Invariants:
//! - SHC operations require captain access or administrative privileges
//! - Member removal requires the member to be decommissioned first
//! - Rolling restart affects all cluster members

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Subcommand;
use tracing::{info, warn};

use crate::cancellation::Cancelled;
use crate::formatters::{
    OutputFormat, Pagination, ShcCaptainOutput, ShcConfigOutput, ShcManagementOutput,
    ShcMemberOutput, ShcStatusOutput, TableFormatter, get_formatter, write_to_file,
};
use splunk_config::constants::*;

/// SHC management subcommands.
#[derive(Debug, Subcommand)]
pub enum ShcCommand {
    /// Show SHC status and information (default)
    #[command(alias = "info")]
    Show {
        /// Show detailed SHC information with members
        #[arg(short, long)]
        detailed: bool,
        /// Offset into the SHC member list (zero-based). Only applies with --detailed.
        #[arg(long, default_value = "0")]
        offset: usize,
        /// Number of members per page. Only applies with --detailed.
        #[arg(short, long, default_value_t = DEFAULT_LIST_PAGE_SIZE, visible_alias = "page-size")]
        count: usize,
    },

    /// Show SHC members
    Members {
        /// Offset into the member list (zero-based)
        #[arg(long, default_value = "0")]
        offset: usize,
        /// Number of members per page
        #[arg(short, long, default_value_t = DEFAULT_LIST_PAGE_SIZE, visible_alias = "page-size")]
        count: usize,
    },

    /// Show SHC captain information
    Captain,

    /// Show SHC configuration
    Config,

    /// Manage SHC members
    #[command(subcommand)]
    Manage(ManageCommand),
}

/// SHC member management subcommands.
#[derive(Debug, Subcommand)]
pub enum ManageCommand {
    /// Add a member to the SHC
    Add {
        /// Target member URI (e.g., https://search-head:8089)
        #[arg(value_name = "TARGET_URI")]
        target_uri: String,
    },
    /// Remove a member from the SHC
    Remove {
        /// Member GUID to remove
        #[arg(value_name = "MEMBER_GUID")]
        member_guid: String,
        /// Force removal without confirmation
        #[arg(short, long)]
        force: bool,
    },
    /// Trigger a rolling restart of all SHC members
    RollingRestart {
        /// Force restart even if some members are not ready
        #[arg(short, long)]
        force: bool,
    },
    /// Set a specific member as captain
    SetCaptain {
        /// Target member GUID to become captain
        #[arg(value_name = "MEMBER_GUID")]
        member_guid: String,
    },
}

/// Run the SHC command.
pub async fn run(
    config: splunk_config::Config,
    command: ShcCommand,
    output_format: &str,
    output_file: Option<PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    match command {
        ShcCommand::Show {
            detailed,
            offset,
            count,
        } => {
            run_show(
                config,
                detailed,
                offset,
                count,
                output_format,
                output_file,
                cancel,
            )
            .await
        }
        ShcCommand::Members { offset, count } => {
            run_members(config, offset, count, output_format, output_file, cancel).await
        }
        ShcCommand::Captain => run_captain(config, output_format, output_file, cancel).await,
        ShcCommand::Config => run_config(config, output_format, output_file, cancel).await,
        ShcCommand::Manage(manage_cmd) => {
            run_manage(config, manage_cmd, output_format, output_file, cancel).await
        }
    }
}

/// Fetch a page of SHC members with pagination.
async fn fetch_shc_members_page(
    client: &mut splunk_client::SplunkClient,
    offset: usize,
    count: usize,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<(Option<Vec<ShcMemberOutput>>, Option<Pagination>)> {
    let members_result = tokio::select! {
        res = client.get_shc_members() => res,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    match members_result {
        Ok(members) => {
            let total = members.len();
            let page: Vec<_> = members
                .into_iter()
                .skip(offset)
                .take(count)
                .map(ShcMemberOutput::from)
                .collect();

            Ok((
                Some(page),
                Some(Pagination {
                    offset,
                    page_size: count,
                    total: Some(total),
                }),
            ))
        }
        Err(e) => {
            warn!("Failed to fetch SHC members: {}", e);
            Ok((None, None))
        }
    }
}

/// Render SHC status output to file or stdout.
async fn render_shc_output(
    status: &ShcStatusOutput,
    detailed: bool,
    members: Option<Vec<ShcMemberOutput>>,
    pagination: Option<Pagination>,
    format: OutputFormat,
    output_file: Option<&PathBuf>,
) -> Result<()> {
    let output = if format == OutputFormat::Table {
        let formatter = TableFormatter;
        formatter.format_shc_status_paginated(status, detailed, members, pagination)?
    } else {
        let formatter = get_formatter(format);
        formatter.format_shc_status(status)?
    };

    if let Some(path) = output_file {
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

async fn run_show(
    config: splunk_config::Config,
    detailed: bool,
    offset: usize,
    count: usize,
    output_format: &str,
    output_file: Option<PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!(
        "Fetching SHC status (detailed: {}, offset: {}, count: {})",
        detailed, offset, count
    );

    // Validate pagination inputs (client-side pagination must be safe)
    if count == 0 {
        anyhow::bail!("The value for --count must be greater than 0");
    }

    let mut client = crate::commands::build_client_from_config(&config)?;

    let shc_status = tokio::select! {
        res = client.get_shc_status() => res,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    match shc_status {
        Ok(shc_status) => {
            // Fetch members if detailed (fetch ALL once, then paginate locally)
            let (members_output, members_pagination) = if detailed {
                fetch_shc_members_page(&mut client, offset, count, cancel).await?
            } else {
                (None, None)
            };

            let status = ShcStatusOutput {
                is_captain: shc_status.is_captain,
                is_searchable: shc_status.is_searchable,
                captain_uri: shc_status.captain_uri,
                member_count: shc_status.member_count,
                minimum_member_count: shc_status.minimum_member_count,
                rolling_restart_flag: shc_status.rolling_restart_flag,
                service_ready_flag: shc_status.service_ready_flag,
            };

            let format = OutputFormat::from_str(output_format)?;
            render_shc_output(
                &status,
                detailed,
                members_output,
                members_pagination,
                format,
                output_file.as_ref(),
            )
            .await?;
        }
        Err(e) => {
            // Not all Splunk instances are in SHC
            warn!("Failed to fetch SHC status: {}", e);
            println!("Note: This Splunk instance may not be configured as a search head cluster.");
        }
    }

    Ok(())
}

async fn run_members(
    config: splunk_config::Config,
    offset: usize,
    count: usize,
    output_format: &str,
    output_file: Option<PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!(
        "Fetching SHC members (offset: {}, count: {})",
        offset, count
    );

    if count == 0 {
        anyhow::bail!("The value for --count must be greater than 0");
    }

    let client = crate::commands::build_client_from_config(&config)?;

    let members_result = tokio::select! {
        res = client.get_shc_members() => res,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    match members_result {
        Ok(members) => {
            let total = members.len();

            // Slice safely (empty slice if offset beyond end)
            let page: Vec<_> = members
                .into_iter()
                .skip(offset)
                .take(count)
                .map(ShcMemberOutput::from)
                .collect();

            let pagination = Pagination {
                offset,
                page_size: count,
                total: Some(total),
            };

            let format = OutputFormat::from_str(output_format)?;
            let formatter = get_formatter(format);
            let output = formatter.format_shc_members(&page, &pagination)?;

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
        }
        Err(e) => {
            warn!("Failed to fetch SHC members: {}", e);
            println!("Note: This Splunk instance may not be configured as a search head cluster.");
        }
    }

    Ok(())
}

async fn run_captain(
    config: splunk_config::Config,
    output_format: &str,
    output_file: Option<PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Fetching SHC captain information");

    let client = crate::commands::build_client_from_config(&config)?;

    let captain_result = tokio::select! {
        res = client.get_shc_captain() => res,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    match captain_result {
        Ok(captain) => {
            let output = ShcCaptainOutput::from(captain);

            let format = OutputFormat::from_str(output_format)?;
            let formatter = get_formatter(format);
            let formatted = formatter.format_shc_captain(&output)?;

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
        }
        Err(e) => {
            warn!("Failed to fetch SHC captain: {}", e);
            println!("Note: This Splunk instance may not be configured as a search head cluster.");
        }
    }

    Ok(())
}

async fn run_config(
    config: splunk_config::Config,
    output_format: &str,
    output_file: Option<PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Fetching SHC configuration");

    let client = crate::commands::build_client_from_config(&config)?;

    let config_result = tokio::select! {
        res = client.get_shc_config() => res,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    match config_result {
        Ok(shc_config) => {
            let output = ShcConfigOutput::from(shc_config);

            let format = OutputFormat::from_str(output_format)?;
            let formatter = get_formatter(format);
            let formatted = formatter.format_shc_config(&output)?;

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
        }
        Err(e) => {
            warn!("Failed to fetch SHC config: {}", e);
            println!("Note: This Splunk instance may not be configured as a search head cluster.");
        }
    }

    Ok(())
}

async fn run_manage(
    config: splunk_config::Config,
    command: ManageCommand,
    output_format: &str,
    output_file: Option<PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    let client = crate::commands::build_client_from_config(&config)?;

    match command {
        ManageCommand::Add { target_uri } => {
            info!("Adding SHC member: {}", target_uri);
            let result = tokio::select! {
                res = client.add_shc_member(&target_uri) => res,
                _ = cancel.cancelled() => return Err(Cancelled.into()),
            }?;

            handle_management_result(
                result,
                &format!("add member {}", target_uri),
                output_format,
                output_file,
            )
            .await?;
        }
        ManageCommand::Remove { member_guid, force } => {
            if !force && !crate::interactive::confirm_delete(&member_guid, "SHC member")? {
                return Ok(());
            }

            info!("Removing SHC member: {}", member_guid);
            let result = tokio::select! {
                res = client.remove_shc_member(&member_guid) => res,
                _ = cancel.cancelled() => return Err(Cancelled.into()),
            }?;

            handle_management_result(
                result,
                &format!("remove member {}", member_guid),
                output_format,
                output_file,
            )
            .await?;
        }
        ManageCommand::RollingRestart { force } => {
            info!("Triggering SHC rolling restart (force: {})", force);
            let result = tokio::select! {
                res = client.rolling_restart_shc(force) => res,
                _ = cancel.cancelled() => return Err(Cancelled.into()),
            }?;

            handle_management_result(result, "rolling restart", output_format, output_file).await?;
        }
        ManageCommand::SetCaptain { member_guid } => {
            info!("Setting SHC captain to: {}", member_guid);
            let result = tokio::select! {
                res = client.set_shc_captain(&member_guid) => res,
                _ = cancel.cancelled() => return Err(Cancelled.into()),
            }?;

            handle_management_result(
                result,
                &format!("set captain {}", member_guid),
                output_format,
                output_file,
            )
            .await?;
        }
    }

    Ok(())
}

async fn handle_management_result(
    result: splunk_client::ShcManagementResponse,
    operation: &str,
    output_format: &str,
    output_file: Option<PathBuf>,
) -> Result<()> {
    let output = ShcManagementOutput {
        operation: operation.to_string(),
        target: "shc".to_string(),
        success: result.success,
        message: result
            .message
            .unwrap_or_else(|| format!("{} completed", operation)),
    };

    handle_management_output(output, output_format, output_file).await
}

async fn handle_management_output(
    output: ShcManagementOutput,
    output_format: &str,
    output_file: Option<PathBuf>,
) -> Result<()> {
    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);
    let formatted = formatter.format_shc_management(&output)?;

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
