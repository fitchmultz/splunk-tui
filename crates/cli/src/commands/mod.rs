//! CLI command implementations.

pub mod apps;
pub mod cluster;
pub mod config;
pub mod health;
pub mod indexes;
pub mod jobs;
pub mod kvstore;
pub mod license;
pub mod list_all;
pub mod logs;
pub mod search;
pub mod users;

use splunk_client::AuthStrategy as ClientAuth;
use splunk_config::AuthStrategy as ConfigAuth;

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
