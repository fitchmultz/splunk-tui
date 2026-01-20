//! Configuration types for Splunk TUI.

use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use std::time::Duration;

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

/// Module for serializing Option<SecretString> as optional strings.
mod secret_string_opt {
    use secrecy::{ExposeSecret, SecretString};
    use serde::{Deserialize as DeserializeTrait, Serialize as SerializeTrait};
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(secret: &Option<SecretString>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match secret {
            Some(s) => s.expose_secret().serialize(serializer),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<SecretString>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<String>::deserialize(deserializer)?;
        Ok(opt.map(|s| SecretString::new(s.into())))
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

/// Module for serializing Duration as seconds (integer).
mod duration_seconds {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

/// Connection configuration for Splunk server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    /// Base URL of the Splunk server (e.g., https://localhost:8089)
    pub base_url: String,
    /// Whether to skip TLS verification (for self-signed certificates)
    pub skip_verify: bool,
    /// Connection timeout (serialized as seconds)
    #[serde(with = "duration_seconds")]
    pub timeout: Duration,
    /// Maximum number of retries for failed requests
    pub max_retries: usize,
}

/// Main configuration structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Connection settings
    pub connection: ConnectionConfig,
    /// Authentication settings
    pub auth: AuthConfig,
}

/// Profile configuration for storing named connection profiles.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct ProfileConfig {
    /// Base URL of the Splunk server
    pub base_url: Option<String>,
    /// Username for session authentication
    pub username: Option<String>,
    /// Password for session authentication
    #[serde(with = "secret_string_opt")]
    pub password: Option<SecretString>,
    /// API token for bearer authentication
    #[serde(with = "secret_string_opt")]
    pub api_token: Option<SecretString>,
    /// Whether to skip TLS verification
    pub skip_verify: Option<bool>,
    /// Connection timeout in seconds
    pub timeout_seconds: Option<u64>,
    /// Maximum number of retries for failed requests
    pub max_retries: Option<usize>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            connection: ConnectionConfig {
                base_url: "https://localhost:8089".to_string(),
                skip_verify: false,
                timeout: Duration::from_secs(30),
                max_retries: 3,
            },
            auth: AuthConfig {
                strategy: AuthStrategy::SessionToken {
                    username: "admin".to_string(),
                    password: SecretString::new("changeme".to_string().into()),
                },
            },
        }
    }
}

impl Config {
    /// Create a new config with the specified base URL and API token.
    pub fn with_api_token(base_url: String, token: SecretString) -> Self {
        Self {
            connection: ConnectionConfig {
                base_url,
                skip_verify: false,
                timeout: Duration::from_secs(30),
                max_retries: 3,
            },
            auth: AuthConfig {
                strategy: AuthStrategy::ApiToken { token },
            },
        }
    }

    /// Create a new config with the specified base URL and username/password.
    pub fn with_session_token(base_url: String, username: String, password: SecretString) -> Self {
        Self {
            connection: ConnectionConfig {
                base_url,
                skip_verify: false,
                timeout: Duration::from_secs(30),
                max_retries: 3,
            },
            auth: AuthConfig {
                strategy: AuthStrategy::SessionToken { username, password },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.connection.base_url, "https://localhost:8089");
        assert!(!config.connection.skip_verify);
    }

    #[test]
    fn test_config_with_api_token() {
        let token = SecretString::new("test-token".to_string().into());
        let config = Config::with_api_token("https://splunk.example.com:8089".to_string(), token);
        assert!(matches!(
            config.auth.strategy,
            AuthStrategy::ApiToken { .. }
        ));
    }

    #[test]
    fn test_config_with_session_token() {
        let password = SecretString::new("test-password".to_string().into());
        let config = Config::with_session_token(
            "https://splunk.example.com:8089".to_string(),
            "admin".to_string(),
            password,
        );
        assert!(matches!(
            config.auth.strategy,
            AuthStrategy::SessionToken { .. }
        ));
    }

    #[test]
    fn test_auth_strategy_serde_round_trip() {
        let token = SecretString::new("test-token".to_string().into());
        let original = AuthStrategy::ApiToken { token };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: AuthStrategy = serde_json::from_str(&json).unwrap();

        assert!(matches!(deserialized, AuthStrategy::ApiToken { .. }));
    }

    #[test]
    fn test_connection_config_serde_seconds() {
        let config = ConnectionConfig {
            base_url: "https://localhost:8089".to_string(),
            skip_verify: true,
            timeout: Duration::from_secs(60),
            max_retries: 5,
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ConnectionConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.timeout, Duration::from_secs(60));
        assert_eq!(deserialized.max_retries, 5);
    }

    #[test]
    fn test_profile_config_serde_round_trip() {
        let password = SecretString::new("test-password".to_string().into());
        let original = ProfileConfig {
            base_url: Some("https://splunk.example.com:8089".to_string()),
            username: Some("admin".to_string()),
            password: Some(password),
            api_token: None,
            skip_verify: Some(true),
            timeout_seconds: Some(60),
            max_retries: Some(5),
        };

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: ProfileConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.base_url, original.base_url);
        assert_eq!(deserialized.username, original.username);
        assert_eq!(deserialized.skip_verify, original.skip_verify);
    }
}
