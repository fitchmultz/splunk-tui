//! Authentication strategy building for profile-based connections.
//!
//! Responsibilities:
//! - Convert profile configuration into client authentication strategy.
//! - Resolve credentials from keyring when configured.
//! - Provide clear error messages for credential issues.
//!
//! Does NOT handle:
//! - Direct API token or password handling from CLI args (see config loading).
//! - Session management or token refresh.
//!
//! Invariants:
//! - API token is preferred over username/password when both are present.
//! - Keyring resolution failures produce descriptive error messages.

use secrecy::{ExposeSecret, SecretString};
use splunk_client::AuthStrategy;

/// Build authentication strategy from profile configuration.
///
/// Returns `Ok(AuthStrategy)` when credentials are successfully resolved,
/// or `Err(String)` with a descriptive error message when credentials are
/// missing or fail to resolve.
pub fn build_auth_strategy_from_profile(
    profile: &splunk_config::types::ProfileConfig,
) -> Result<AuthStrategy, String> {
    // Prefer API token if available
    if let Some(ref token) = profile.api_token {
        match token.resolve() {
            Ok(resolved) => {
                return Ok(AuthStrategy::ApiToken {
                    token: SecretString::from(resolved.expose_secret()),
                });
            }
            Err(e) => {
                return Err(format!("Failed to resolve API token from keyring: {}", e));
            }
        }
    }

    // Check for partial username/password configuration
    match (&profile.username, &profile.password) {
        (Some(username), Some(password)) => match password.resolve() {
            Ok(resolved) => Ok(AuthStrategy::SessionToken {
                username: username.clone(),
                password: SecretString::from(resolved.expose_secret()),
            }),
            Err(e) => Err(format!("Failed to resolve password from keyring: {}", e)),
        },
        (Some(_), None) => Err("Username configured but password is missing".to_string()),
        (None, Some(_)) => Err("Password configured but username is missing".to_string()),
        (None, None) => {
            Err("No credentials configured (expected api_token or username/password)".to_string())
        }
    }
}
