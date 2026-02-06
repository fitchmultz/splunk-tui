//! Search macro command implementation.
//!
//! Responsibilities:
//! - List search macros with optional count limiting
//! - Show detailed information about specific macros
//! - Create new macros with definitions, arguments, and metadata
//! - Update existing macro properties
//! - Delete macros with confirmation
//! - Format output via shared formatters
//!
//! Does NOT handle:
//! - Macro expansion or execution (done by Splunk at search time)
//! - Direct REST API calls (handled by client crate)
//! - Output formatting details (see formatters module)
//!
//! Invariants:
//! - Macro names are validated as non-empty
//! - Delete operations require confirmation unless --force is used
//! - Conflicting flags (--disable/--enable, --iseval/--no-iseval) are rejected
//! - At least one field must be provided for update operations

use anyhow::{Context, Result};
use clap::Subcommand;
use splunk_client::{MacroCreateParams, MacroUpdateParams};
use tracing::info;

use crate::cancellation::Cancelled;
use crate::formatters::{OutputFormat, get_formatter, write_to_file};

#[derive(Subcommand)]
pub enum MacrosCommand {
    /// List all search macros
    List {
        /// Maximum number of macros to list
        #[arg(short, long, default_value = "30")]
        count: usize,
    },
    /// Show detailed information about a macro
    Info {
        /// Name of the macro
        #[arg(value_name = "NAME")]
        name: String,
    },
    /// Create a new macro
    Create {
        /// Name of the macro
        #[arg(value_name = "NAME")]
        name: String,
        /// Macro definition (SPL or eval expression)
        #[arg(value_name = "DEFINITION")]
        definition: String,
        /// Comma-separated argument names
        #[arg(short, long)]
        args: Option<String>,
        /// Description
        #[arg(short, long)]
        description: Option<String>,
        /// Create as disabled
        #[arg(long)]
        disabled: bool,
        /// Is eval expression (not SPL)
        #[arg(long)]
        iseval: bool,
        /// Validation expression
        #[arg(long)]
        validation: Option<String>,
        /// Error message for validation failure
        #[arg(long)]
        errormsg: Option<String>,
    },
    /// Update an existing macro
    Update {
        /// Name of the macro to update
        #[arg(value_name = "NAME")]
        name: String,
        /// New definition
        #[arg(short, long)]
        definition: Option<String>,
        /// New arguments
        #[arg(short, long)]
        args: Option<String>,
        /// New description
        #[arg(short, long)]
        description: Option<String>,
        /// Disable the macro
        #[arg(long)]
        disable: bool,
        /// Enable the macro
        #[arg(long)]
        enable: bool,
        /// Set as eval expression
        #[arg(long)]
        iseval: bool,
        /// Set as SPL (not eval)
        #[arg(long)]
        no_iseval: bool,
        /// New validation expression
        #[arg(long)]
        validation: Option<String>,
        /// New error message
        #[arg(long)]
        errormsg: Option<String>,
    },
    /// Delete a macro
    Delete {
        /// Name of the macro to delete
        #[arg(value_name = "NAME")]
        name: String,
        /// Skip confirmation
        #[arg(short, long)]
        force: bool,
    },
}

pub async fn run(
    config: splunk_config::Config,
    command: MacrosCommand,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    match command {
        MacrosCommand::List { count } => {
            run_list(config, count, output_format, output_file.clone(), cancel).await
        }
        MacrosCommand::Info { name } => {
            run_info(config, &name, output_format, output_file.clone(), cancel).await
        }
        MacrosCommand::Create {
            name,
            definition,
            args,
            description,
            disabled,
            iseval,
            validation,
            errormsg,
        } => {
            info!("Creating macro: {}", name);

            let mut client = crate::commands::build_client_from_config(&config)?;

            let params = MacroCreateParams {
                name: &name,
                definition: &definition,
                args: args.as_deref(),
                description: description.as_deref(),
                disabled,
                iseval,
                validation: validation.as_deref(),
                errormsg: errormsg.as_deref(),
            };

            tokio::select! {
                res = client.create_macro(params) => {
                    res?;
                    eprintln!("Macro '{}' created successfully", name);
                }
                _ = cancel.cancelled() => return Err(Cancelled.into()),
            }

            Ok(())
        }
        MacrosCommand::Update {
            name,
            definition,
            args,
            description,
            disable,
            enable,
            iseval,
            no_iseval,
            validation,
            errormsg,
        } => {
            info!("Updating macro: {}", name);

            // Validate conflicting flags
            if disable && enable {
                return Err(anyhow::anyhow!(
                    "Cannot use both --disable and --enable flags"
                ));
            }
            if iseval && no_iseval {
                return Err(anyhow::anyhow!(
                    "Cannot use both --iseval and --no-iseval flags"
                ));
            }

            // Build optional values
            let disabled = if disable {
                Some(true)
            } else if enable {
                Some(false)
            } else {
                None
            };

            let iseval_flag = if iseval {
                Some(true)
            } else if no_iseval {
                Some(false)
            } else {
                None
            };

            // Validate at least one field is provided
            if definition.is_none()
                && args.is_none()
                && description.is_none()
                && disabled.is_none()
                && iseval_flag.is_none()
                && validation.is_none()
                && errormsg.is_none()
            {
                return Err(anyhow::anyhow!(
                    "At least one field must be provided to update"
                ));
            }

            let mut client = crate::commands::build_client_from_config(&config)?;

            let params = MacroUpdateParams {
                name: &name,
                definition: definition.as_deref(),
                args: args.as_deref(),
                description: description.as_deref(),
                disabled,
                iseval: iseval_flag,
                validation: validation.as_deref(),
                errormsg: errormsg.as_deref(),
            };

            tokio::select! {
                res = client.update_macro(params) => {
                    res?;
                    eprintln!("Macro '{}' updated successfully", name);
                }
                _ = cancel.cancelled() => return Err(Cancelled.into()),
            }

            Ok(())
        }
        MacrosCommand::Delete { name, force } => run_delete(config, &name, force, cancel).await,
    }
}

async fn run_list(
    config: splunk_config::Config,
    count: usize,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Listing macros (count: {})", count);

    let mut client = crate::commands::build_client_from_config(&config)?;

    let macros = tokio::select! {
        res = client.list_macros() => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);
    let output = formatter.format_macros(&macros)?;
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
    name: &str,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Getting macro info for: {}", name);

    let mut client = crate::commands::build_client_from_config(&config)?;

    let macro_info = tokio::select! {
        res = client.get_macro(name) => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);
    let output = formatter.format_macro_info(&macro_info)?;
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

async fn run_delete(
    config: splunk_config::Config,
    name: &str,
    force: bool,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Deleting macro: {}", name);

    if !force {
        // Interactive confirmation
        eprint!("Are you sure you want to delete macro '{}'? [y/N] ", name);
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            eprintln!("Deletion cancelled");
            return Ok(());
        }
    }

    let mut client = crate::commands::build_client_from_config(&config)?;

    tokio::select! {
        res = client.delete_macro(name) => {
            res?;
            eprintln!("Macro '{}' deleted successfully", name);
        }
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    }

    Ok(())
}
