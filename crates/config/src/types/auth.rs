//! Authentication types for Splunk TUI configuration.
//!
//! Responsibilities:
//! - Define authentication strategies (session token, API token).
//! - Provide secure value storage (plain text or keyring).
//! - Handle serialization of secret values.
//!
//! Does NOT handle:
//! - Actual authentication flow or token exchange (see client crate).
//! - Keyring entry creation/management (only retrieval).
//!
//! Invariants:
//! - All secret values use `secrecy::SecretString` to prevent accidental logging.
//! - Serialization includes secrets for config file persistence; secrecy is for runtime safety.
//! - `KEYRING_SERVICE` is the canonical service name for all keyring operations.

use secrecy::SecretString;
use serde::{Deserialize, Serialize};

/// Module for serializing SecretString as strings.
mod secret_string {
    use secrecy::{ExposeSecret, SecretString};
    use serde::{Deserialize as DeserializeTrait, Serialize as SerializeTrait};
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(secret: &SecretString, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        secret.expose_secret().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SecretString, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(SecretString::new(s.into()))
    }
}

/// Strategy for authenticating with Splunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AuthStrategy {
    /// Username and password authentication (creates session token)
    #[serde(rename = "session")]
    SessionToken {
        username: String,
        #[serde(with = "secret_string")]
        password: SecretString,
    },
    /// API token (bearer token authentication)
    #[serde(rename = "token")]
    ApiToken {
        #[serde(with = "secret_string")]
        token: SecretString,
    },
}

/// Authentication configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// The authentication strategy to use.
    #[serde(flatten)]
    pub strategy: AuthStrategy,
}

/// Service name used for keyring storage.
pub const KEYRING_SERVICE: &str = "splunk-tui";

/// A value that can be stored either in plain text or in the system keyring.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SecureValue {
    /// Value stored in the system keyring.
    Keyring {
        /// The account name in the keyring.
        keyring_account: String,
    },
    /// Value stored in plain text (as a SecretString).
    #[serde(with = "secret_string")]
    Plain(SecretString),
}

impl SecureValue {
    /// Resolve the secure value to a SecretString.
    ///
    /// If the value is stored in the keyring, it will be fetched.
    pub fn resolve(&self) -> Result<SecretString, keyring::Error> {
        match self {
            Self::Plain(secret) => Ok(secret.clone()),
            Self::Keyring { keyring_account } => {
                let entry = keyring::Entry::new(KEYRING_SERVICE, keyring_account)?;
                let password = entry.get_password()?;
                Ok(SecretString::new(password.into()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_strategy_serde_round_trip() {
        let token = SecretString::new("test-token".to_string().into());
        let original = AuthStrategy::ApiToken { token };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: AuthStrategy = serde_json::from_str(&json).unwrap();

        assert!(matches!(deserialized, AuthStrategy::ApiToken { .. }));
    }

    #[test]
    fn test_secure_value_resolve_plain() {
        use secrecy::ExposeSecret;
        let secret = SecretString::new("test-secret".to_string().into());
        let val = SecureValue::Plain(secret.clone());
        let resolved = val.resolve().unwrap();
        assert_eq!(resolved.expose_secret(), secret.expose_secret());
    }

    // ============================================================================
    // Security-focused tests for secret handling
    // ============================================================================

    /// Test that AuthConfig Debug output does not expose API token secrets.
    #[test]
    fn test_auth_config_debug_does_not_expose_api_token() {
        let token = SecretString::new("api-token-secret-123".to_string().into());
        let auth_config = AuthConfig {
            strategy: AuthStrategy::ApiToken { token },
        };

        let debug_output = format!("{:?}", auth_config);

        // The secret token should NOT appear in debug output
        assert!(
            !debug_output.contains("api-token-secret-123"),
            "Debug output should not contain the API token"
        );
    }

    /// Test that AuthConfig Debug output does not expose session password.
    #[test]
    fn test_auth_config_debug_does_not_expose_password() {
        let password = SecretString::new("session-password-456".to_string().into());
        let auth_config = AuthConfig {
            strategy: AuthStrategy::SessionToken {
                username: "admin".to_string(),
                password,
            },
        };

        let debug_output = format!("{:?}", auth_config);

        // The password should NOT appear in debug output
        assert!(
            !debug_output.contains("session-password-456"),
            "Debug output should not contain the password"
        );

        // But the username SHOULD be visible
        assert!(debug_output.contains("admin"));
    }

    /// Test that SecureValue::Plain does not expose secret in Debug output.
    #[test]
    fn test_secure_value_plain_not_exposed_in_debug() {
        let secret = SecretString::new("secure-value-secret".to_string().into());
        let secure_value = SecureValue::Plain(secret);

        let debug_output = format!("{:?}", secure_value);

        // The secret should NOT appear in debug output
        assert!(
            !debug_output.contains("secure-value-secret"),
            "Debug output should not contain the secret"
        );
    }

    /// Test that SecureValue::Keyring does not expose any secret data.
    #[test]
    fn test_secure_value_keyring_not_exposed_in_debug() {
        let secure_value = SecureValue::Keyring {
            keyring_account: "splunk-admin".to_string(),
        };

        let debug_output = format!("{:?}", secure_value);

        // The account name is not a secret and can be visible
        assert!(debug_output.contains("splunk-admin"));
        assert!(debug_output.contains("Keyring"));
    }

    /// Test serialization of AuthStrategy includes secrets (for persistence).
    ///
    /// Note: This test verifies that serialization DOES include the secret,
    /// which is intentional for config file persistence. The secrecy is for
    /// logging safety, not persistence safety.
    #[test]
    fn test_auth_strategy_serialization_includes_secret() {
        use secrecy::ExposeSecret;

        let token = SecretString::new("serializable-token".to_string().into());
        let strategy = AuthStrategy::ApiToken { token };

        let json = serde_json::to_string(&strategy).unwrap();

        // Serialization SHOULD include the secret for persistence
        assert!(json.contains("serializable-token"));

        // Deserialize and verify
        let deserialized: AuthStrategy = serde_json::from_str(&json).unwrap();
        match deserialized {
            AuthStrategy::ApiToken { token } => {
                assert_eq!(token.expose_secret(), "serializable-token");
            }
            _ => panic!("Expected ApiToken variant"),
        }
    }

    /// Test that serialization of session auth includes password.
    #[test]
    fn test_session_auth_serialization_includes_password() {
        use secrecy::ExposeSecret;

        let password = SecretString::new("serializable-password".to_string().into());
        let strategy = AuthStrategy::SessionToken {
            username: "admin".to_string(),
            password,
        };

        let json = serde_json::to_string(&strategy).unwrap();

        // Serialization SHOULD include the password for persistence
        assert!(json.contains("serializable-password"));
        assert!(json.contains("admin"));

        // Deserialize and verify
        let deserialized: AuthStrategy = serde_json::from_str(&json).unwrap();
        match deserialized {
            AuthStrategy::SessionToken { username, password } => {
                assert_eq!(username, "admin");
                assert_eq!(password.expose_secret(), "serializable-password");
            }
            _ => panic!("Expected SessionToken variant"),
        }
    }
}
