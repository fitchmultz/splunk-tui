//! Configuration management commands.
//!
//! Responsibilities:
//! - Provide CLI commands for listing, showing, setting, and deleting profiles.
//! - Facilitate manual configuration of Splunk connection details.
//! - Handle secure credential storage via keyring integration.
//!
//! Does NOT handle:
//! - Automated configuration loading for other commands (see `splunk_config`).
//! - TUI configuration persistence (shared via `splunk_config::persistence`).
//!
//! Invariants / Assumptions:
//! - Commands assume a valid terminal for user interaction (especially for password prompts).
//! - Keyring operations require a supported system keyring service.

use crate::formatters::{OutputFormat, get_formatter, write_to_file};
use anyhow::{Context, Result};
use clap::Subcommand;
use serde::Serialize;
use splunk_config::persistence::ConfigManager;
use splunk_config::types::{ProfileConfig, SecureValue};

#[derive(Subcommand)]
pub enum ConfigCommand {
    /// List all configured profiles
    List {
        /// Output format (json, table)
        #[arg(short, long, default_value = "json")]
        output: String,
    },

    /// Set or update a profile
    Set {
        /// Profile name
        profile_name: String,

        /// Base URL of the Splunk server
        #[arg(short, long)]
        base_url: Option<String>,

        /// Username for session authentication
        #[arg(short, long)]
        username: Option<String>,

        /// Password for session authentication
        #[arg(short, long)]
        password: Option<String>,

        /// API token for bearer authentication
        #[arg(short, long)]
        api_token: Option<String>,

        /// Skip TLS certificate verification
        #[arg(short, long)]
        skip_verify: Option<bool>,

        /// Connection timeout in seconds
        #[arg(short, long)]
        timeout_seconds: Option<u64>,

        /// Maximum number of retries for failed requests
        #[arg(short, long)]
        max_retries: Option<usize>,

        /// Store password/token in system keyring
        #[arg(long)]
        use_keyring: bool,
    },

    /// Show a profile's configuration
    Show {
        /// Profile name to display
        profile_name: String,

        /// Output format (json, table, csv, xml)
        #[arg(short, long, default_value = "table")]
        output: String,
    },

    /// Delete a profile
    Delete {
        /// Profile name to delete
        profile_name: String,
    },

    /// Edit a profile interactively
    Edit {
        /// Profile name to edit
        profile_name: String,

        /// Store credentials in system keyring
        #[arg(long)]
        use_keyring: bool,
    },
}

pub fn run(command: ConfigCommand, output_file: Option<std::path::PathBuf>) -> Result<()> {
    let mut manager = if let Ok(config_path) = std::env::var("SPLUNK_CONFIG_PATH") {
        if !config_path.is_empty() {
            ConfigManager::new_with_path(std::path::PathBuf::from(config_path))?
        } else {
            ConfigManager::new()?
        }
    } else {
        ConfigManager::new()?
    };

    match command {
        ConfigCommand::List { output } => {
            run_list(&manager, &output, output_file.clone())?;
        }
        ConfigCommand::Set {
            profile_name,
            base_url,
            username,
            password,
            api_token,
            skip_verify,
            timeout_seconds,
            max_retries,
            use_keyring,
        } => {
            run_set(
                &mut manager,
                &profile_name,
                base_url,
                username,
                password,
                api_token,
                skip_verify,
                timeout_seconds,
                max_retries,
                use_keyring,
            )?;
        }
        ConfigCommand::Show {
            profile_name,
            output,
        } => {
            run_show(&manager, &profile_name, &output, output_file.clone())?;
        }
        ConfigCommand::Edit {
            profile_name,
            use_keyring,
        } => {
            run_edit(&mut manager, &profile_name, use_keyring)?;
        }
        ConfigCommand::Delete { profile_name } => {
            run_delete(&mut manager, &profile_name)?;
        }
    }

    Ok(())
}

