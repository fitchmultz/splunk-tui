//! Profile file loading for configuration.
//!
//! Responsibilities:
//! - Load configuration from JSON profile files.
//! - Apply profile settings to a ConfigLoader instance.
//! - Handle profile file path resolution and migration from legacy paths.
//!
//! Does NOT handle:
//! - Environment variable parsing (see env.rs).
//! - Building the final Config (see builder.rs).
//! - Persisting configuration changes (see persistence.rs).
//!
//! Invariants:
//! - Profile settings are applied before environment variables (env vars take precedence).
//! - Missing profiles are recorded for later error handling in build().
//! - Config file migration is attempted only when using the default config path.

use super::builder::ConfigLoader;
use super::error::ConfigError;
use crate::encryption::MasterKeySource;
use crate::persistence::{
    ConfigManager, default_config_path, legacy_config_path, migrate_config_file_if_needed,
};
use crate::types::ProfileConfig;

/// Apply profile configuration from a profile file to the loader.
///
/// If the profile is not found, this records the missing profile name
/// for later error handling in `build()`.
pub fn apply_profile(loader: &mut ConfigLoader) -> Result<(), ConfigError> {
    let profile_name = match loader.profile_name() {
        Some(name) => name.clone(),
        None => return Ok(()),
    };

    let config_path = if let Some(path) = loader.config_path() {
        path.clone()
    } else {
        default_config_path().map_err(|e| ConfigError::ConfigDirUnavailable(e.to_string()))?
    };

    // If we're using the default config path, attempt a best-effort migration from the
    // legacy path before checking existence. This prevents TUI startup failures when
    // users rely on profiles stored at the legacy location.
    if loader.config_path().is_none()
        && let Ok(legacy_path) = legacy_config_path()
    {
        migrate_config_file_if_needed(&legacy_path, &config_path);
    }

    if !config_path.exists() {
        loader.set_profile_missing(Some(profile_name));
        return Ok(());
    }

    let source = if let Some(pw) = loader.config_password() {
        MasterKeySource::Password(pw.clone())
    } else if let Some(var) = loader.config_key_var() {
        MasterKeySource::Env(var.clone())
    } else {
        MasterKeySource::Keyring
    };

    let manager = ConfigManager::new_with_path_and_source(config_path, source)
        .map_err(|e| ConfigError::DecryptionFailed(e.to_string()))?;

    let profile = match manager.list_profiles().get(&profile_name) {
        Some(p) => p,
        None => {
            loader.set_profile_missing(Some(profile_name));
            return Ok(());
        }
    };

    apply_profile_config(loader, profile)?;
    Ok(())
}

/// Apply profile configuration values to the loader.
fn apply_profile_config(
    loader: &mut ConfigLoader,
    profile: &ProfileConfig,
) -> Result<(), ConfigError> {
    if let Some(url) = &profile.base_url {
        loader.set_base_url(Some(url.clone()));
    }
    if let Some(username) = &profile.username {
        loader.set_username(Some(username.clone()));
    }
    if let Some(password) = &profile.password {
        loader.set_password(Some(password.resolve()?));
    }
    if let Some(token) = &profile.api_token {
        loader.set_api_token(Some(token.resolve()?));
    }
    if let Some(skip) = profile.skip_verify {
        loader.set_skip_verify(Some(skip));
    }
    if let Some(secs) = profile.timeout_seconds {
        loader.set_timeout(Some(std::time::Duration::from_secs(secs)));
    }
    if let Some(retries) = profile.max_retries {
        loader.set_max_retries(Some(retries));
    }
    if let Some(buffer) = profile.session_expiry_buffer_seconds {
        loader.set_session_expiry_buffer_seconds(Some(buffer));
    }
    if let Some(ttl) = profile.session_ttl_seconds {
        loader.set_session_ttl_seconds(Some(ttl));
    }
    if let Some(interval) = profile.health_check_interval_seconds {
        loader.set_health_check_interval_seconds(Some(interval));
    }
    Ok(())
}
