//! Command dispatch logic.
//!
//! Responsibilities:
//! - Route parsed CLI arguments to appropriate command handlers.
//! - Extract and validate configuration for each command.
//! - Handle command execution with cancellation support.
//!
//! Non-responsibilities:
//! - Does not define CLI structure (see `args` module).
//! - Does not load configuration (see `main()` and `config_context`).

use anyhow::Result;

use crate::args::{Cli, Commands};
use crate::cancellation::CancellationToken;
use crate::commands;
use crate::config_context::ConfigCommandContext;

/// Dispatch CLI commands to their respective handlers.
///
/// This function routes the parsed CLI arguments to the appropriate command
/// module based on the subcommand variant. It handles configuration extraction
/// and passes the cancellation token to support graceful shutdown.
///
/// # Arguments
/// * `cli` - The parsed CLI arguments
/// * `config` - The configuration context (real or placeholder)
/// * `cancel_token` - Token for cancellation support
///
/// # Returns
/// Result indicating success or failure of the command execution
pub(crate) async fn run_command(
    cli: Cli,
    config: ConfigCommandContext,
    cancel_token: &CancellationToken,
) -> Result<()> {
    match cli.command {
        Commands::Config { command } => {
            // Config commands don't use the config parameter - they use ConfigManager directly
            // The config context is ignored here (can be Real or Placeholder)
            commands::config::run(
                command,
                &cli.output,
                cli.output_file.clone(),
                cli.config_path.clone(),
            )?;
        }
        Commands::Search {
            query,
            wait,
            earliest,
            latest,
            count,
            realtime,
            realtime_window,
        } => {
            let (config, search_defaults) = config.into_real_config_with_search_defaults()?;
            commands::search::run(
                config,
                query,
                wait,
                earliest.as_deref(),
                latest.as_deref(),
                count,
                &search_defaults,
                &cli.output,
                cli.quiet,
                cli.output_file.clone(),
                cancel_token,
                realtime,
                realtime_window,
            )
            .await?;
        }
        Commands::Indexes { command } => {
            let config = config.into_real_config()?;
            commands::indexes::run(
                config,
                command,
                &cli.output,
                cli.output_file.clone(),
                cancel_token,
            )
            .await?;
        }
        Commands::Forwarders {
            detailed,
            count,
            offset,
        } => {
            let config = config.into_real_config()?;
            commands::forwarders::run(
                config,
                detailed,
                count,
                offset,
                &cli.output,
                cli.output_file.clone(),
                cancel_token,
            )
            .await?;
        }
        Commands::SearchPeers {
            detailed,
            count,
            offset,
        } => {
            let config = config.into_real_config()?;
            commands::search_peers::run(
                config,
                detailed,
                count,
                offset,
                &cli.output,
                cli.output_file.clone(),
                cancel_token,
            )
            .await?;
        }
        Commands::Cluster {
            command,
            detailed,
            offset,
            page_size,
        } => {
            let config = config.into_real_config()?;
            // Handle backward compatibility: if no subcommand but old flags are used, use Show
            let cmd = match command {
                Some(cmd) => cmd,
                None => commands::cluster::ClusterCommand::Show {
                    detailed,
                    offset,
                    page_size,
                },
            };
            commands::cluster::run(
                config,
                cmd,
                &cli.output,
                cli.output_file.clone(),
                cancel_token,
            )
            .await?;
        }
        Commands::Jobs {
            list,
            inspect,
            cancel,
            delete,
            count,
        } => {
            let config = config.into_real_config()?;
            commands::jobs::run(
                config,
                list,
                inspect,
                cancel,
                delete,
                count,
                &cli.output,
                cli.quiet,
                cli.output_file.clone(),
                cancel_token,
            )
            .await?;
        }
        Commands::Health => {
            let config = config.into_real_config()?;
            commands::health::run(config, &cli.output, cli.output_file.clone(), cancel_token)
                .await?;
        }
        Commands::Kvstore { command } => {
            let config = config.into_real_config()?;
            commands::kvstore::run(
                config,
                command,
                &cli.output,
                cli.output_file.clone(),
                cancel_token,
            )
            .await?;
        }
        Commands::License { command } => {
            let config = config.into_real_config()?;
            // Default to "show" if no subcommand is provided
            let cmd = command.unwrap_or(commands::license::LicenseCommand::Show);
            commands::license::run(
                config,
                cmd,
                &cli.output,
                cli.output_file.clone(),
                cancel_token,
            )
            .await?;
        }
        Commands::Logs {
            count,
            earliest,
            tail,
        } => {
            let config = config.into_real_config()?;
            commands::logs::run(
                config,
                count,
                earliest,
                tail,
                &cli.output,
                cli.output_file.clone(),
                cancel_token,
            )
            .await?;
        }
        Commands::Users { command } => {
            let config = config.into_real_config()?;
            commands::users::run(
                config,
                command,
                &cli.output,
                cli.output_file.clone(),
                cancel_token,
            )
            .await?;
        }
        Commands::Roles { command } => {
            let config = config.into_real_config()?;
            commands::roles::run(
                config,
                command,
                &cli.output,
                cli.output_file.clone(),
                cancel_token,
            )
            .await?;
        }
        Commands::Apps { apps_command } => {
            let config = config.into_real_config()?;
            commands::apps::run(
                config,
                apps_command,
                &cli.output,
                cli.output_file.clone(),
                cancel_token,
            )
            .await?;
        }
        Commands::ListAll {
            resources,
            profiles,
            all_profiles,
        } => {
            // Load ConfigManager if multi-profile mode
            let config_manager = if all_profiles || profiles.is_some() {
                // Use custom config path if provided via CLI arg or env var (already resolved)
                if let Some(config_path) = &cli.config_path {
                    Some(splunk_config::ConfigManager::new_with_path(
                        config_path.clone(),
                    )?)
                } else {
                    Some(splunk_config::ConfigManager::new()?)
                }
            } else {
                None
            };

            // Determine which profiles to query
            let is_multi_profile = all_profiles || profiles.is_some();

            // Only extract real config for single-profile mode
            let config = if is_multi_profile {
                // Multi-profile mode doesn't use the config parameter
                // (it loads configs from ConfigManager)
                None
            } else {
                Some(config.into_real_config()?)
            };

            // Unwrap config for single-profile mode, use placeholder for multi-profile
            let config_for_list_all = config.unwrap_or(splunk_config::Config {
                connection: splunk_config::ConnectionConfig {
                    base_url: String::new(),
                    skip_verify: false,
                    timeout: std::time::Duration::from_secs(30),
                    max_retries: 3,
                    session_expiry_buffer_seconds: 60,
                    session_ttl_seconds: 3600,
                    health_check_interval_seconds: 60,
                },
                auth: splunk_config::AuthConfig {
                    strategy: splunk_config::types::AuthStrategy::SessionToken {
                        username: String::new(),
                        password: secrecy::SecretString::new(String::new().into()),
                    },
                },
            });

            commands::list_all::run(
                config_for_list_all,
                resources,
                profiles,
                all_profiles,
                config_manager,
                &cli.output,
                cli.output_file.clone(),
                cancel_token,
            )
            .await?;
        }
        Commands::SavedSearches { command } => {
            let config = config.into_real_config()?;
            commands::saved_searches::run(
                config,
                command,
                &cli.output,
                cli.output_file.clone(),
                cancel_token,
            )
            .await?;
        }
        Commands::Macros { command } => {
            let config = config.into_real_config()?;
            commands::macros::run(
                config,
                command,
                &cli.output,
                cli.output_file.clone(),
                cancel_token,
            )
            .await?;
        }
        Commands::Inputs { command } => {
            let config = config.into_real_config()?;
            commands::inputs::run(
                config,
                command,
                &cli.output,
                cli.output_file.clone(),
                cancel_token,
            )
            .await?;
        }
        Commands::Configs { command } => {
            let config = config.into_real_config()?;
            commands::configs::run(
                config,
                command,
                &cli.output,
                cli.output_file.clone(),
                cancel_token,
            )
            .await?;
        }
        Commands::Alerts { command } => {
            let config = config.into_real_config()?;
            commands::alerts::run(
                config,
                command,
                &cli.output,
                cli.output_file.clone(),
                cancel_token,
            )
            .await?;
        }
        Commands::Audit { command } => {
            let config = config.into_real_config()?;
            commands::audit::run(
                config,
                command,
                &cli.output,
                cli.output_file.clone(),
                cancel_token,
            )
            .await?;
        }
        Commands::Dashboards { command } => {
            let config = config.into_real_config()?;
            commands::dashboards::run(
                config,
                command,
                &cli.output,
                cli.output_file.clone(),
                cancel_token,
            )
            .await?;
        }
        Commands::Datamodels { command } => {
            let config = config.into_real_config()?;
            commands::datamodels::run(
                config,
                command,
                &cli.output,
                cli.output_file.clone(),
                cancel_token,
            )
            .await?;
        }
        Commands::Lookups { count, offset } => {
            let config = config.into_real_config()?;
            commands::lookups::run(
                config,
                count,
                offset,
                &cli.output,
                cli.output_file.clone(),
                cancel_token,
            )
            .await?;
        }
        Commands::Workload {
            detailed,
            count,
            offset,
        } => {
            let config = config.into_real_config()?;
            commands::workload::run(
                config,
                detailed,
                count,
                offset,
                &cli.output,
                cli.output_file.clone(),
                cancel_token,
            )
            .await?;
        }
        Commands::Hec { command } => {
            // HEC commands don't use the standard config - they use HEC-specific URL/token
            commands::hec::run(command, &cli.output, cli.output_file.clone(), cancel_token).await?;
        }
        Commands::Shc {
            command,
            detailed,
            offset,
            page_size,
        } => {
            let config = config.into_real_config()?;
            // Handle backward compatibility: if no subcommand but old flags are used, use Show
            let cmd = match command {
                Some(cmd) => cmd,
                None => commands::shc::ShcCommand::Show {
                    detailed,
                    offset,
                    page_size,
                },
            };
            commands::shc::run(
                config,
                cmd,
                &cli.output,
                cli.output_file.clone(),
                cancel_token,
            )
            .await?;
        }
    }

    Ok(())
}
