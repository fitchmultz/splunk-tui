//! Configuration types for Splunk TUI.

use std::collections::BTreeMap;

use ratatui::style::Color;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use std::{fmt, time::Duration};

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

/// Service name used for keyring storage.
pub const KEYRING_SERVICE: &str = "splunk-tui";

/// User-selectable color theme.
///
/// This is persisted to disk via `PersistedState` and expanded into a full `Theme` at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ColorTheme {
    #[default]
    Default,
    Light,
    Dark,
    HighContrast,
}

impl ColorTheme {
    /// Human-readable display name for UI surfaces.
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::Light => "Light",
            Self::Dark => "Dark",
            Self::HighContrast => "High Contrast",
        }
    }

    /// Next theme in the cycle (used by Settings screen "t" key).
    pub fn cycle_next(self) -> Self {
        match self {
            Self::Default => Self::Light,
            Self::Light => Self::Dark,
            Self::Dark => Self::HighContrast,
            Self::HighContrast => Self::Default,
        }
    }
}

impl fmt::Display for ColorTheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.display_name())
    }
}

/// Expanded runtime theme.
///
/// Invariants:
/// - This is intentionally **not serialized**. Persist `ColorTheme` and expand on startup.
/// - Colors should be semantically meaningful (error/warn/success/info).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    // Global / chrome
    pub background: Color,
    pub text: Color,
    pub text_dim: Color,
    pub border: Color,
    pub title: Color,
    pub accent: Color,

    // Selection / highlight
    pub highlight_fg: Color,
    pub highlight_bg: Color,

    // Semantics
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,
    pub disabled: Color,

    // Tables
    pub table_header_fg: Color,
    pub table_header_bg: Color,

    // Health indicator
    pub health_healthy: Color,
    pub health_unhealthy: Color,
    pub health_unknown: Color,

    // Logs
    pub log_error: Color,
    pub log_warn: Color,
    pub log_info: Color,
    pub log_debug: Color,
    pub log_component: Color,

    // Syntax highlighting
    pub syntax_command: Color,
    pub syntax_operator: Color,
    pub syntax_function: Color,
    pub syntax_string: Color,
    pub syntax_number: Color,
    pub syntax_comment: Color,
    pub syntax_punctuation: Color,
    pub syntax_pipe: Color,
    pub syntax_comparison: Color,
}

