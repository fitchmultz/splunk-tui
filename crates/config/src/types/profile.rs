//! Profile configuration types for Splunk TUI.
//!
//! Responsibilities:
//! - Define `ProfileConfig` for storing named connection profiles.
//! - Support partial configuration (all fields optional) for profile inheritance.
//!
//! Does NOT handle:
//! - Profile loading or merging (see `loader` module).
//! - Profile persistence (see `persistence` module).
//!
//! Invariants:
//! - All fields are optional to allow partial profile definitions.
//! - Password/token fields use `SecureValue` for flexible secret storage.
//! - ProfileConfig uses `#[serde(default)]` for backward compatibility.

use crate::types::auth::SecureValue;
use serde::{Deserialize, Serialize};

/// Profile configuration for storing named connection profiles.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProfileConfig {
    /// Base URL of the Splunk server
    pub base_url: Option<String>,
    /// Username for session authentication
    pub username: Option<String>,
    /// Password for session authentication
    pub password: Option<SecureValue>,
    /// API token for bearer authentication
    pub api_token: Option<SecureValue>,
    /// Whether to skip TLS verification
    pub skip_verify: Option<bool>,
    /// Connection timeout in seconds
    pub timeout_seconds: Option<u64>,
    /// Maximum number of retries for failed requests
    pub max_retries: Option<usize>,
    /// Buffer time before session expiry to proactively refresh tokens (in seconds)
    /// This prevents race conditions where a token expires during an API call.
    /// Default: 60 seconds
    pub session_expiry_buffer_seconds: Option<u64>,
    /// Session time-to-live in seconds (how long tokens remain valid)
    /// Default: 3600 seconds (1 hour)
    pub session_ttl_seconds: Option<u64>,
    /// Health check interval in seconds (how often to poll server health)
    /// Default: 60 seconds
    pub health_check_interval_seconds: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::connection::{
        default_health_check_interval, default_session_expiry_buffer, default_session_ttl,
    };
    use secrecy::{ExposeSecret, SecretString};

    #[test]
    fn test_profile_config_serde_round_trip() {
        let password = SecretString::new("test-password".to_string().into());
        let original = ProfileConfig {
            base_url: Some("https://splunk.example.com:8089".to_string()),
            username: Some("admin".to_string()),
            password: Some(SecureValue::Plain(password)),
            api_token: None,
            skip_verify: Some(true),
            timeout_seconds: Some(60),
            max_retries: Some(5),
            session_expiry_buffer_seconds: Some(default_session_expiry_buffer()),
            session_ttl_seconds: Some(default_session_ttl()),
            health_check_interval_seconds: Some(default_health_check_interval()),
        };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: ProfileConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.base_url, original.base_url);
        assert_eq!(deserialized.username, original.username);
        assert_eq!(deserialized.skip_verify, original.skip_verify);
        assert!(matches!(deserialized.password, Some(SecureValue::Plain(_))));
    }

    #[test]
    fn test_profile_config_backward_compatibility() {
        let json = r#"{
            "base_url": "https://localhost:8089",
            "password": "old-password"
        }"#;
        let deserialized: ProfileConfig = serde_json::from_str(json).unwrap();

        match deserialized.password {
            Some(SecureValue::Plain(s)) => {
                assert_eq!(s.expose_secret(), "old-password");
            }
            _ => panic!("Expected SecureValue::Plain"),
        }
    }

    #[test]
    fn test_profile_config_keyring_serde() {
        let json = r#"{
            "password": { "keyring_account": "splunk-admin" }
        }"#;

        let deserialized: ProfileConfig = serde_json::from_str(json).unwrap();

        match deserialized.password {
            Some(SecureValue::Keyring { keyring_account }) => {
                assert_eq!(keyring_account, "splunk-admin");
            }
            _ => panic!("Expected SecureValue::Keyring"),
        }
    }

    /// Test that ProfileConfig Debug output does not expose secrets.
    #[test]
    fn test_profile_config_debug_does_not_expose_secrets() {
        let password = SecretString::new("profile-password-789".to_string().into());
        let profile = ProfileConfig {
            base_url: Some("https://localhost:8089".to_string()),
            username: Some("admin".to_string()),
            password: Some(SecureValue::Plain(password)),
            api_token: None,
            skip_verify: Some(false),
            timeout_seconds: Some(30),
            max_retries: Some(3),
            session_expiry_buffer_seconds: Some(default_session_expiry_buffer()),
            session_ttl_seconds: Some(default_session_ttl()),
            health_check_interval_seconds: Some(default_health_check_interval()),
        };

        let debug_output = format!("{:?}", profile);

        // The password should NOT appear in debug output
        assert!(
            !debug_output.contains("profile-password-789"),
            "Debug output should not contain the password"
        );

        // Non-sensitive data should be visible
        assert!(debug_output.contains("admin"));
        assert!(debug_output.contains("https://localhost:8089"));
    }

    /// Test that ProfileConfig with API token does not expose token.
    #[test]
    fn test_profile_config_api_token_not_exposed() {
        let token = SecretString::new("profile-api-token-xyz".to_string().into());
        let profile = ProfileConfig {
            base_url: Some("https://localhost:8089".to_string()),
            username: Some("admin".to_string()),
            password: None,
            api_token: Some(SecureValue::Plain(token)),
            skip_verify: Some(false),
            timeout_seconds: Some(30),
            max_retries: Some(3),
            session_expiry_buffer_seconds: Some(default_session_expiry_buffer()),
            session_ttl_seconds: Some(default_session_ttl()),
            health_check_interval_seconds: Some(default_health_check_interval()),
        };

        let debug_output = format!("{:?}", profile);

        // The API token should NOT appear in debug output
        assert!(
            !debug_output.contains("profile-api-token-xyz"),
            "Debug output should not contain the API token"
        );
    }
}
