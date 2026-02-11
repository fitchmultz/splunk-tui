//! Profile and keyring management for configuration persistence.
//!
//! Responsibilities:
//! - Manage configuration profiles.
//! - Handle keyring operations for passwords and tokens.
//! - Atomic save operations.
//! - Profile CRUD operations.
//!
//! Does NOT handle:
//! - Path determination (uses path module).
//! - Migration logic.
//! - State type definitions.
//!
//! Invariants:
//! - Profile names are unique within a config file.
//! - Writes are atomic (temp file + rename).
//! - Plain text secrets are never written when keyring is enabled.

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use secrecy::SecretString;

use crate::encryption::{Encryptor, MasterKeySource};
use crate::types::{KEYRING_SERVICE, ProfileConfig, SecureValue};

use super::create_corrupt_backup;

/// Error type for credential storage operations when keyring storage fails.
#[derive(Debug, thiserror::Error)]
pub enum CredentialStorageError {
    /// Failed to store password in keyring.
    #[error("Failed to store password in keyring for profile '{profile}': {source}")]
    PasswordStoreFailed {
        /// The profile name for which storage failed.
        profile: String,
        /// The underlying error.
        #[source]
        source: anyhow::Error,
    },
    /// Failed to store API token in keyring.
    #[error("Failed to store API token in keyring for profile '{profile}': {source}")]
    TokenStoreFailed {
        /// The profile name for which storage failed.
        profile: String,
        /// The underlying error.
        #[source]
        source: anyhow::Error,
    },
}
use super::migration::migrate_config_file_if_needed;
use super::path::{default_config_path, legacy_config_path};
use super::state::{ConfigFile, ConfigFileError, ConfigStorage, PersistedState, read_config_file};
use crate::env_var_or_none;

/// Manages loading and saving user configuration to disk.
pub struct ConfigManager {
    /// Path to the configuration file.
    config_path: PathBuf,
    /// Cached config file data (profiles + state).
    config_file: ConfigFile,
    /// Source for the master encryption key.
    master_key_source: MasterKeySource,
    /// Whether to encrypt the config file on save.
    use_encryption: bool,
}

impl ConfigManager {
    /// Creates a new `ConfigManager` using platform-standard config directories.
    ///
    /// If the `SPLUNK_CONFIG_PATH` environment variable is set (and not empty/whitespace),
    /// it will be used instead of the default path.
    ///
    /// On first run after upgrading from a legacy version, this will automatically migrate
    /// the config file from the legacy location to the documented location.
    ///
    /// # Errors
    /// Returns an error if `ProjectDirs::from` fails (should be rare).
    pub fn new() -> Result<Self> {
        Self::new_with_source(MasterKeySource::Keyring)
    }

    /// Creates a new `ConfigManager` using platform-standard config directories and a specific master key source.
    pub fn new_with_source(master_key_source: MasterKeySource) -> Result<Self> {
        // Check SPLUNK_CONFIG_PATH env var first (centralized handling)
        let config_path = if let Some(path_str) = env_var_or_none("SPLUNK_CONFIG_PATH") {
            PathBuf::from(path_str)
        } else {
            let default_path = default_config_path()?;

            // Safe, best-effort migration from legacy path to documented path.
            // Must never prevent startup.
            if let Ok(legacy_path) = legacy_config_path() {
                migrate_config_file_if_needed(&legacy_path, &default_path);
            }

            default_path
        };

        Self::new_with_path_and_source(config_path, master_key_source)
    }

    /// Creates a new `ConfigManager` with a specific config file path.
    ///
    /// If the config file exists but cannot be read or parsed (e.g., due to
    /// corruption or invalid JSON), the file is backed up with a `.corrupt.{timestamp}`
    /// extension and a default configuration is used instead. This prevents
    /// data loss while allowing the application to start.
    pub fn new_with_path(config_path: PathBuf) -> Result<Self> {
        Self::new_with_path_and_source(config_path, MasterKeySource::Keyring)
    }

