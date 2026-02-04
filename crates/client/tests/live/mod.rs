//! Live server tests shared utilities.
//!
//! This module provides shared infrastructure for live tests that run against
//! a real Splunk instance. Tests using this module are designed to be "best effort":
//! - If required `SPLUNK_*` variables are not set, the tests no-op (pass).
//! - If the configured server is unreachable, the tests no-op (pass).
//! - If the server is reachable but requests fail (auth, API errors), the tests fail.
//!
//! Run live tests with: cargo test -p splunk-client --test live_* -- --ignored

use std::net::{TcpStream, ToSocketAddrs};
use std::sync::OnceLock;
use std::time::Duration;

use secrecy::SecretString;
use splunk_client::AuthStrategy;
use splunk_client::SplunkClient;

/// Authentication variants for live tests.
#[derive(Debug, Clone)]
pub enum LiveAuth {
    /// Session-based authentication with username and password.
    Session {
        /// Username for authentication.
        username: String,
        /// Password for authentication.
        password: String,
    },
    /// API token-based authentication.
    ApiToken {
        /// The API token.
        token: String,
    },
}

/// Test environment configuration for live tests.
#[derive(Debug, Clone)]
pub struct LiveEnv {
    /// Base URL of the Splunk server.
    pub base_url: String,
    /// Authentication strategy to use.
    pub auth: LiveAuth,
    /// Whether to skip TLS certificate verification.
    pub skip_verify: bool,
}

/// Parse the SPLUNK_SKIP_VERIFY environment variable.
fn parse_skip_verify_env() -> bool {
    use splunk_config::env_var_or_none;
    matches!(
        env_var_or_none("SPLUNK_SKIP_VERIFY").as_deref(),
        Some("1") | Some("true") | Some("TRUE") | Some("yes") | Some("YES")
    )
}

/// Check if a TCP connection can be established to the given base URL.
fn tcp_reachable(base_url: &str) -> bool {
    let without_scheme = base_url
        .strip_prefix("https://")
        .or_else(|| base_url.strip_prefix("http://"))
        .unwrap_or(base_url);
    let host_port = without_scheme.split('/').next().unwrap_or("");

    let (host, port) = match host_port.rsplit_once(':') {
        Some((h, p)) if !h.is_empty() && p.chars().all(|c| c.is_ascii_digit()) => {
            let port: u16 = match p.parse() {
                Ok(v) => v,
                Err(_) => return false,
            };
            (h, port)
        }
        _ => return false,
    };

    let addr = match (host, port).to_socket_addrs() {
        Ok(mut addrs) => match addrs.next() {
            Some(a) => a,
            None => return false,
        },
        Err(_) => return false,
    };

    TcpStream::connect_timeout(&addr, Duration::from_secs(2)).is_ok()
}

/// Check if live tests should run by verifying server reachability.
/// Results are cached for the duration of the test run.
pub fn should_run_live_tests(base_url: &str) -> bool {
    static REACHABLE: OnceLock<bool> = OnceLock::new();
    *REACHABLE.get_or_init(|| tcp_reachable(base_url))
}

