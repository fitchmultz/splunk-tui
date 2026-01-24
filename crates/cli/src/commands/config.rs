//! Configuration management commands.

use anyhow::Result;
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

    /// Delete a profile
    Delete {
        /// Profile name to delete
        profile_name: String,
    },
}

pub fn run(command: ConfigCommand) -> Result<()> {
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
            run_list(&manager, &output)?;
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
        ConfigCommand::Delete { profile_name } => {
            run_delete(&mut manager, &profile_name)?;
        }
    }

    Ok(())
}

fn run_list(manager: &ConfigManager, output_format: &str) -> Result<()> {
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
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        "table" => {
            println!("{:<20} {:<40} {:<15}", "Profile", "Base URL", "Username");
            println!("{}", "-".repeat(75));

            for (name, profile) in profiles {
                let base_url = profile.base_url.as_deref().unwrap_or("-");
                let username = profile.username.as_deref().unwrap_or("-");
                println!("{:<20} {:<40} {:<15}", name, base_url, username);
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

fn run_delete(manager: &mut ConfigManager, profile_name: &str) -> Result<()> {
    manager.delete_profile(profile_name)?;
    println!("Profile '{}' deleted successfully.", profile_name);
    Ok(())
}