    /// Creates a new `ConfigManager` with a specific config file path and master key source.
    pub fn new_with_path_and_source(
        config_path: PathBuf,
        master_key_source: MasterKeySource,
    ) -> Result<Self> {
        let no_migrate = std::env::var("SPLUNK_CONFIG_NO_MIGRATE")
            .map(|v| v == "1")
            .unwrap_or(false);
        let mut use_encryption = !no_migrate; // Default to true for auto-migration, unless disabled
        let config_file = if config_path.exists() {
            match read_config_file(&config_path) {
                Ok((storage, is_legacy)) => {
                    if matches!(storage, ConfigStorage::Encrypted { .. }) {
                        use_encryption = true;
                    } else if !is_legacy || no_migrate {
                        use_encryption = false;
                    }
                    match Self::decrypt_storage(storage, &master_key_source) {
                        Ok(file) => file,
                        Err(e) => {
                            tracing::error!(error = %e, "Failed to decrypt config file, using defaults");
                            ConfigFile::default()
                        }
                    }
                }
                Err(e) => {
                    // Check if this is a "file not found" error - if so, no backup needed
                    let is_not_found = matches!(
                        &e,
                        ConfigFileError::Read { source, .. } if source.kind() == std::io::ErrorKind::NotFound
                    );

                    if !is_not_found {
                        // File exists but is corrupt/unreadable - create backup
                        match create_corrupt_backup(&config_path) {
                            Ok(backup_path) => {
                                tracing::warn!(
                                    path = %config_path.display(),
                                    backup_path = %backup_path.display(),
                                    error = %e,
                                    "Config file is corrupt, backed up and using defaults"
                                );
                            }
                            Err(backup_err) => {
                                // Backup failed - log error but continue with defaults
                                tracing::error!(
                                    path = %config_path.display(),
                                    error = %e,
                                    backup_error = %backup_err,
                                    "Config file is corrupt and backup failed, using defaults"
                                );
                            }
                        }
                    } else {
                        // File not found - no backup needed
                        tracing::warn!(
                            path = %config_path.display(),
                            error = %e,
                            "Config file not found, using defaults"
                        );
                    }
                    ConfigFile::default()
                }
            }
        } else {
            if no_migrate {
                use_encryption = false;
            }
            ConfigFile::default()
        };

        Ok(Self {
            config_path,
            config_file,
            master_key_source,
            use_encryption,
        })
    }