fn run_list(
    manager: &ConfigManager,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
) -> Result<()> {
    let profiles = manager.list_profiles();

    // Validate output format before checking for empty profiles
    match output_format {
        "json" | "table" => {}
        _ => {
            anyhow::bail!(
                "Invalid output format '{}'. Valid values are 'json' or 'table'",
                output_format
            );
        }
    }

    if profiles.is_empty() {
        println!("No profiles configured. Use 'splunk-cli config set <profile-name>' to add one.");
        return Ok(());
    }

    match output_format {
        "json" => {
            #[derive(Serialize)]
            struct Output {
                profiles: std::collections::BTreeMap<String, ProfileDisplay>,
            }

            #[derive(Serialize)]
            struct ProfileDisplay {
                base_url: Option<String>,
                username: Option<String>,
                skip_verify: Option<bool>,
                timeout_seconds: Option<u64>,
                max_retries: Option<usize>,
                password: Option<String>,
                api_token: Option<String>,
            }

            let display_profiles: std::collections::BTreeMap<String, ProfileDisplay> = profiles
                .iter()
                .map(|(name, profile): (&String, &ProfileConfig)| {
                    (
                        name.clone(),
                        ProfileDisplay {
                            base_url: profile.base_url.clone(),
                            username: profile.username.clone(),
                            skip_verify: profile.skip_verify,
                            timeout_seconds: profile.timeout_seconds,
                            max_retries: profile.max_retries,
                            password: profile.password.as_ref().map(|_| "****".to_string()),
                            api_token: profile.api_token.as_ref().map(|_| "****".to_string()),
                        },
                    )
                })
                .collect();

            let output = Output {
                profiles: display_profiles,
            };
            let formatted = serde_json::to_string_pretty(&output)?;
            if let Some(ref path) = output_file {
                let format = OutputFormat::Json;
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
        }
        "table" => {
            let mut table_output =
                format!("{:<20} {:<40} {:<15}\n", "Profile", "Base URL", "Username");
            table_output.push_str(&format!("{}\n", "-".repeat(75)));

            for (name, profile) in profiles {
                let base_url = profile.base_url.as_deref().unwrap_or("-");
                let username = profile.username.as_deref().unwrap_or("-");
                table_output.push_str(&format!("{:<20} {:<40} {:<15}\n", name, base_url, username));
            }

            if let Some(ref path) = output_file {
                let format = OutputFormat::Table;
                write_to_file(&table_output, path)
                    .with_context(|| format!("Failed to write output to {}", path.display()))?;
                eprintln!(
                    "Results written to {} ({:?} format)",
                    path.display(),
                    format
                );
            } else {
                print!("{}", table_output);
            }
        }
        _ => {
            anyhow::bail!(
                "Invalid output format '{}'. Valid values are 'json' or 'table'",
                output_format
            );
        }
    }

    Ok(())
}

/// Prompts for a secret value (password or token) interactively.
///
/// If `existing_value` is `Some`, returns the existing value without prompting.
/// Otherwise, prompts the user for input.
fn prompt_for_secret(
    prompt_text: &str,
    existing_value: &Option<SecureValue>,
) -> Option<SecureValue> {
    if existing_value.is_some() {
        existing_value.clone()
    } else if let Ok(input) = dialoguer::Password::new()
        .with_prompt(prompt_text)
        .allow_empty_password(false)
        .interact()
    {
        Some(SecureValue::Plain(secrecy::SecretString::new(input.into())))
    } else {
        None
    }
}

#[allow(clippy::too_many_arguments)]
fn run_set(
    manager: &mut ConfigManager,
    profile_name: &str,
    base_url: Option<String>,
    username: Option<String>,
    password: Option<String>,
    api_token: Option<String>,
    skip_verify: Option<bool>,
    timeout_seconds: Option<u64>,
    max_retries: Option<usize>,
    use_keyring: bool,
) -> Result<()> {
    // Load existing profile if it exists
    let existing_profile = manager.list_profiles().get(profile_name).cloned();

    let profile = existing_profile.unwrap_or_default();

    let base_url = base_url.or(profile.base_url);
    let username = username.or(profile.username.clone());

    // Filter out empty strings from CLI args - treat as None
    let password = password.filter(|s| !s.is_empty());
    let api_token = api_token.filter(|s| !s.is_empty());

    // Validate that we have required fields
    if base_url.is_none() {
        anyhow::bail!("Base URL is required. Use --base-url to specify the Splunk server URL");
    }

    // Validate auth requirements BEFORE prompting to avoid interactive prompts in test environments
    // Check if we have auth from CLI args or existing profile
    let has_password = password.is_some() || profile.password.is_some();
    let has_token = api_token.is_some() || profile.api_token.is_some();

    // If username is set (CLI or profile), password or token must also be set
    if (username.is_some() || profile.username.is_some()) && !has_password && !has_token {
        anyhow::bail!(
            "Either --password or --api-token must be provided when using username. Use one for authentication"
        );
    }

    // Resolve password: CLI arg → existing profile → interactive prompt
    let password = password
        .map(|pw| SecureValue::Plain(secrecy::SecretString::new(pw.into())))
        .or_else(|| profile.password.clone())
        .or_else(|| prompt_for_secret("Password", &None));

    // Resolve API token: CLI arg → existing profile → interactive prompt
    let api_token = api_token
        .map(|token| SecureValue::Plain(secrecy::SecretString::new(token.into())))
        .or_else(|| profile.api_token.clone())
        .or_else(|| prompt_for_secret("API Token", &None));

    let mut profile_config = ProfileConfig {
        base_url,
        username: username.clone(),
        skip_verify: skip_verify.or(profile.skip_verify),
        timeout_seconds: timeout_seconds.or(profile.timeout_seconds),
        max_retries: max_retries.or(profile.max_retries),
        ..Default::default()
    };

    // If use_keyring flag is set, store credentials in keyring BEFORE first save
    // This ensures plaintext secrets are never written to disk
    let password = if use_keyring {
        if let (Some(username), Some(SecureValue::Plain(pw))) = (&username, &password) {
            Some(manager.store_password_in_keyring(profile_name, username, pw)?)
        } else {
            password
        }
    } else {
        password
    };

    let api_token = if use_keyring {
        if let Some(SecureValue::Plain(token)) = &api_token {
            Some(manager.store_token_in_keyring(profile_name, token)?)
        } else {
            api_token
        }
    } else {
        api_token
    };

    profile_config.password = password.or(profile.password);
    profile_config.api_token = api_token.or(profile.api_token);

    manager.save_profile(profile_name, profile_config)?;
    println!("Profile '{}' saved successfully.", profile_name);

    Ok(())
}

fn run_show(
    manager: &ConfigManager,
    profile_name: &str,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
) -> Result<()> {
    let profiles = manager.list_profiles();

    let profile = profiles
        .get(profile_name)
        .ok_or_else(|| anyhow::anyhow!("Profile '{}' not found", profile_name))?;

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);

    let output = formatter.format_profile(profile_name, profile)?;
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

fn run_edit(manager: &mut ConfigManager, profile_name: &str, use_keyring: bool) -> Result<()> {
    let profiles = manager.list_profiles();

    // Check if profile exists
    let existing_profile = profiles.get(profile_name).ok_or_else(|| {
        anyhow::anyhow!(
            "Profile '{}' not found. Use 'config set' to create a new profile.",
            profile_name
        )
    })?;

    let profile = existing_profile.clone();

    // Prompt for each field interactively
    let base_url = if let Some(current) = &profile.base_url {
        dialoguer::Input::<String>::new()
            .with_prompt("Base URL")
            .default(current.clone())
            .interact()
            .ok()
    } else {
        dialoguer::Input::<String>::new()
            .with_prompt("Base URL")
            .interact()
            .ok()
    };

    let username = if let Some(current) = &profile.username {
        dialoguer::Input::<String>::new()
            .with_prompt("Username")
            .default(current.clone())
            .allow_empty(true)
            .interact()
            .ok()
            .filter(|s| !s.is_empty())
    } else {
        dialoguer::Input::<String>::new()
            .with_prompt("Username")
            .allow_empty(true)
            .interact()
            .ok()
            .filter(|s| !s.is_empty())
    };

    // Validate that we have required fields
    if base_url.is_none() {
        anyhow::bail!("Base URL is required");
    }

    // Prompt for password if username is set
    let password = if username.is_some() {
        // For edit, always prompt but allow keeping existing
        if let Ok(input) = dialoguer::Password::new()
            .with_prompt("Password (press Enter to keep existing)")
            .allow_empty_password(true)
            .interact()
        {
            if input.is_empty() {
                profile.password.clone()
            } else {
                Some(SecureValue::Plain(secrecy::SecretString::new(input.into())))
            }
        } else {
            profile.password.clone()
        }
    } else {
        profile.password.clone()
    };

    // Prompt for API token (alternative to password)
    let api_token = if let Ok(input) = dialoguer::Password::new()
        .with_prompt("API Token (press Enter to keep existing or skip)")
        .allow_empty_password(true)
        .interact()
    {
        if input.is_empty() {
            profile.api_token.clone()
        } else {
            Some(SecureValue::Plain(secrecy::SecretString::new(input.into())))
        }
    } else {
        profile.api_token.clone()
    };

    // Validate auth requirements
    if username.is_some() && password.is_none() && api_token.is_none() {
        anyhow::bail!("Either password or API token must be provided when using username");
    }

    let skip_verify = if let Some(current) = profile.skip_verify {
        dialoguer::Confirm::new()
            .with_prompt("Skip TLS certificate verification?")
            .default(current)
            .interact()
            .ok()
    } else {
        dialoguer::Confirm::new()
            .with_prompt("Skip TLS certificate verification?")
            .default(false)
            .interact()
            .ok()
    };

    let timeout_seconds = if let Some(current) = profile.timeout_seconds {
        dialoguer::Input::<u64>::new()
            .with_prompt("Connection timeout (seconds)")
            .default(current)
            .interact()
            .ok()
    } else {
        dialoguer::Input::<u64>::new()
            .with_prompt("Connection timeout (seconds)")
            .default(30)
            .interact()
            .ok()
    };

    let max_retries = if let Some(current) = profile.max_retries {
        dialoguer::Input::<usize>::new()
            .with_prompt("Maximum retries")
            .default(current)
            .interact()
            .ok()
    } else {
        dialoguer::Input::<usize>::new()
            .with_prompt("Maximum retries")
            .default(3)
            .interact()
            .ok()
    };

    let mut profile_config = ProfileConfig {
        base_url,
        username: username.clone(),
        skip_verify,
        timeout_seconds,
        max_retries,
        ..Default::default()
    };

    // If use_keyring flag is set, store credentials in keyring BEFORE first save
    let password = if use_keyring {
        if let (Some(username), Some(SecureValue::Plain(pw))) = (&username, &password) {
            Some(manager.store_password_in_keyring(profile_name, username, pw)?)
        } else {
            password
        }
    } else {
        password
    };

    let api_token = if use_keyring {
        if let Some(SecureValue::Plain(token)) = &api_token {
            Some(manager.store_token_in_keyring(profile_name, token)?)
        } else {
            api_token
        }
    } else {
        api_token
    };

    profile_config.password = password.or(profile.password);
    profile_config.api_token = api_token.or(profile.api_token);

    manager.save_profile(profile_name, profile_config)?;
    println!("Profile '{}' updated successfully.", profile_name);

    Ok(())
}

fn run_delete(manager: &mut ConfigManager, profile_name: &str) -> Result<()> {
    manager.delete_profile(profile_name)?;
    println!("Profile '{}' deleted successfully.", profile_name);
    Ok(())
}