impl Theme {
    /// Expand a persisted `ColorTheme` into a full runtime palette.
    pub fn from_color_theme(theme: ColorTheme) -> Self {
        match theme {
            ColorTheme::Default => Self {
                background: Color::Black,
                text: Color::White,
                text_dim: Color::Gray,
                border: Color::Cyan,
                title: Color::Cyan,
                accent: Color::Yellow,

                highlight_fg: Color::Yellow,
                highlight_bg: Color::DarkGray,

                success: Color::Green,
                warning: Color::Yellow,
                error: Color::Red,
                info: Color::Cyan,
                disabled: Color::DarkGray,

                table_header_fg: Color::Cyan,
                table_header_bg: Color::DarkGray,

                health_healthy: Color::Green,
                health_unhealthy: Color::Red,
                health_unknown: Color::Yellow,

                log_error: Color::Red,
                log_warn: Color::Yellow,
                log_info: Color::Green,
                log_debug: Color::Blue,
                log_component: Color::Magenta,

                syntax_command: Color::Cyan,
                syntax_operator: Color::Magenta,
                syntax_function: Color::Blue,
                syntax_string: Color::Green,
                syntax_number: Color::Blue,
                syntax_comment: Color::Gray,
                syntax_punctuation: Color::DarkGray,
                syntax_pipe: Color::Yellow,
                syntax_comparison: Color::Red,
            },
            ColorTheme::Light => Self {
                background: Color::White,
                text: Color::Black,
                text_dim: Color::Gray,
                border: Color::Blue,
                title: Color::Blue,
                accent: Color::Magenta,

                highlight_fg: Color::Black,
                highlight_bg: Color::Gray,

                success: Color::Green,
                warning: Color::Yellow,
                error: Color::Red,
                info: Color::Blue,
                disabled: Color::Gray,

                table_header_fg: Color::Black,
                table_header_bg: Color::Gray,

                health_healthy: Color::Green,
                health_unhealthy: Color::Red,
                health_unknown: Color::Yellow,

                log_error: Color::Red,
                log_warn: Color::Yellow,
                log_info: Color::Green,
                log_debug: Color::Blue,
                log_component: Color::Magenta,

                syntax_command: Color::Blue,
                syntax_operator: Color::Magenta,
                syntax_function: Color::Blue,
                syntax_string: Color::Green,
                syntax_number: Color::Blue,
                syntax_comment: Color::Gray,
                syntax_punctuation: Color::Gray,
                syntax_pipe: Color::Magenta,
                syntax_comparison: Color::Red,
            },
            ColorTheme::Dark => Self {
                background: Color::Black,
                text: Color::White,
                text_dim: Color::Gray,
                border: Color::Indexed(110), // soft blue/cyan
                title: Color::Indexed(110),
                accent: Color::Indexed(214), // orange-ish

                highlight_fg: Color::White,
                highlight_bg: Color::Indexed(236),

                success: Color::Green,
                warning: Color::Yellow,
                error: Color::Red,
                info: Color::Indexed(110),
                disabled: Color::DarkGray,

                table_header_fg: Color::Indexed(110),
                table_header_bg: Color::Indexed(236),

                health_healthy: Color::Green,
                health_unhealthy: Color::Red,
                health_unknown: Color::Yellow,

                log_error: Color::Red,
                log_warn: Color::Yellow,
                log_info: Color::Green,
                log_debug: Color::Indexed(110),
                log_component: Color::Indexed(176),

                syntax_command: Color::Indexed(110),
                syntax_operator: Color::Indexed(176),
                syntax_function: Color::Indexed(75),
                syntax_string: Color::Green,
                syntax_number: Color::Indexed(75),
                syntax_comment: Color::Gray,
                syntax_punctuation: Color::DarkGray,
                syntax_pipe: Color::Indexed(214),
                syntax_comparison: Color::Red,
            },
            ColorTheme::HighContrast => Self {
                background: Color::Black,
                text: Color::White,
                text_dim: Color::Gray,
                border: Color::White,
                title: Color::White,
                accent: Color::Yellow,

                highlight_fg: Color::White,
                highlight_bg: Color::Blue,

                success: Color::Green,
                warning: Color::Yellow,
                error: Color::Red,
                info: Color::Cyan,
                disabled: Color::Gray,

                table_header_fg: Color::Black,
                table_header_bg: Color::White,

                health_healthy: Color::Green,
                health_unhealthy: Color::Red,
                health_unknown: Color::Yellow,

                log_error: Color::Red,
                log_warn: Color::Yellow,
                log_info: Color::Green,
                log_debug: Color::Cyan,
                log_component: Color::Yellow,

                syntax_command: Color::Cyan,
                syntax_operator: Color::Yellow,
                syntax_function: Color::Magenta,
                syntax_string: Color::Green,
                syntax_number: Color::Cyan,
                syntax_comment: Color::Gray,
                syntax_punctuation: Color::White,
                syntax_pipe: Color::Yellow,
                syntax_comparison: Color::Red,
            },
        }
    }
}

impl From<ColorTheme> for Theme {
    fn from(value: ColorTheme) -> Self {
        Self::from_color_theme(value)
    }
}

impl Default for Theme {
    fn default() -> Self {
        ColorTheme::Default.into()
    }
}

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
}

/// An overridable keybinding action identifier.
///
/// This enum represents the subset of actions that users can customize.
/// Starting with global navigation only; may expand in the future.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum KeybindAction {
    /// Quit the application
    Quit,
    /// Open the help popup
    Help,
    /// Navigate to the next screen
    NextScreen,
    /// Navigate to the previous screen
    PreviousScreen,
}

impl fmt::Display for KeybindAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Quit => write!(f, "quit"),
            Self::Help => write!(f, "help"),
            Self::NextScreen => write!(f, "next_screen"),
            Self::PreviousScreen => write!(f, "previous_screen"),
        }
    }
}

/// User-defined keybinding overrides.
///
/// Maps action identifiers to key combinations. Only actions explicitly
/// listed here override the defaults; all others use built-in bindings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KeybindOverrides {
    /// Map of action -> key combination string.
    /// Using BTreeMap for deterministic serialization.
    #[serde(default)]
    pub overrides: BTreeMap<KeybindAction, String>,
}

impl KeybindOverrides {
    /// Returns true if there are no overrides configured.
    pub fn is_empty(&self) -> bool {
        self.overrides.is_empty()
    }

    /// Get the override for a specific action, if any.
    pub fn get(&self, action: KeybindAction) -> Option<&str> {
        self.overrides.get(&action).map(|s| s.as_str())
    }
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
            password: Some(SecureValue::Plain(password)),
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
                use secrecy::ExposeSecret;
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

    #[test]
    fn test_secure_value_resolve_plain() {
        use secrecy::ExposeSecret;
        let secret = SecretString::new("test-secret".to_string().into());
        let val = SecureValue::Plain(secret.clone());
        let resolved = val.resolve().unwrap();
        assert_eq!(resolved.expose_secret(), secret.expose_secret());
    }
}
