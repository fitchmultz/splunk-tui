//! Splunk client creation and authentication.
//!
//! Responsibilities:
//! - Create and authenticate Splunk client instances from configuration.
//! - Map configuration auth strategy to client auth strategy.
//!
//! Does NOT handle:
//! - Configuration loading (see `runtime::config`).
//! - Terminal state management (see `runtime::terminal`).
//!
//! Invariants / Assumptions:
//! - The provided config has valid base_url and auth credentials.
//! - Session token auth requires calling `login()` before use.

use anyhow::Result;
use splunk_client::{AuthStrategy, SplunkClient};
use splunk_config::{AuthStrategy as ConfigAuthStrategy, Config};

/// Create and authenticate a new Splunk client.
///
/// This function builds a SplunkClient from the provided configuration,
/// mapping the config auth strategy to the client auth strategy. For
/// session token authentication, it also performs the initial login.
///
/// # Arguments
///
/// * `config` - The loaded configuration containing connection and auth settings
///
/// # Errors
///
/// Returns an error if client construction fails or if login fails for
/// session token authentication.
pub async fn create_client(config: &Config) -> Result<SplunkClient> {
    let auth_strategy = match &config.auth.strategy {
        ConfigAuthStrategy::SessionToken { username, password } => AuthStrategy::SessionToken {
            username: username.clone(),
            password: password.clone(),
        },
        ConfigAuthStrategy::ApiToken { token } => AuthStrategy::ApiToken {
            token: token.clone(),
        },
    };

    let mut client = SplunkClient::builder()
        .base_url(config.connection.base_url.clone())
        .auth_strategy(auth_strategy)
        .skip_verify(config.connection.skip_verify)
        .timeout(config.connection.timeout)
        .session_ttl_seconds(config.connection.session_ttl_seconds)
        .session_expiry_buffer_seconds(config.connection.session_expiry_buffer_seconds)
        .build()?;

    // Login if using session token
    if !client.is_api_token_auth() {
        client.login().await?;
    }

    Ok(client)
}