    fn decrypt_storage(storage: ConfigStorage, source: &MasterKeySource) -> Result<ConfigFile> {
        match storage {
            ConfigStorage::Plain(file) => Ok(*file),
            ConfigStorage::Encrypted {
                kdf_salt,
                nonce,
                ciphertext,
            } => {
                let key = source
                    .resolve(kdf_salt.as_deref())
                    .map_err(|e| anyhow::anyhow!("Failed to resolve master key: {}", e))?;
                let nonce_bytes: [u8; 12] = nonce
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("Invalid nonce size"))?;
                let plaintext = Encryptor::decrypt(&ciphertext, &key, &nonce_bytes)
                    .map_err(|e| anyhow::anyhow!("Decryption failed: {}", e))?;
                let file: ConfigFile = serde_json::from_slice(&plaintext)
                    .context("Failed to parse decrypted config JSON")?;
                Ok(file)
            }
        }
    }

    /// Returns the path to the configuration file.
    pub fn config_path(&self) -> &PathBuf {
        &self.config_path
    }

    /// Loads persisted state from disk.
    ///
    /// Returns default state if the file doesn't exist or cannot be read.
    /// Invalid persisted values are sanitized to their defaults.
    pub fn load(&self) -> PersistedState {
        let mut state = self.config_file.state.clone().unwrap_or_default();
        // Sanitize search defaults to enforce invariants
        state.search_defaults = state.search_defaults.sanitize();
        state
    }

    /// Saves persisted state to disk.
    ///
    /// This preserves any existing profiles in the config file.
    ///
    /// # Errors
    /// Returns an error if the parent directory cannot be created
    /// or the file cannot be written.
    pub fn save(&mut self, state: &PersistedState) -> Result<()> {
        // Update the state while preserving profiles
        self.config_file.state = Some(state.clone());
        self.atomic_save()
    }

    /// Moves a profile's password to the system keyring.
    pub fn move_password_to_keyring(&mut self, profile_name: &str) -> Result<()> {
        let profile = self
            .config_file
            .profiles
            .get_mut(profile_name)
            .context(format!("Profile '{}' not found", profile_name))?;

        if let Some(SecureValue::Plain(password)) = &profile.password {
            use secrecy::ExposeSecret;
            let username = profile.username.as_deref().unwrap_or("unknown");
            let keyring_account = format!("{}-{}", profile_name, username);

            let entry = keyring::Entry::new(KEYRING_SERVICE, &keyring_account)?;
            entry.set_password(password.expose_secret())?;

            profile.password = Some(SecureValue::Keyring { keyring_account });
            self.atomic_save()?;
        }

        Ok(())
    }

    /// Moves a profile's API token to the system keyring.
    pub fn move_token_to_keyring(&mut self, profile_name: &str) -> Result<()> {
        let profile = self
            .config_file
            .profiles
            .get_mut(profile_name)
            .context(format!("Profile '{}' not found", profile_name))?;

        if let Some(SecureValue::Plain(token)) = &profile.api_token {
            use secrecy::ExposeSecret;
            let keyring_account = format!("{}-token", profile_name);

            let entry = keyring::Entry::new(KEYRING_SERVICE, &keyring_account)?;
            entry.set_password(token.expose_secret())?;

            profile.api_token = Some(SecureValue::Keyring { keyring_account });
            self.atomic_save()?;
        }

        Ok(())
    }

    /// Stores a password in the system keyring and returns the Keyring variant.
    ///
    /// This helper is used to store credentials in the keyring before saving a profile,
    /// ensuring that plain text secrets are never written to disk when keyring is enabled.
    pub fn store_password_in_keyring(
        &self,
        profile_name: &str,
        username: &str,
        password: &SecretString,
    ) -> Result<SecureValue> {
        use secrecy::ExposeSecret;
        let keyring_account = format!("{}-{}", profile_name, username);

        let entry = keyring::Entry::new(KEYRING_SERVICE, &keyring_account)?;
        entry.set_password(password.expose_secret())?;

        Ok(SecureValue::Keyring { keyring_account })
    }

    /// Stores an API token in the system keyring and returns the Keyring variant.
    ///
    /// This helper is used to store credentials in the keyring before saving a profile,
    /// ensuring that plain text secrets are never written to disk when keyring is enabled.
    pub fn store_token_in_keyring(
        &self,
        profile_name: &str,
        token: &SecretString,
    ) -> Result<SecureValue> {
        use secrecy::ExposeSecret;
        let keyring_account = format!("{}-token", profile_name);

        let entry = keyring::Entry::new(KEYRING_SERVICE, &keyring_account)?;
        entry.set_password(token.expose_secret())?;

        Ok(SecureValue::Keyring { keyring_account })
    }

    /// Attempts to store a password in the system keyring, returning plaintext on failure.
    ///
    /// This is the default behavior for credential storage - it tries the keyring first
    /// for security, but falls back to plaintext storage if the keyring is unavailable.
    /// A warning is logged when fallback occurs.
    pub fn try_store_password_in_keyring(
        &self,
        profile_name: &str,
        username: &str,
        password: &SecretString,
    ) -> SecureValue {
        match self.store_password_in_keyring(profile_name, username, password) {
            Ok(keyring_value) => keyring_value,
            Err(e) => {
                tracing::warn!(
                    "Failed to store password in keyring: {}. Storing as plaintext.",
                    e
                );
                SecureValue::Plain(password.clone())
            }
        }
    }

    /// Attempts to store an API token in the system keyring, returning plaintext on failure.
    ///
    /// This is the default behavior for credential storage - it tries the keyring first
    /// for security, but falls back to plaintext storage if the keyring is unavailable.
    /// A warning is logged when fallback occurs.
    pub fn try_store_token_in_keyring(
        &self,
        profile_name: &str,
        token: &SecretString,
    ) -> SecureValue {
        match self.store_token_in_keyring(profile_name, token) {
            Ok(keyring_value) => keyring_value,
            Err(e) => {
                tracing::warn!(
                    "Failed to store API token in keyring: {}. Storing as plaintext.",
                    e
                );
                SecureValue::Plain(token.clone())
            }
        }
    }

    /// Stores a password in the system keyring, returning an error on failure.
    ///
    /// This is the secure-by-default version that does NOT fall back to plaintext.
    /// Use this when you want to fail if keyring storage is unavailable.
    ///
    /// # Errors
    /// Returns `CredentialStorageError::PasswordStoreFailed` if keyring storage fails.
    pub fn strict_store_password_in_keyring(
        &self,
        profile_name: &str,
        username: &str,
        password: &SecretString,
    ) -> Result<SecureValue, CredentialStorageError> {
        self.store_password_in_keyring(profile_name, username, password)
            .map_err(|e| CredentialStorageError::PasswordStoreFailed {
                profile: profile_name.to_string(),
                source: e,
            })
    }

    /// Stores an API token in the system keyring, returning an error on failure.
    ///
    /// This is the secure-by-default version that does NOT fall back to plaintext.
    /// Use this when you want to fail if keyring storage is unavailable.
    ///
    /// # Errors
    /// Returns `CredentialStorageError::TokenStoreFailed` if keyring storage fails.
    pub fn strict_store_token_in_keyring(
        &self,
        profile_name: &str,
        token: &SecretString,
    ) -> Result<SecureValue, CredentialStorageError> {
        self.store_token_in_keyring(profile_name, token)
            .map_err(|e| CredentialStorageError::TokenStoreFailed {
                profile: profile_name.to_string(),
                source: e,
            })
    }

    /// Returns a reference to all configured profiles.
    pub fn list_profiles(&self) -> &BTreeMap<String, ProfileConfig> {
        &self.config_file.profiles
    }

    /// Enables encryption for the configuration file.
    pub fn enable_encryption(&mut self, source: MasterKeySource) -> Result<()> {
        self.master_key_source = source;
        self.use_encryption = true;
        self.atomic_save()
    }

    /// Disables encryption for the configuration file, storing it in plaintext.
    pub fn disable_encryption(&mut self) -> Result<()> {
        self.use_encryption = false;
        self.atomic_save()
    }

    /// Rotates the master encryption key.
    pub fn rotate_key(&mut self, new_source: MasterKeySource) -> Result<()> {
        self.master_key_source = new_source;
        self.use_encryption = true;
        self.atomic_save()
    }

    /// Saves or updates a profile configuration.
    ///
    /// This inserts a new profile or updates an existing one, then saves
    /// the configuration file atomically.
    pub fn save_profile(&mut self, name: &str, profile: ProfileConfig) -> Result<()> {
        self.config_file.profiles.insert(name.to_string(), profile);
        self.atomic_save()?;
        Ok(())
    }

    /// Deletes a profile configuration.
    ///
    /// Returns an error if the profile doesn't exist.
    /// Also cleans up any keyring entries associated with the profile.
    pub fn delete_profile(&mut self, name: &str) -> Result<()> {
        // Get profile before removing to check for keyring entries
        let profile = self
            .config_file
            .profiles
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Profile '{}' not found", name))?;

        // Clean up keyring entries if present (best effort - don't fail if keyring cleanup fails)
        if let Some(SecureValue::Keyring { keyring_account }) = &profile.password {
            let _ = keyring::Entry::new(KEYRING_SERVICE, keyring_account)
                .and_then(|e| e.delete_credential());
        }
        if let Some(SecureValue::Keyring { keyring_account }) = &profile.api_token {
            let _ = keyring::Entry::new(KEYRING_SERVICE, keyring_account)
                .and_then(|e| e.delete_credential());
        }

        // Remove from config
        self.config_file.profiles.remove(name);
        self.atomic_save()?;
        Ok(())
    }

    /// Atomically saves the current configuration to disk.
    ///
    /// Writes to a temporary file first, then renames it to the target path.
    /// This ensures the config file is never left in a partially written state.
    fn atomic_save(&self) -> Result<()> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create config directory")?;
        }

        let storage = if self.use_encryption {
            let (salt, key) = match &self.master_key_source {
                MasterKeySource::Password(pw) => {
                    let salt = Encryptor::generate_salt();
                    let key = Encryptor::derive_key(pw, &salt)
                        .map_err(|e| anyhow::anyhow!("Key derivation failed: {}", e))?;
                    (Some(salt.to_vec()), key)
                }
                source => {
                    let key = source
                        .resolve(None)
                        .map_err(|e| anyhow::anyhow!("Failed to resolve master key: {}", e))?;
                    (None, key)
                }
            };

            let plaintext = serde_json::to_vec(&self.config_file)?;
            let (ciphertext, nonce) = Encryptor::encrypt(&plaintext, &key)
                .map_err(|e| anyhow::anyhow!("Encryption failed: {}", e))?;

            ConfigStorage::Encrypted {
                kdf_salt: salt,
                nonce: nonce.to_vec(),
                ciphertext,
            }
        } else {
            ConfigStorage::Plain(Box::new(self.config_file.clone()))
        };

        // Write to a temporary file first
        let temp_path = self.config_path.with_extension("tmp");
        let content = serde_json::to_string_pretty(&storage)?;
        std::fs::write(&temp_path, content).context("Failed to write temporary config file")?;

        // Atomically rename the temporary file to the target path
        std::fs::rename(&temp_path, &self.config_path)
            .context("Failed to rename temporary config file")?;

        tracing::debug!(
            path = %self.config_path.display(),
            encryption = %self.use_encryption,
            "Config saved atomically"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::MIGRATION_DELAY_MS;
    use secrecy::SecretString;
    use serial_test::serial;
    use tempfile::NamedTempFile;

    #[test]
    fn test_save_preserves_profiles() {
        temp_env::with_var("SPLUNK_CONFIG_NO_MIGRATE", Some("1"), || {
            let temp_file = NamedTempFile::new().unwrap();
            let mut manager = ConfigManager::new_with_path(temp_file.path().to_path_buf()).unwrap();

            // Add a profile
            let password = SecretString::new("test-password".to_string().into());
            manager.config_file.profiles.insert(
                "test-profile".to_string(),
                ProfileConfig {
                    base_url: Some("https://splunk.example.com:8089".to_string()),
                    username: Some("admin".to_string()),
                    password: Some(SecureValue::Plain(password)),
                    api_token: None,
                    skip_verify: Some(true),
                    timeout_seconds: Some(60),
                    max_retries: Some(5),
                    session_expiry_buffer_seconds: Some(60),
                    session_ttl_seconds: Some(3600),
                    health_check_interval_seconds: None,
                },
            );

            // Save state
            let state = PersistedState {
                auto_refresh: true,
                ..Default::default()
            };
            manager.save(&state).unwrap();

            // Reload and verify profiles are preserved
            let reloaded = ConfigManager::new_with_path(temp_file.path().to_path_buf()).unwrap();
            assert!(reloaded.config_file.profiles.contains_key("test-profile"));
            assert_eq!(
                reloaded.config_file.profiles["test-profile"]
                    .base_url
                    .as_ref()
                    .unwrap(),
                "https://splunk.example.com:8089"
            );
        });
    }

    #[test]
    #[serial]
    fn test_move_password_to_keyring() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut manager = ConfigManager::new_with_path(temp_file.path().to_path_buf()).unwrap();

        let profile_name = "test-password-profile-unique";
        let password_str = "keyring-password";
        let password = SecretString::new(password_str.to_string().into());

        manager.config_file.profiles.insert(
            profile_name.to_string(),
            ProfileConfig {
                username: Some("admin".to_string()),
                password: Some(SecureValue::Plain(password)),
                ..Default::default()
            },
        );

        // Attempt to move to keyring. We handle errors gracefully in case the test environment
        // doesn't have a functional keyring backend.
        match manager.move_password_to_keyring(profile_name) {
            Ok(()) => {
                let profile = &manager.config_file.profiles[profile_name];
                assert!(matches!(
                    profile.password,
                    Some(SecureValue::Keyring { .. })
                ));

                // Verify it can be resolved
                match profile.password.as_ref().unwrap().resolve() {
                    Ok(resolved) => {
                        use secrecy::ExposeSecret;
                        assert_eq!(resolved.expose_secret(), password_str);
                    }
                    Err(e) => {
                        eprintln!("Skipping resolve check: {}", e);
                    }
                }

                // Clean up
                if let Some(SecureValue::Keyring { keyring_account }) = &profile.password {
                    let entry = keyring::Entry::new(KEYRING_SERVICE, keyring_account).unwrap();
                    let _ = entry.delete_credential();
                }
            }
            Err(e) => {
                eprintln!("Skipping keyring test: {}", e);
            }
        }
    }

    #[test]
    #[serial]
    fn test_move_token_to_keyring() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut manager = ConfigManager::new_with_path(temp_file.path().to_path_buf()).unwrap();

        let profile_name = "test-token-profile-unique";
        let token_str = "test-token-123";
        let token = SecretString::new(token_str.to_string().into());

        manager.config_file.profiles.insert(
            profile_name.to_string(),
            ProfileConfig {
                api_token: Some(SecureValue::Plain(token)),
                ..Default::default()
            },
        );

        match manager.move_token_to_keyring(profile_name) {
            Ok(()) => {
                let profile = &manager.config_file.profiles[profile_name];
                assert!(matches!(
                    profile.api_token,
                    Some(SecureValue::Keyring { .. })
                ));

                // Verify it can be resolved
                match profile.api_token.as_ref().unwrap().resolve() {
                    Ok(resolved) => {
                        use secrecy::ExposeSecret;
                        assert_eq!(resolved.expose_secret(), token_str);
                    }
                    Err(e) => {
                        eprintln!("Skipping token resolve check: {}", e);
                    }
                }

                // Clean up
                if let Some(SecureValue::Keyring { keyring_account }) = &profile.api_token {
                    let entry = keyring::Entry::new(KEYRING_SERVICE, keyring_account).unwrap();
                    let _ = entry.delete_credential();
                }
            }
            Err(e) => {
                eprintln!("Skipping keyring token test: {}", e);
            }
        }
    }

    #[test]
    fn test_list_profiles() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut manager = ConfigManager::new_with_path(temp_file.path().to_path_buf()).unwrap();

        let password = SecretString::new("password1".to_string().into());
        let token = SecretString::new("token1".to_string().into());

        manager.config_file.profiles.insert(
            "dev".to_string(),
            ProfileConfig {
                base_url: Some("https://dev.splunk.com:8089".to_string()),
                username: Some("dev_user".to_string()),
                password: Some(SecureValue::Plain(password)),
                ..Default::default()
            },
        );

        manager.config_file.profiles.insert(
            "prod".to_string(),
            ProfileConfig {
                base_url: Some("https://prod.splunk.com:8089".to_string()),
                api_token: Some(SecureValue::Plain(token)),
                ..Default::default()
            },
        );

        let profiles = manager.list_profiles();
        assert_eq!(profiles.len(), 2);
        assert!(profiles.contains_key("dev"));
        assert!(profiles.contains_key("prod"));
    }

    #[test]
    fn test_save_profile() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut manager = ConfigManager::new_with_path(temp_file.path().to_path_buf()).unwrap();

        let profile = ProfileConfig {
            base_url: Some("https://test.splunk.com:8089".to_string()),
            username: Some("test_user".to_string()),
            skip_verify: Some(true),
            timeout_seconds: Some(30),
            max_retries: Some(3),
            ..Default::default()
        };

        manager.save_profile("test-profile", profile).unwrap();

        let profiles = manager.list_profiles();
        assert_eq!(profiles.len(), 1);
        assert!(profiles.contains_key("test-profile"));
        assert_eq!(
            profiles["test-profile"].base_url,
            Some("https://test.splunk.com:8089".to_string())
        );
    }

    #[test]
    fn test_save_profile_updates_existing() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut manager = ConfigManager::new_with_path(temp_file.path().to_path_buf()).unwrap();

        let profile1 = ProfileConfig {
            base_url: Some("https://old.splunk.com:8089".to_string()),
            username: Some("old_user".to_string()),
            ..Default::default()
        };

        manager.save_profile("test-profile", profile1).unwrap();

        let profile2 = ProfileConfig {
            base_url: Some("https://new.splunk.com:8089".to_string()),
            username: Some("new_user".to_string()),
            skip_verify: Some(true),
            ..Default::default()
        };

        manager.save_profile("test-profile", profile2).unwrap();

        let profiles = manager.list_profiles();
        assert_eq!(profiles.len(), 1);
        assert_eq!(
            profiles["test-profile"].base_url,
            Some("https://new.splunk.com:8089".to_string())
        );
        assert_eq!(
            profiles["test-profile"].username,
            Some("new_user".to_string())
        );
    }

    #[test]
    fn test_delete_profile() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut manager = ConfigManager::new_with_path(temp_file.path().to_path_buf()).unwrap();

        let profile = ProfileConfig {
            base_url: Some("https://test.splunk.com:8089".to_string()),
            ..Default::default()
        };

        manager.save_profile("to-delete", profile).unwrap();
        assert_eq!(manager.list_profiles().len(), 1);

        manager.delete_profile("to-delete").unwrap();
        assert_eq!(manager.list_profiles().len(), 0);
    }

    #[test]
    fn test_delete_nonexistent_profile() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut manager = ConfigManager::new_with_path(temp_file.path().to_path_buf()).unwrap();

        let result = manager.delete_profile("nonexistent");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Profile 'nonexistent' not found")
        );
    }

    #[test]
    #[serial]
    fn test_try_store_password_in_keyring_fallback() {
        let temp_file = NamedTempFile::new().unwrap();
        let manager = ConfigManager::new_with_path(temp_file.path().to_path_buf()).unwrap();

        let profile_name = "test-try-password-fallback";
        let username = "admin";
        let password_str = "test-password";
        let password = SecretString::new(password_str.to_string().into());

        // This should try keyring and fall back to plaintext if keyring fails
        let result = manager.try_store_password_in_keyring(profile_name, username, &password);

        // Result should be either Keyring (if keyring succeeded) or Plain (if it failed/fell back)
        // Both are valid - the important thing is that the password value is preserved
        match result {
            SecureValue::Keyring { keyring_account } => {
                // Keyring succeeded - verify the account name is correct
                assert_eq!(keyring_account, format!("{}-{}", profile_name, username));
                // Clean up the keyring entry
                let entry = keyring::Entry::new(KEYRING_SERVICE, &keyring_account).unwrap();
                let _ = entry.delete_credential();
            }
            SecureValue::Plain(stored) => {
                // Keyring failed - verify the password value is preserved
                use secrecy::ExposeSecret;
                assert_eq!(stored.expose_secret(), password_str);
            }
        }
    }

    #[test]
    fn test_corrupt_config_backup_created() {
        use std::io::Write;

        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("config.json");

        // Create a corrupt config file with invalid JSON
        let mut file = std::fs::File::create(&config_path).unwrap();
        file.write_all(b"{ invalid json }").unwrap();
        drop(file);

        // Initialize ConfigManager - should create backup and use defaults
        let manager = ConfigManager::new_with_path(config_path.clone()).unwrap();

        // Verify ConfigManager uses defaults (no profiles)
        assert!(manager.list_profiles().is_empty());
        // Verify default state is loaded (auto_refresh defaults to false)
        assert!(!manager.load().auto_refresh);

        // Verify backup was created
        let backup_files: Vec<_> = std::fs::read_dir(&temp_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name();
                let name_str = name.to_string_lossy();
                name_str.starts_with("config.corrupt.")
            })
            .collect();

        assert_eq!(backup_files.len(), 1, "Expected exactly one backup file");

        // Verify backup contains original corrupt content
        let backup_content = std::fs::read_to_string(backup_files[0].path()).unwrap();
        assert_eq!(backup_content, "{ invalid json }");

        // Verify original config path no longer exists (was renamed to backup)
        assert!(!config_path.exists());
    }

    #[test]
    fn test_corrupt_config_backup_path_format() {
        use std::io::Write;

        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("config.json");

        // Create a corrupt config file
        let mut file = std::fs::File::create(&config_path).unwrap();
        file.write_all(b"{ bad json }").unwrap();
        drop(file);

        let before_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Initialize ConfigManager
        let _manager = ConfigManager::new_with_path(config_path.clone()).unwrap();

        let after_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Find backup file
        let backup_files: Vec<_> = std::fs::read_dir(&temp_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name();
                let name_str = name.to_string_lossy();
                name_str.starts_with("config.corrupt.")
            })
            .collect();

        assert_eq!(backup_files.len(), 1);

        let backup_name = backup_files[0].file_name().to_string_lossy().to_string();

        // Verify format: config.corrupt.{timestamp}
        let parts: Vec<_> = backup_name.split('.').collect();
        assert_eq!(
            parts.len(),
            3,
            "Backup name should have 3 parts: config.corrupt.TIMESTAMP"
        );
        assert_eq!(parts[0], "config");
        assert_eq!(parts[1], "corrupt");

        // Verify timestamp is within expected range
        let timestamp: u64 = parts[2]
            .parse()
            .expect("Timestamp should be a valid number");
        assert!(
            timestamp >= before_timestamp && timestamp <= after_timestamp,
            "Timestamp should be within test execution time"
        );
    }

    #[test]
    fn test_no_data_loss_on_corrupt_config_recovery() {
        temp_env::with_var("SPLUNK_CONFIG_NO_MIGRATE", Some("1"), || {
            use std::io::Write;

            let temp_dir = tempfile::tempdir().unwrap();
            let config_path = temp_dir.path().join("config.json");

            // Create a valid config with profiles
            let valid_config = r#"{
            "profiles": {
                "production": {
                    "base_url": "https://prod.splunk.com:8089",
                    "username": "admin"
                }
            },
            "state": {
                "auto_refresh": true,
                "sort_column": "status",
                "sort_direction": "desc"
            }
        }"#;

            std::fs::write(&config_path, valid_config).unwrap();

            // Corrupt the file by overwriting with invalid JSON
            let mut file = std::fs::File::create(&config_path).unwrap();
            file.write_all(b"{ corrupted }").unwrap();
            drop(file);

            // Initialize ConfigManager - should backup corrupt file
            let mut manager = ConfigManager::new_with_path(config_path.clone()).unwrap();

            // Verify backup exists with original content
            let backup_files: Vec<_> = std::fs::read_dir(&temp_dir)
                .unwrap()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    let name = e.file_name();
                    let name_str = name.to_string_lossy();
                    name_str.starts_with("config.corrupt.")
                })
                .collect();

            assert_eq!(backup_files.len(), 1);

            // Verify backup contains the corrupted content (not the original valid config)
            let backup_content = std::fs::read_to_string(backup_files[0].path()).unwrap();
            assert_eq!(backup_content, "{ corrupted }");

            // Verify manager uses defaults (empty)
            assert!(manager.list_profiles().is_empty());

            // Add a new profile and save
            let new_profile = ProfileConfig {
                base_url: Some("https://new.splunk.com:8089".to_string()),
                username: Some("new_user".to_string()),
                ..Default::default()
            };
            manager.save_profile("new-profile", new_profile).unwrap();

            // Reload and verify new config is valid
            let reloaded = ConfigManager::new_with_path(config_path.clone()).unwrap();
            assert_eq!(reloaded.list_profiles().len(), 1);
            assert!(reloaded.list_profiles().contains_key("new-profile"));
        });
    }

    #[test]
    fn test_encryption_roundtrip_persistence() {
        let temp_file = NamedTempFile::new().unwrap();
        let config_path = temp_file.path().to_path_buf();

        // Use password for testing to avoid keyring dependency in CI
        let password = SecretString::new("test-password".to_string().into());
        let source = MasterKeySource::Password(password.clone());

        let mut manager =
            ConfigManager::new_with_path_and_source(config_path.clone(), source.clone()).unwrap();
        manager.enable_encryption(source.clone()).unwrap();

        let profile = ProfileConfig {
            base_url: Some("https://encrypted.splunk.com:8089".to_string()),
            ..Default::default()
        };
        manager.save_profile("encrypted-profile", profile).unwrap();

        // Check that file is indeed encrypted (contains version 2-encrypted)
        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("2-encrypted"));
        assert!(!content.contains("encrypted.splunk.com"));

        // Reload and verify
        let reloaded = ConfigManager::new_with_path_and_source(config_path, source).unwrap();
        assert!(reloaded.use_encryption);
        assert_eq!(
            reloaded.config_file.profiles["encrypted-profile"].base_url,
            Some("https://encrypted.splunk.com:8089".to_string())
        );
    }

    #[test]
    #[serial]
    fn test_multiple_corrupt_backups_different_timestamps() {
        use std::thread;
        use std::time::Duration;

        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("config.json");

        // First corruption
        std::fs::write(&config_path, "{ corrupt 1 }").unwrap();
        let _manager1 = ConfigManager::new_with_path(config_path.clone()).unwrap();

        // Small delay to ensure different timestamp
        thread::sleep(Duration::from_millis(MIGRATION_DELAY_MS));

        // Second corruption (create new file and corrupt it)
        std::fs::write(&config_path, "{ corrupt 2 }").unwrap();
        let _manager2 = ConfigManager::new_with_path(config_path.clone()).unwrap();

        // Verify two backups exist
        let backup_files: Vec<_> = std::fs::read_dir(&temp_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name();
                let name_str = name.to_string_lossy();
                name_str.starts_with("config.corrupt.")
            })
            .collect();

        assert_eq!(
            backup_files.len(),
            2,
            "Expected two backup files with different timestamps"
        );

        // Verify contents are different
        let contents: Vec<_> = backup_files
            .iter()
            .map(|f| std::fs::read_to_string(f.path()).unwrap())
            .collect();

        assert!(contents.contains(&"{ corrupt 1 }".to_string()));
        assert!(contents.contains(&"{ corrupt 2 }".to_string()));
    }

    #[test]
    fn test_valid_config_no_backup() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("config.json");

        // Create a valid config
        let valid_config = r#"{
            "profiles": {
                "test": {
                    "base_url": "https://test.splunk.com:8089"
                }
            }
        }"#;

        std::fs::write(&config_path, valid_config).unwrap();

        // Initialize ConfigManager
        let manager = ConfigManager::new_with_path(config_path.clone()).unwrap();

        // Verify no backup was created
        let backup_files: Vec<_> = std::fs::read_dir(&temp_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name();
                let name_str = name.to_string_lossy();
                name_str.starts_with("config.corrupt.")
            })
            .collect();

        assert!(
            backup_files.is_empty(),
            "No backup should be created for valid config"
        );

        // Verify config loaded correctly
        assert_eq!(manager.list_profiles().len(), 1);
        assert!(manager.list_profiles().contains_key("test"));
    }

    #[test]
    fn test_missing_config_no_backup() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("config.json");

        // Don't create the file - it doesn't exist
        assert!(!config_path.exists());

        // Initialize ConfigManager
        let manager = ConfigManager::new_with_path(config_path.clone()).unwrap();

        // Verify no backup was created
        let backup_files: Vec<_> = std::fs::read_dir(&temp_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name();
                let name_str = name.to_string_lossy();
                name_str.starts_with("config.corrupt.")
            })
            .collect();

        assert!(
            backup_files.is_empty(),
            "No backup should be created for missing config"
        );

        // Verify manager uses defaults
        assert!(manager.list_profiles().is_empty());
    }

    #[test]
    #[serial]
    fn test_try_store_token_in_keyring_fallback() {
        let temp_file = NamedTempFile::new().unwrap();
        let manager = ConfigManager::new_with_path(temp_file.path().to_path_buf()).unwrap();

        let profile_name = "test-try-token-fallback";
        let token_str = "test-token-abc123";
        let token = SecretString::new(token_str.to_string().into());

        // This should try keyring and fall back to plaintext if keyring fails
        let result = manager.try_store_token_in_keyring(profile_name, &token);

        // Result should be either Keyring (if keyring succeeded) or Plain (if it failed/fell back)
        // Both are valid - the important thing is that the token value is preserved
        match result {
            SecureValue::Keyring { keyring_account } => {
                // Keyring succeeded - verify the account name is correct
                assert_eq!(keyring_account, format!("{}-token", profile_name));
                // Clean up the keyring entry
                let entry = keyring::Entry::new(KEYRING_SERVICE, &keyring_account).unwrap();
                let _ = entry.delete_credential();
            }
            SecureValue::Plain(stored) => {
                // Keyring failed - verify the token value is preserved
                use secrecy::ExposeSecret;
                assert_eq!(stored.expose_secret(), token_str);
            }
        }
    }

    #[test]
    #[serial]
    fn test_strict_store_password_in_keyring_never_returns_plain() {
        let temp_file = NamedTempFile::new().unwrap();
        let manager = ConfigManager::new_with_path(temp_file.path().to_path_buf()).unwrap();

        let profile_name = "test-strict-password-never-plain";
        let username = "admin";
        let password_str = "test-password-strict";
        let password = SecretString::new(password_str.to_string().into());

        let result = manager.strict_store_password_in_keyring(profile_name, username, &password);

        match result {
            Ok(SecureValue::Keyring { keyring_account }) => {
                // Keyring succeeded - verify the account name
                assert_eq!(keyring_account, format!("{}-{}", profile_name, username));
                // Clean up
                let entry = keyring::Entry::new(KEYRING_SERVICE, &keyring_account).unwrap();
                let _ = entry.delete_credential();
            }
            Err(CredentialStorageError::PasswordStoreFailed { profile, .. }) => {
                // Expected when keyring is unavailable - this is the SECURE behavior
                assert_eq!(profile, profile_name);
            }
            Err(CredentialStorageError::TokenStoreFailed { .. }) => {
                panic!("Password storage should not return TokenStoreFailed error");
            }
            Ok(SecureValue::Plain(_)) => {
                panic!("strict_store_password_in_keyring must NEVER return Plain variant");
            }
        }
    }

    #[test]
    #[serial]
    fn test_strict_store_token_in_keyring_never_returns_plain() {
        let temp_file = NamedTempFile::new().unwrap();
        let manager = ConfigManager::new_with_path(temp_file.path().to_path_buf()).unwrap();

        let profile_name = "test-strict-token-never-plain";
        let token_str = "test-token-strict-xyz";
        let token = SecretString::new(token_str.to_string().into());

        let result = manager.strict_store_token_in_keyring(profile_name, &token);

        match result {
            Ok(SecureValue::Keyring { keyring_account }) => {
                // Keyring succeeded
                assert_eq!(keyring_account, format!("{}-token", profile_name));
                // Clean up
                let entry = keyring::Entry::new(KEYRING_SERVICE, &keyring_account).unwrap();
                let _ = entry.delete_credential();
            }
            Err(CredentialStorageError::TokenStoreFailed { profile, .. }) => {
                // Expected when keyring is unavailable - this is the SECURE behavior
                assert_eq!(profile, profile_name);
            }
            Err(CredentialStorageError::PasswordStoreFailed { .. }) => {
                panic!("Token storage should not return PasswordStoreFailed error");
            }
            Ok(SecureValue::Plain(_)) => {
                panic!("strict_store_token_in_keyring must NEVER return Plain variant");
            }
        }
    }

    #[test]
    #[serial]
    fn test_try_store_functions_still_provide_fallback_for_backwards_compat() {
        // Verify that try_store_* functions still provide fallback behavior
        // for any existing callers that depend on it
        let temp_file = NamedTempFile::new().unwrap();
        let manager = ConfigManager::new_with_path(temp_file.path().to_path_buf()).unwrap();

        let profile_name = "test-try-fallback-compat";
        let password = SecretString::new("test-password".to_string().into());

        let result = manager.try_store_password_in_keyring(profile_name, "admin", &password);

        // Result must be either Keyring (success) or Plain (fallback) - never an error
        match result {
            SecureValue::Keyring { keyring_account } => {
                let entry = keyring::Entry::new(KEYRING_SERVICE, &keyring_account).unwrap();
                let _ = entry.delete_credential();
            }
            SecureValue::Plain(_) => {
                // Fallback occurred - this is expected behavior for try_* functions
            }
        }
    }
}
