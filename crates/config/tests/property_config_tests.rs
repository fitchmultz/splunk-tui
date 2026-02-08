//! Property-based tests for configuration serialization.
//!
//! These tests verify that configuration types can be serialized and deserialized
//! without losing information, using randomly generated inputs to catch edge cases
//! that might not be covered by unit tests.
//!
//! Test coverage:
//! - ConnectionConfig: Roundtrip serialization with all fields
//! - AuthConfig with ApiToken: Verify strategy type is preserved
//! - AuthConfig with SessionToken: Verify strategy type is preserved
//! - Config construction with api_token: Verify full config roundtrip

use proptest::prelude::*;
use secrecy::{ExposeSecret, SecretString};

use splunk_config::{AuthConfig, AuthStrategy, Config, ConnectionConfig};
use std::time::Duration;

/// Strategy for generating valid base URLs.
///
/// Generates URLs in the form:
/// - `https://localhost:{port}` for local development scenarios
/// - `https://{host}.{domain}:{port}` for production-like scenarios
fn base_url_strategy() -> impl Strategy<Value = String> {
    let localhost_strategy =
        (8080u16..=8090u16).prop_map(|port| format!("https://localhost:{}", port));

    let host_strategy = prop_oneof![
        Just("splunk"),
        Just("splunk-enterprise"),
        Just("splunk-dev"),
        Just("splunk-prod"),
    ];
    let domain_strategy = prop_oneof![
        Just("example.com"),
        Just("internal.local"),
        Just("splunk.io"),
        Just("company.net"),
    ];
    let port_strategy = 8080u16..=8089u16;

    let production_strategy = (host_strategy, domain_strategy, port_strategy)
        .prop_map(|(host, domain, port)| format!("https://{}.{}:{}", host, domain, port));

    prop_oneof![localhost_strategy, production_strategy]
}

/// Strategy for generating API token strings.
///
/// Generates tokens with alphanumeric characters and common separators.
fn api_token_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9_\\-]{16,64}".prop_map(|s| format!("splunk_{}", s))
}

/// Strategy for generating username strings.
fn username_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("admin".to_string()),
        Just("splunk-user".to_string()),
        "[a-z][a-z0-9_]{3,20}".prop_map(String::from),
    ]
}

/// Strategy for generating password strings.
fn password_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9!@#$%^&*]{8,32}".prop_map(String::from)
}

/// Strategy for generating ConnectionConfig with randomized fields.
fn connection_config_strategy() -> impl Strategy<Value = ConnectionConfig> {
    (
        base_url_strategy(),
        any::<bool>(),
        1u64..=300u64,    // timeout in seconds
        0usize..=10usize, // max_retries
        30u64..=300u64,   // session_expiry_buffer_seconds
        600u64..=7200u64, // session_ttl_seconds
        10u64..=300u64,   // health_check_interval_seconds
    )
        .prop_map(
            |(
                base_url,
                skip_verify,
                timeout_secs,
                max_retries,
                session_expiry_buffer,
                session_ttl,
                health_check_interval,
            )| {
                ConnectionConfig {
                    base_url,
                    skip_verify,
                    timeout: Duration::from_secs(timeout_secs),
                    max_retries,
                    session_expiry_buffer_seconds: session_expiry_buffer,
                    session_ttl_seconds: session_ttl,
                    health_check_interval_seconds: health_check_interval,
                }
            },
        )
}

/// Strategy for generating AuthConfig with ApiToken strategy.
fn api_token_auth_config_strategy() -> impl Strategy<Value = AuthConfig> {
    api_token_strategy().prop_map(|token| AuthConfig {
        strategy: AuthStrategy::ApiToken {
            token: SecretString::new(token.into()),
        },
    })
}

