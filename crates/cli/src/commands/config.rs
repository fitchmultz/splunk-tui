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
//! Invariants:
//! - Commands assume a valid terminal for user interaction (especially for password prompts).
//! - Keyring operations require a supported system keyring service.

use crate::formatters::{OutputFormat, get_formatter, write_to_file};
use anyhow::{Context, Result};
use clap::Subcommand;
use splunk_config::persistence::ConfigManager;
use splunk_config::types::{ProfileConfig, SecureValue};
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum ConfigCommand {
    /// List all configured profiles
    List,

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
        #[arg(short, long = "timeout")]
        timeout: Option<u64>,

        /// Maximum number of retries for failed requests
        #[arg(short, long)]
        max_retries: Option<usize>,

        /// Store credentials as plaintext instead of using system keyring
        #[arg(
            long,
            help = "Store credentials as plaintext instead of using system keyring"
        )]
        plaintext: bool,
    },

    /// Show a profile's configuration
    Show {
        /// Profile name to display
        profile_name: String,
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

        /// Store credentials as plaintext instead of using system keyring
        #[arg(
            long,
            help = "Store credentials as plaintext instead of using system keyring"
        )]
        plaintext: bool,
    },
}

pub fn run(
    command: ConfigCommand,
    output_format: &str,
    output_file: Option<PathBuf>,
    config_path: Option<PathBuf>,
) -> Result<()> {
    let mut manager = if let Some(path) = config_path {
        ConfigManager::new_with_path(path)?
    } else {
        ConfigManager::new()?
    };

    match command {
        ConfigCommand::List => {
            run_list(&manager, output_format, output_file.clone())?;
        }
        ConfigCommand::Set {
            profile_name,
            base_url,
            username,
            password,
            api_token,
            skip_verify,
            timeout,
            max_retries,
            plaintext,
        } => {
            run_set(
                &mut manager,
                &profile_name,
                base_url,
                username,
                password,
                api_token,
                skip_verify,
                timeout,
                max_retries,
                plaintext,
            )?;
        }
        ConfigCommand::Show { profile_name } => {
            run_show(&manager, &profile_name, output_format, output_file.clone())?;
        }
        ConfigCommand::Edit {
            profile_name,
            plaintext,
        } => {
            run_edit(&mut manager, &profile_name, plaintext)?;
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
    output_file: Option<PathBuf>,
) -> Result<()> {
    let profiles = manager.list_profiles();

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);

    let output = formatter.format_profiles(profiles)?;

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

/// Store password with optional keyring fallback.
fn store_password(
    manager: &ConfigManager,
    profile_name: &str,
    username: Option<&String>,
    password: Option<SecureValue>,
    plaintext: bool,
) -> Option<SecureValue> {
    if plaintext {
        return password;
    }

    if let (Some(u), Some(SecureValue::Plain(pw))) = (username, &password) {
        let keyring_value = manager.try_store_password_in_keyring(profile_name, u, pw);
        if matches!(keyring_value, SecureValue::Plain(_)) {
            eprintln!("Warning: Failed to store password in keyring. Storing as plaintext.");
        }
        return Some(keyring_value);
    }

    password
}

/// Store API token with optional keyring fallback.
fn store_api_token(
    manager: &ConfigManager,
    profile_name: &str,
    api_token: Option<SecureValue>,
    plaintext: bool,
) -> Option<SecureValue> {
    if plaintext {
        return api_token;
    }

    if let Some(SecureValue::Plain(token)) = &api_token {
        let keyring_value = manager.try_store_token_in_keyring(profile_name, token);
        if matches!(keyring_value, SecureValue::Plain(_)) {
            eprintln!("Warning: Failed to store API token in keyring. Storing as plaintext.");
        }
        return Some(keyring_value);
    }

    api_token
}

/// Validate that required fields are present for profile creation/update.
fn validate_profile_requirements(
    base_url: &Option<String>,
    username: &Option<String>,
    password: &Option<SecureValue>,
    api_token: &Option<SecureValue>,
    profile_username: &Option<String>,
    profile_password: &Option<SecureValue>,
    profile_token: &Option<SecureValue>,
) -> Result<()> {
    // Validate that we have required fields
    if base_url.is_none() {
        anyhow::bail!("Base URL is required. Use --base-url to specify the Splunk server URL");
    }

    // Validate auth requirements BEFORE prompting to avoid interactive prompts in test environments
    // Check if we have auth from CLI args or existing profile
    let has_password = password.is_some() || profile_password.is_some();
    let has_token = api_token.is_some() || profile_token.is_some();

    // If username is set (CLI or profile), password or token must also be set
    if (username.is_some() || profile_username.is_some()) && !has_password && !has_token {
        anyhow::bail!(
            "Either --password or --api-token must be provided when using username. Use one for authentication"
        );
    }

    Ok(())
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
    timeout: Option<u64>,
    max_retries: Option<usize>,
    plaintext: bool,
) -> Result<()> {
    // Load existing profile if it exists
    let existing_profile = manager.list_profiles().get(profile_name).cloned();
    let profile = existing_profile.clone().unwrap_or_default();

    // Filter out empty strings from CLI args - treat as None
    let password = password.filter(|s| !s.is_empty());
    let api_token = api_token.filter(|s| !s.is_empty());

    // Validate required fields
    validate_profile_requirements(
        &base_url,
        &username,
        &password
            .as_ref()
            .map(|p| SecureValue::Plain(secrecy::SecretString::new(p.clone().into()))),
        &api_token
            .as_ref()
            .map(|t| SecureValue::Plain(secrecy::SecretString::new(t.clone().into()))),
        &profile.username,
        &profile.password,
        &profile.api_token,
    )?;

    let resolved_base_url = base_url.or(profile.base_url.clone());
    let resolved_username = username.or(profile.username.clone());

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
        base_url: resolved_base_url,
        username: resolved_username.clone(),
        skip_verify: skip_verify.or(profile.skip_verify),
        timeout_seconds: timeout.or(profile.timeout_seconds),
        max_retries: max_retries.or(profile.max_retries),
        ..Default::default()
    };

    // Store credentials with keyring fallback
    let password = store_password(
        manager,
        profile_name,
        resolved_username.as_ref(),
        password,
        plaintext,
    );
    let api_token = store_api_token(manager, profile_name, api_token, plaintext);

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
    output_file: Option<PathBuf>,
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

/// Prompt for base URL, using existing value as default if present.
fn prompt_for_base_url(existing: Option<&String>) -> Result<Option<String>> {
    let input = dialoguer::Input::<String>::new();
    let input = input.with_prompt("Base URL");
    let input = if let Some(current) = existing {
        input.default(current.clone())
    } else {
        input
    };
    Ok(input.interact().ok())
}

/// Prompt for username, using existing value as default if present.
fn prompt_for_username(existing: Option<&String>) -> Result<Option<String>> {
    let input = dialoguer::Input::<String>::new();
    let input = input.with_prompt("Username").allow_empty(true);
    let input = if let Some(current) = existing {
        input.default(current.clone())
    } else {
        input
    };
    Ok(input.interact().ok().filter(|s| !s.is_empty()))
}

/// Prompt for password during edit, allowing keep existing.
fn prompt_for_password_edit(existing: Option<&SecureValue>) -> Result<Option<SecureValue>> {
    let input = dialoguer::Password::new()
        .with_prompt("Password (press Enter to keep existing)")
        .allow_empty_password(true)
        .interact()?;

    if input.is_empty() {
        Ok(existing.cloned())
    } else {
        Ok(Some(SecureValue::Plain(secrecy::SecretString::new(
            input.into(),
        ))))
    }
}

/// Prompt for API token during edit, allowing keep existing.
fn prompt_for_token_edit(existing: Option<&SecureValue>) -> Result<Option<SecureValue>> {
    let input = dialoguer::Password::new()
        .with_prompt("API Token (press Enter to keep existing or skip)")
        .allow_empty_password(true)
        .interact()?;

    if input.is_empty() {
        Ok(existing.cloned())
    } else {
        Ok(Some(SecureValue::Plain(secrecy::SecretString::new(
            input.into(),
        ))))
    }
}

/// Prompt for boolean confirmation with default.
fn prompt_for_bool(prompt: &str, existing: Option<bool>) -> Result<Option<bool>> {
    let confirm = dialoguer::Confirm::new();
    let confirm = confirm
        .with_prompt(prompt)
        .default(existing.unwrap_or(false));
    Ok(confirm.interact().ok())
}

/// Prompt for numeric input with default.
fn prompt_for_number<T>(prompt: &str, existing: Option<T>, default: T) -> Result<Option<T>>
where
    T: Clone + std::fmt::Display + std::str::FromStr,
    <T as std::str::FromStr>::Err: std::fmt::Display,
{
    let input = dialoguer::Input::<T>::new();
    let input = input
        .with_prompt(prompt)
        .default(existing.unwrap_or(default));
    Ok(input.interact().ok())
}

/// Gather all profile inputs interactively for edit command.
fn gather_edit_inputs(profile: &ProfileConfig) -> Result<ProfileConfig> {
    let base_url = prompt_for_base_url(profile.base_url.as_ref())?;
    let username = prompt_for_username(profile.username.as_ref())?;

    // Validate required fields
    if base_url.is_none() {
        anyhow::bail!("Base URL is required");
    }

    let password = if username.is_some() {
        prompt_for_password_edit(profile.password.as_ref())?
    } else {
        profile.password.clone()
    };

    let api_token = prompt_for_token_edit(profile.api_token.as_ref())?;

    // Validate auth requirements
    if username.is_some() && password.is_none() && api_token.is_none() {
        anyhow::bail!("Either password or API token must be provided when using username");
    }

    let skip_verify = prompt_for_bool("Skip TLS certificate verification?", profile.skip_verify)?;
    let timeout_seconds =
        prompt_for_number("Connection timeout (seconds)", profile.timeout_seconds, 30)?;
    let max_retries = prompt_for_number("Maximum retries", profile.max_retries, 3)?;

    Ok(ProfileConfig {
        base_url,
        username: username.clone(),
        skip_verify,
        timeout_seconds,
        max_retries,
        password,
        api_token,
        ..Default::default()
    })
}

fn run_edit(manager: &mut ConfigManager, profile_name: &str, plaintext: bool) -> Result<()> {
    let profiles = manager.list_profiles();

    // Check if profile exists
    let existing_profile = profiles.get(profile_name).ok_or_else(|| {
        anyhow::anyhow!(
            "Profile '{}' not found. Use 'config set' to create a new profile.",
            profile_name
        )
    })?;

    // Gather all inputs interactively
    let gathered = gather_edit_inputs(existing_profile)?;

    // Build final profile config, falling back to existing values
    let mut profile_config = ProfileConfig {
        base_url: gathered.base_url.clone(),
        username: gathered.username.clone(),
        skip_verify: gathered.skip_verify,
        timeout_seconds: gathered.timeout_seconds,
        max_retries: gathered.max_retries,
        ..Default::default()
    };

    // Store credentials with keyring fallback
    let password = store_password(
        manager,
        profile_name,
        gathered.username.as_ref(),
        gathered.password,
        plaintext,
    );
    let api_token = store_api_token(manager, profile_name, gathered.api_token, plaintext);

    profile_config.password = password.or(existing_profile.password.clone());
    profile_config.api_token = api_token.or(existing_profile.api_token.clone());

    manager.save_profile(profile_name, profile_config)?;
    println!("Profile '{}' updated successfully.", profile_name);

    Ok(())
}

fn run_delete(manager: &mut ConfigManager, profile_name: &str) -> Result<()> {
    manager.delete_profile(profile_name)?;
    println!("Profile '{}' deleted successfully.", profile_name);
    Ok(())
}
