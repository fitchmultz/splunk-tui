//! Command dispatch logic.
//!
//! Responsibilities:
//! - Route parsed CLI arguments to appropriate command handlers.
//! - Extract and validate configuration for each command.
//! - Handle command execution with cancellation support.
//!
//! Does NOT handle:
//! - CLI structure definitions (see `args` module).
//! - Configuration loading (see `main()` and `config_context`).
//!
//! Invariants:
//! - All commands receive a valid cancellation token
//! - Commands are routed based on the top-level Commands enum variant

use anyhow::Result;
use tracing::trace;

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
    trace!("Dispatching command");

    match cli.command {
        Commands::Config { command } => {
            trace!("Routing to config command");
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
            command,
            query,
            wait,
            earliest,
            latest,
            count,
            realtime,
            realtime_window,
        } => {
            trace!("Routing to search command");
            let (config, search_defaults) = config.into_real_config_with_search_defaults()?;

            // Handle subcommand or backward-compatible direct query
            match command {
                Some(commands::search::SearchCommand::Execute {
                    query,
                    wait,
                    earliest,
                    latest,
                    count,
                    realtime,
                    realtime_window,
                }) => {
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
                Some(commands::search::SearchCommand::Validate { query, file, json }) => {
                    // Determine output format: --json flag overrides global --output flag
                    let output_format = if json { "json" } else { &cli.output };

                    commands::search::run_validate(
                        config,
                        query,
                        file,
                        output_format,
                        cli.output_file.clone(),
                        cancel_token,
                    )
                    .await?;
                }
                None => {
                    // Backward compatibility: if no subcommand, check for legacy positional query
                    if let Some(query) = query {
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
                    } else {
                        anyhow::bail!(
                            "Failed to execute search: either provide a query or use a subcommand (execute, validate). See 'splunk-cli search --help' for more information."
                        );
                    }
                }
            }
        }
        Commands::Indexes { command } => {
            trace!("Routing to indexes command");
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
            trace!("Routing to forwarders command");
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
            trace!("Routing to search-peers command");
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
            count,
        } => {
            trace!("Routing to cluster command");
            let config = config.into_real_config()?;
            // Handle backward compatibility: if no subcommand but old flags are used, use Show
            let cmd = match command {
                Some(cmd) => cmd,
                None => commands::cluster::ClusterCommand::Show {
                    detailed,
                    offset,
                    count,
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
            command,
            list,
            inspect,
            cancel,
            delete,
            results,
            result_count,
            result_offset,
            count,
        } => {
            trace!("Routing to jobs command");
            let config = config.into_real_config()?;
            commands::jobs::run(
                config,
                list,
                inspect,
                cancel,
                delete,
                results,
                result_count,
                result_offset,
                count,
                &cli.output,
                cli.quiet,
                cli.output_file.clone(),
                command,
                cancel_token,
            )
            .await?;
        }
        Commands::Health => {
            trace!("Routing to health command");
            let config = config.into_real_config()?;
            commands::health::run(config, &cli.output, cli.output_file.clone(), cancel_token)
                .await?;
        }
        Commands::Doctor {
            bundle,
            include_logs,
        } => {
            trace!("Routing to doctor command");
            let config = config.into_real_config()?;
            commands::doctor::run(
                config,
                bundle,
                include_logs,
                &cli.output,
                cli.output_file.clone(),
                cancel_token,
            )
            .await?;
        }
        Commands::Kvstore { command } => {
            trace!("Routing to kvstore command");
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
            trace!("Routing to license command");
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
            trace!("Routing to logs command");
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
            trace!("Routing to users command");
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
            trace!("Routing to roles command");
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
            trace!("Routing to apps command");
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
            trace!("Routing to list-all command");
            // Determine mode: multi-profile uses ConfigManager, single-profile uses Config
            let is_multi_profile = all_profiles || profiles.is_some();

            if is_multi_profile {
                // Multi-profile mode: build ConfigManager and route to run_multi_profile
                // No Config is needed since each profile loads its own config
                let config_manager = if let Some(config_path) = &cli.config_path {
                    splunk_config::ConfigManager::new_with_path(config_path.clone())?
                } else {
                    splunk_config::ConfigManager::new()?
                };

                commands::list_all::run_multi_profile(
                    config_manager,
                    resources,
                    profiles,
                    all_profiles,
                    &cli.output,
                    cli.output_file.clone(),
                    cancel_token,
                )
                .await?;
            } else {
                // Single-profile mode: extract real config and route to run_single_profile
                let config = config.into_real_config()?;

                commands::list_all::run_single_profile(
                    config,
                    resources,
                    &cli.output,
                    cli.output_file.clone(),
                    cancel_token,
                )
                .await?;
            }
        }
        Commands::SavedSearches { command } => {
            trace!("Routing to saved-searches command");
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
            trace!("Routing to macros command");
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
            trace!("Routing to inputs command");
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
            trace!("Routing to configs command");
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
            trace!("Routing to alerts command");
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
            trace!("Routing to audit command");
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
            trace!("Routing to dashboards command");
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
            trace!("Routing to datamodels command");
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
        Commands::Lookups {
            command,
            count,
            offset,
        } => {
            trace!("Routing to lookups command");
            let config = config.into_real_config()?;
            commands::lookups::run(
                config,
                command,
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
            trace!("Routing to workload command");
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
            trace!("Routing to HEC command (no config required)");
            // HEC commands don't use the standard config - they use HEC-specific URL/token
            commands::hec::run(command, &cli.output, cli.output_file.clone(), cancel_token).await?;
        }
        Commands::Shc {
            command,
            detailed,
            offset,
            count,
        } => {
            trace!("Routing to SHC command");
            let config = config.into_real_config()?;
            // Handle backward compatibility: if no subcommand but old flags are used, use Show
            let cmd = match command {
                Some(cmd) => cmd,
                None => commands::shc::ShcCommand::Show {
                    detailed,
                    offset,
                    count,
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
        Commands::Completions {
            shell,
            dynamic,
            completion_cache_ttl,
        } => {
            trace!("Routing to completions command (no config required)");
            // Completions command doesn't need config - works offline
            commands::completions::run(shell, dynamic, completion_cache_ttl)?;
        }
        Commands::Complete {
            completion_type,
            cache_ttl,
        } => {
            trace!("Routing to complete command for dynamic completions");
            // Complete command may need config for server-based completions
            let config_result = config.into_real_config();
            let values = crate::dynamic_complete::generate_completions(
                completion_type,
                config_result.as_ref().ok(),
                Some(cache_ttl),
            )
            .await;

            // Print each value on its own line for shell parsing
            for value in values {
                println!("{}", value);
            }
        }
        Commands::Man => {
            trace!("Routing to manpage command (no config required)");
            // Manpage command doesn't need config - works offline
            commands::manpage::run()?;
        }
    }

    trace!("Command execution completed successfully");
    Ok(())
}
