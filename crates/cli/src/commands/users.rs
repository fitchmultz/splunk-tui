//! Users command implementation.

use anyhow::{Context, Result};
use clap::Subcommand;
use secrecy::SecretString;
use tracing::info;

use crate::cancellation::Cancelled;
use crate::formatters::{OutputFormat, get_formatter, write_to_file};

#[derive(Debug, Subcommand)]
pub enum UsersCommand {
    /// List all users (default)
    List {
        /// Maximum number of users to list
        #[arg(short, long, default_value = "30")]
        count: usize,
    },
    /// Create a new user
    Create {
        /// Username (required)
        name: String,
        /// Initial password (will prompt if not provided)
        #[arg(short, long)]
        password: Option<String>,
        /// Roles to assign (comma-separated, at least one required)
        #[arg(short, long, value_delimiter = ',')]
        roles: Vec<String>,
        /// Real name of the user
        #[arg(long)]
        realname: Option<String>,
        /// Email address of the user
        #[arg(long)]
        email: Option<String>,
        /// Default app for the user
        #[arg(long)]
        default_app: Option<String>,
    },
    /// Modify an existing user
    Modify {
        /// Username (required)
        name: String,
        /// New password
        #[arg(short, long)]
        password: Option<String>,
        /// Roles to assign (comma-separated, replaces existing)
        #[arg(short, long, value_delimiter = ',')]
        roles: Option<Vec<String>>,
        /// Real name of the user
        #[arg(long)]
        realname: Option<String>,
        /// Email address of the user
        #[arg(long)]
        email: Option<String>,
        /// Default app for the user
        #[arg(long)]
        default_app: Option<String>,
    },
    /// Delete a user
    Delete {
        /// Username (required)
        name: String,
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
}

pub async fn run(
    config: splunk_config::Config,
    command: UsersCommand,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    match command {
        UsersCommand::List { count } => {
            run_list(config, count, output_format, output_file, cancel).await
        }
        UsersCommand::Create {
            name,
            password,
            roles,
            realname,
            email,
            default_app,
        } => {
            run_create(
                config,
                &name,
                password,
                roles,
                realname,
                email,
                default_app,
                cancel,
            )
            .await
        }
        UsersCommand::Modify {
            name,
            password,
            roles,
            realname,
            email,
            default_app,
        } => {
            run_modify(
                config,
                &name,
                password,
                roles,
                realname,
                email,
                default_app,
                cancel,
            )
            .await
        }
        UsersCommand::Delete { name, force } => run_delete(config, &name, force, cancel).await,
    }
}

async fn run_list(
    config: splunk_config::Config,
    count: usize,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Listing users");

    let mut client = crate::commands::build_client_from_config(&config)?;

    let users = tokio::select! {
        res = client.list_users(Some(count as u64), None) => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);

    let output = formatter.format_users(&users)?;
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
    password: Option<String>,
    roles: Vec<String>,
    realname: Option<String>,
    email: Option<String>,
    default_app: Option<String>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Creating user: {}", name);

    // Prompt for password if not provided via CLI
    let password = match password {
        Some(p) => SecretString::new(p.into()),
        None => {
            print!("Enter password for user '{}': ", name);
            use std::io::Write;
            std::io::stdout().flush()?;
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            SecretString::new(input.trim().to_string().into())
        }
    };

    let mut client = crate::commands::build_client_from_config(&config)?;

    let params = splunk_client::CreateUserParams {
        name: name.to_string(),
        password,
        roles,
        realname,
        email,
        default_app,
    };

    tokio::select! {
        res = client.create_user(&params) => {
            let user = res?;
            println!("User '{}' created successfully.", user.name);
            Ok(())
        }
        _ = cancel.cancelled() => Err(Cancelled.into()),
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_modify(
    config: splunk_config::Config,
    name: &str,
    password: Option<String>,
    roles: Option<Vec<String>>,
    realname: Option<String>,
    email: Option<String>,
    default_app: Option<String>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Modifying user: {}", name);

    let mut client = crate::commands::build_client_from_config(&config)?;

    let params = splunk_client::ModifyUserParams {
        password: password.map(|p| SecretString::new(p.into())),
        roles,
        realname,
        email,
        default_app,
    };

    tokio::select! {
        res = client.modify_user(name, &params) => {
            let user = res?;
            println!("User '{}' modified successfully.", user.name);
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
        print!("Are you sure you want to delete user '{}'? [y/N] ", name);
        use std::io::Write;
        std::io::stdout().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Delete cancelled.");
            return Ok(());
        }
    }

    info!("Deleting user: {}", name);

    let mut client = crate::commands::build_client_from_config(&config)?;

    tokio::select! {
        res = client.delete_user(name) => {
            res?;
            println!("User '{}' deleted successfully.", name);
            Ok(())
        }
        _ = cancel.cancelled() => Err(Cancelled.into()),
    }
}