/// Load test environment variables.
///
/// Returns `Some(LiveEnv)` if all required variables are set and the server
/// is reachable. Returns `None` otherwise, causing tests to skip.
pub fn load_test_env_or_skip() -> Option<LiveEnv> {
    // Resolve path to .env.test from CARGO_MANIFEST_DIR
    // CARGO_MANIFEST_DIR for this test file is crates/client
    // .env.test is at the workspace root, two levels up
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let env_path = std::path::Path::new(manifest_dir)
        .join("..")
        .join("..")
        .join(".env.test");

    // Override any pre-existing SPLUNK_* variables so `.env.test` is the source of truth,
    // but only if the file exists (CI or other environments may not have it).
    if env_path.exists()
        && let Err(e) = dotenvy::from_path_override(&env_path)
    {
        // Only log the error, don't fail - tests should be best-effort
        eprintln!(
            "Warning: failed to load .env.test from {}: {}",
            env_path.display(),
            // Use a safe error message that doesn't leak file contents
            match e {
                dotenvy::Error::Io(_) => "I/O error".to_string(),
                dotenvy::Error::LineParse(_, idx) => {
                    format!("parse error at position {}", idx)
                }
                _ => "unknown error".to_string(),
            }
        );
    }

    use splunk_config::env_var_or_none;
    let base_url = match env_var_or_none("SPLUNK_BASE_URL") {
        Some(v) => v,
        None => {
            eprintln!("Skipping live tests: SPLUNK_BASE_URL is not set.");
            return None;
        }
    };
    let auth = if let Some(token) = env_var_or_none("SPLUNK_API_TOKEN") {
        LiveAuth::ApiToken { token }
    } else {
        let username = match env_var_or_none("SPLUNK_USERNAME") {
            Some(v) => v,
            None => {
                eprintln!("Skipping live tests: SPLUNK_USERNAME is not set.");
                return None;
            }
        };
        let password = match env_var_or_none("SPLUNK_PASSWORD") {
            Some(v) => v,
            None => {
                eprintln!("Skipping live tests: SPLUNK_PASSWORD is not set.");
                return None;
            }
        };
        LiveAuth::Session { username, password }
    };

    let skip_verify = parse_skip_verify_env();

    if !should_run_live_tests(&base_url) {
        eprintln!("Skipping live tests: Splunk server is unreachable.");
        return None;
    }

    Some(LiveEnv {
        base_url,
        auth,
        skip_verify,
    })
}

/// Create a client for testing.
///
/// Returns `Some(SplunkClient)` if environment is configured and server is reachable.
/// Returns `None` otherwise, causing tests to skip.
pub fn create_test_client_or_skip() -> Option<SplunkClient> {
    let env = load_test_env_or_skip()?;

    let auth_strategy = match env.auth {
        LiveAuth::Session { username, password } => AuthStrategy::SessionToken {
            username,
            password: SecretString::new(password.into()),
        },
        LiveAuth::ApiToken { token } => AuthStrategy::ApiToken {
            token: SecretString::new(token.into()),
        },
    };

    Some(
        SplunkClient::builder()
            .base_url(env.base_url)
            .auth_strategy(auth_strategy)
            .skip_verify(env.skip_verify)
            .build()
            .expect("Failed to create client"),
    )
}

/// Generate a unique name with the given prefix.
///
/// Uses the current timestamp in milliseconds to ensure uniqueness.
pub fn unique_name(prefix: &str) -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_millis();
    format!("{prefix}_{ts}")
}

/// RAII guard for cleaning up saved searches after tests.
///
/// When dropped, this will attempt to delete the saved search with the given name.
/// This is a best-effort cleanup that runs asynchronously in the current runtime.
pub struct SavedSearchCleanup {
    name: String,
    base_url: String,
    auth_strategy: AuthStrategy,
    skip_verify: bool,
}

impl SavedSearchCleanup {
    /// Create a new cleanup guard for the given saved search name.
    ///
    /// Returns `None` if the test environment is not configured.
    pub fn new(name: String) -> Option<Self> {
        let env = load_test_env_or_skip()?;
        let auth_strategy = match env.auth {
            LiveAuth::Session { username, password } => AuthStrategy::SessionToken {
                username,
                password: SecretString::new(password.into()),
            },
            LiveAuth::ApiToken { token } => AuthStrategy::ApiToken {
                token: SecretString::new(token.into()),
            },
        };

        Some(Self {
            name,
            base_url: env.base_url,
            auth_strategy,
            skip_verify: env.skip_verify,
        })
    }
}

impl Drop for SavedSearchCleanup {
    fn drop(&mut self) {
        let Ok(handle) = tokio::runtime::Handle::try_current() else {
            return;
        };
        let name = std::mem::take(&mut self.name);
        if name.is_empty() {
            return;
        }
        let base_url = self.base_url.clone();
        let auth_strategy = self.auth_strategy.clone();
        let skip_verify = self.skip_verify;

        handle.spawn(async move {
            let mut client = match SplunkClient::builder()
                .base_url(base_url)
                .auth_strategy(auth_strategy)
                .skip_verify(skip_verify)
                .build()
            {
                Ok(c) => c,
                Err(_) => return,
            };
            let _ = client.delete_saved_search(&name).await;
        });
    }
}
