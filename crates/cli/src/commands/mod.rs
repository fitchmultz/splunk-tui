//! CLI command implementations.

pub mod apps;
pub mod cluster;
pub mod config;
pub mod health;
pub mod indexes;
pub mod internal_logs;
pub mod jobs;
pub mod kvstore;
pub mod license;
pub mod list_all;
pub mod logs;
pub mod saved_searches;
pub mod search;
pub mod users;

use splunk_client::AuthStrategy as ClientAuth;
use splunk_config::AuthStrategy as ConfigAuth;

/// Convert configuration authentication strategy to client authentication strategy.
///
/// This helper centralizes AuthStrategy conversion logic to avoid duplication
/// across all CLI command modules.
///
/// # Arguments
/// * `config_auth` - The config crate's AuthStrategy variant
///
/// # Returns
/// The corresponding client crate's AuthStrategy variant
pub fn convert_auth_strategy(config_auth: &ConfigAuth) -> ClientAuth {
    match config_auth {
        ConfigAuth::SessionToken { username, password } => ClientAuth::SessionToken {
            username: username.clone(),
            password: password.clone(),
        },
        ConfigAuth::ApiToken { token } => ClientAuth::ApiToken {
            token: token.clone(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::SecretString;

    #[test]
    fn test_convert_session_token_auth() {
        use secrecy::ExposeSecret;

        let config_auth = ConfigAuth::SessionToken {
            username: "test_user".to_string(),
            password: SecretString::new("test_pass".to_string().into()),
        };

        let client_auth = convert_auth_strategy(&config_auth);

        match client_auth {
            ClientAuth::SessionToken { username, password } => {
                assert_eq!(username, "test_user");
                assert_eq!(password.expose_secret(), "test_pass");
            }
            _ => unreachable!(
                "convert_auth_strategy always returns SessionToken for SessionToken input"
            ),
        }
    }

    #[test]
    fn test_convert_api_token_auth() {
        use secrecy::ExposeSecret;

        let config_auth = ConfigAuth::ApiToken {
            token: SecretString::new("test_token".to_string().into()),
        };

        let client_auth = convert_auth_strategy(&config_auth);

        match client_auth {
            ClientAuth::ApiToken { token } => {
                assert_eq!(token.expose_secret(), "test_token");
            }
            _ => unreachable!("convert_auth_strategy always returns ApiToken for ApiToken input"),
        }
    }
}