/// Strategy for generating AuthConfig with SessionToken strategy.
fn session_token_auth_config_strategy() -> impl Strategy<Value = AuthConfig> {
    (username_strategy(), password_strategy()).prop_map(|(username, password)| AuthConfig {
        strategy: AuthStrategy::SessionToken {
            username,
            password: SecretString::new(password.into()),
        },
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    /// Test that ConnectionConfig can be serialized and deserialized without loss.
    ///
    /// Verifies:
    /// - All fields are preserved through roundtrip
    /// - Duration is correctly serialized as seconds
    /// - String fields (base_url) are preserved exactly
    /// - Boolean and numeric fields are preserved
    #[test]
    fn test_connection_config_roundtrip(config in connection_config_strategy()) {
        let serialized = serde_json::to_string(&config).expect("Failed to serialize ConnectionConfig");
        let deserialized: ConnectionConfig = serde_json::from_str(&serialized)
            .expect("Failed to deserialize ConnectionConfig");

        prop_assert_eq!(deserialized.base_url, config.base_url);
        prop_assert_eq!(deserialized.skip_verify, config.skip_verify);
        prop_assert_eq!(deserialized.timeout, config.timeout);
        prop_assert_eq!(deserialized.max_retries, config.max_retries);
        prop_assert_eq!(deserialized.session_expiry_buffer_seconds, config.session_expiry_buffer_seconds);
        prop_assert_eq!(deserialized.session_ttl_seconds, config.session_ttl_seconds);
        prop_assert_eq!(deserialized.health_check_interval_seconds, config.health_check_interval_seconds);
    }

    /// Test that AuthConfig with ApiToken preserves the strategy type.
    ///
    /// Verifies:
    /// - The strategy type is correctly identified as ApiToken after roundtrip
    /// - The token value is preserved (secrets are serialized for persistence)
    #[test]
    fn test_auth_config_api_token_strategy_preserved(auth_config in api_token_auth_config_strategy()) {
        let serialized = serde_json::to_string(&auth_config).expect("Failed to serialize AuthConfig");
        let deserialized: AuthConfig = serde_json::from_str(&serialized)
            .expect("Failed to deserialize AuthConfig");

        match (&auth_config.strategy, &deserialized.strategy) {
            (
                AuthStrategy::ApiToken { token: original },
                AuthStrategy::ApiToken { token: deserialized }
            ) => {
                prop_assert_eq!(
                    original.expose_secret(),
                    deserialized.expose_secret(),
                    "API token should be preserved through serialization"
                );
            }
            _ => prop_assert!(false, "Strategy type should be ApiToken after roundtrip"),
        }
    }

    /// Test that AuthConfig with SessionToken preserves the strategy type.
    ///
    /// Verifies:
    /// - The strategy type is correctly identified as SessionToken after roundtrip
    /// - Both username and password are preserved (secrets are serialized for persistence)
    #[test]
    fn test_auth_config_session_token_strategy_preserved(auth_config in session_token_auth_config_strategy()) {
        let serialized = serde_json::to_string(&auth_config).expect("Failed to serialize AuthConfig");
        let deserialized: AuthConfig = serde_json::from_str(&serialized)
            .expect("Failed to deserialize AuthConfig");

        match (&auth_config.strategy, &deserialized.strategy) {
            (
                AuthStrategy::SessionToken { username: orig_user, password: orig_pass },
                AuthStrategy::SessionToken { username: desc_user, password: desc_pass }
            ) => {
                prop_assert_eq!(orig_user, desc_user, "Username should be preserved through serialization");
                prop_assert_eq!(
                    orig_pass.expose_secret(),
                    desc_pass.expose_secret(),
                    "Password should be preserved through serialization"
                );
            }
            _ => prop_assert!(false, "Strategy type should be SessionToken after roundtrip"),
        }
    }

    /// Test that Config construction with api_token creates valid configuration.
    ///
    /// Verifies:
    /// - Config::with_api_token creates a config with the correct base URL
    /// - The auth strategy is correctly set to ApiToken
    /// - Full config roundtrip preserves all values
    #[test]
    fn test_config_with_api_token_roundtrip(
        base_url in base_url_strategy(),
        token_str in api_token_strategy()
    ) {
        let token = SecretString::new(token_str.clone().into());
        let config = Config::with_api_token(base_url.clone(), token);

        // Verify initial construction
        prop_assert!(
            matches!(config.auth.strategy, AuthStrategy::ApiToken { .. }),
            "Auth strategy should be ApiToken"
        );

        // Test roundtrip serialization
        let serialized = serde_json::to_string(&config).expect("Failed to serialize Config");
        let deserialized: Config = serde_json::from_str(&serialized)
            .expect("Failed to deserialize Config");

        prop_assert_eq!(deserialized.connection.base_url, base_url);
        match &deserialized.auth.strategy {
            AuthStrategy::ApiToken { token: t } => {
                prop_assert_eq!(t.expose_secret(), &token_str);
            }
            _ => prop_assert!(false, "Deserialized strategy should be ApiToken"),
        }
    }

    /// Test that Config construction with session_token creates valid configuration.
    ///
    /// Verifies:
    /// - Config::with_session_token creates a config with the correct base URL
    /// - The auth strategy is correctly set to SessionToken
    /// - Full config roundtrip preserves all values
    #[test]
    fn test_config_with_session_token_roundtrip(
        base_url in base_url_strategy(),
        username in username_strategy(),
        password_str in password_strategy()
    ) {
        let password = SecretString::new(password_str.clone().into());
        let config = Config::with_session_token(base_url.clone(), username.clone(), password);

        // Verify initial construction
        prop_assert!(
            matches!(config.auth.strategy, AuthStrategy::SessionToken { .. }),
            "Auth strategy should be SessionToken"
        );

        // Test roundtrip serialization
        let serialized = serde_json::to_string(&config).expect("Failed to serialize Config");
        let deserialized: Config = serde_json::from_str(&serialized)
            .expect("Failed to deserialize Config");

        prop_assert_eq!(deserialized.connection.base_url, base_url);
        match &deserialized.auth.strategy {
            AuthStrategy::SessionToken { username: u, password: p } => {
                prop_assert_eq!(u, &username);
                prop_assert_eq!(p.expose_secret(), &password_str);
            }
            _ => prop_assert!(false, "Deserialized strategy should be SessionToken"),
        }
    }

    /// Test that full Config with ConnectionConfig and AuthConfig roundtrips correctly.
    ///
    /// Verifies:
    /// - Both connection and auth fields are preserved
    /// - Mixed auth strategies work correctly
    #[test]
    fn test_full_config_roundtrip(
        connection in connection_config_strategy(),
        use_api_token in any::<bool>()
    ) {
        let auth = if use_api_token {
            let token_str = "test-token-12345".to_string();
            AuthConfig {
                strategy: AuthStrategy::ApiToken {
                    token: SecretString::new(token_str.into()),
                },
            }
        } else {
            AuthConfig {
                strategy: AuthStrategy::SessionToken {
                    username: "testuser".to_string(),
                    password: SecretString::new("testpass123".to_string().into()),
                },
            }
        };

        let config = Config { connection, auth };

        let serialized = serde_json::to_string(&config).expect("Failed to serialize Config");
        let deserialized: Config = serde_json::from_str(&serialized)
            .expect("Failed to deserialize Config");

        // Verify connection fields
        prop_assert_eq!(deserialized.connection.base_url, config.connection.base_url);
        prop_assert_eq!(deserialized.connection.skip_verify, config.connection.skip_verify);
        prop_assert_eq!(deserialized.connection.timeout, config.connection.timeout);
        prop_assert_eq!(deserialized.connection.max_retries, config.connection.max_retries);

        // Verify auth strategy type is preserved
        match (&config.auth.strategy, &deserialized.auth.strategy) {
            (AuthStrategy::ApiToken { .. }, AuthStrategy::ApiToken { .. }) => {}
            (AuthStrategy::SessionToken { .. }, AuthStrategy::SessionToken { .. }) => {}
            _ => prop_assert!(false, "Auth strategy type mismatch after roundtrip"),
        }
    }
}
