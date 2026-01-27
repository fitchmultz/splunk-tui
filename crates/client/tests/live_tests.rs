//! Live server tests against a real Splunk instance.
//!
//! These tests require a reachable Splunk server configured via environment
//! variables or `.env.test` (workspace root).
//!
//! These tests are designed to be "best effort":
//! - If required `SPLUNK_*` variables are not set, the tests no-op (pass).
//! - If the configured server is unreachable, the tests no-op (pass).
//! - If the server is reachable but requests fail (auth, API errors), the tests fail.
//!
//! Run with: cargo test -p splunk-client --test live_tests -- --ignored

use std::net::{TcpStream, ToSocketAddrs};
use std::sync::OnceLock;
use std::time::Duration;

use secrecy::SecretString;
use splunk_client::AuthStrategy;
use splunk_client::SplunkClient;
use splunk_client::endpoints::search::CreateJobOptions;

#[derive(Debug, Clone)]
enum LiveAuth {
    Session { username: String, password: String },
    ApiToken { token: String },
}

#[derive(Debug, Clone)]
struct LiveEnv {
    base_url: String,
    auth: LiveAuth,
    skip_verify: bool,
}

fn parse_skip_verify_env() -> bool {
    use splunk_config::ConfigLoader;
    matches!(
        ConfigLoader::env_var_or_none("SPLUNK_SKIP_VERIFY").as_deref(),
        Some("1") | Some("true") | Some("TRUE") | Some("yes") | Some("YES")
    )
}

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

fn should_run_live_tests(base_url: &str) -> bool {
    static REACHABLE: OnceLock<bool> = OnceLock::new();
    *REACHABLE.get_or_init(|| tcp_reachable(base_url))
}

/// Load test environment variables.
fn load_test_env_or_skip() -> Option<LiveEnv> {
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
    if env_path.exists() {
        dotenvy::from_path_override(env_path).ok();
    }

    use splunk_config::ConfigLoader;
    let base_url = match ConfigLoader::env_var_or_none("SPLUNK_BASE_URL") {
        Some(v) => v,
        None => {
            eprintln!("Skipping live tests: SPLUNK_BASE_URL is not set.");
            return None;
        }
    };
    let auth = if let Some(token) = ConfigLoader::env_var_or_none("SPLUNK_API_TOKEN") {
        LiveAuth::ApiToken { token }
    } else {
        let username = match ConfigLoader::env_var_or_none("SPLUNK_USERNAME") {
            Some(v) => v,
            None => {
                eprintln!("Skipping live tests: SPLUNK_USERNAME is not set.");
                return None;
            }
        };
        let password = match ConfigLoader::env_var_or_none("SPLUNK_PASSWORD") {
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
fn create_test_client_or_skip() -> Option<SplunkClient> {
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

fn unique_name(prefix: &str) -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_millis();
    format!("{prefix}_{ts}")
}

struct SavedSearchCleanup {
    name: String,
    base_url: String,
    auth_strategy: AuthStrategy,
    skip_verify: bool,
}

impl SavedSearchCleanup {
    fn new(name: String) -> Option<Self> {
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

#[tokio::test]
#[ignore = "requires live Splunk server"]
async fn test_live_login() {
    let Some(mut client) = create_test_client_or_skip() else {
        return;
    };
    // Login by calling any authenticated method
    // If this succeeds without error, login worked
    client
        .list_indexes(Some(1), Some(0))
        .await
        .expect("Login failed");
}

#[tokio::test]
#[ignore = "requires live Splunk server"]
async fn test_live_list_indexes() {
    let Some(mut client) = create_test_client_or_skip() else {
        return;
    };
    let indexes = client
        .list_indexes(Some(500), Some(0))
        .await
        .expect("Failed to list indexes");

    assert!(!indexes.is_empty(), "Should have at least one index");
    assert!(
        indexes.iter().any(|i| i.name == "main"),
        "Should have 'main' index"
    );
}

#[tokio::test]
#[ignore = "requires live Splunk server"]
async fn test_live_search_and_get_results() {
    let Some(mut client) = create_test_client_or_skip() else {
        return;
    };

    // Create a search job
    let sid = client
        .create_search_job(
            r#"| makeresults | eval foo="bar" | table foo"#,
            &CreateJobOptions {
                wait: Some(true),
                exec_time: Some(60),
                ..Default::default()
            },
        )
        .await
        .expect("Failed to create search job");

    // Even with `wait=true`, Splunk can briefly return an empty results page.
    // Poll until we see the expected row, or time out.
    let mut last_total = None;
    for _ in 0..20 {
        // get_search_results takes u64, not Option<u64>
        let results = client
            .get_search_results(&sid, 5, 0)
            .await
            .expect("Failed to get search results");
        last_total = results.total;

        if let Some(first) = results.results.first()
            && first.get("foo").and_then(|v| v.as_str()) == Some("bar")
        {
            return;
        }

        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }

    panic!(
        "Search results did not contain expected foo=bar row (last total={:?})",
        last_total
    );
}

#[tokio::test]
#[ignore = "requires live Splunk server"]
async fn test_live_create_status_and_delete_job() {
    let Some(mut client) = create_test_client_or_skip() else {
        return;
    };

    let sid = client
        .create_search_job(
            r#"| makeresults | eval foo="job" | table foo"#,
            &CreateJobOptions {
                wait: Some(false),
                exec_time: Some(60),
                ..Default::default()
            },
        )
        .await
        .expect("Failed to create search job");

    let _status = client
        .get_job_status(&sid)
        .await
        .expect("Failed to get job status");

    client.delete_job(&sid).await.expect("Failed to delete job");
}

#[tokio::test]
#[ignore = "requires live Splunk server"]
async fn test_live_list_jobs() {
    let Some(mut client) = create_test_client_or_skip() else {
        return;
    };
    // Just verify we can list jobs successfully
    let _jobs = client
        .list_jobs(Some(10), Some(0))
        .await
        .expect("Failed to list jobs");
}

#[tokio::test]
#[ignore = "requires live Splunk server"]
async fn test_live_create_and_cancel_job() {
    let Some(mut client) = create_test_client_or_skip() else {
        return;
    };

    // Create a search job without waiting
    let sid = client
        .create_search_job(
            r#"| makeresults | eval foo="cancel" | table foo"#,
            &CreateJobOptions {
                wait: Some(false),
                ..Default::default()
            },
        )
        .await
        .expect("Failed to create search job");

    // Cancel the job
    client.cancel_job(&sid).await.expect("Failed to cancel job");
}

#[tokio::test]
#[ignore = "requires live Splunk server"]
async fn test_live_cluster_info() {
    let Some(mut client) = create_test_client_or_skip() else {
        return;
    };

    // This may fail on standalone instances - just verify we can make the call
    let _result = client.get_cluster_info().await;
}

#[tokio::test]
#[ignore = "requires live Splunk server"]
async fn test_live_get_server_info() {
    let Some(mut client) = create_test_client_or_skip() else {
        return;
    };
    let info = client
        .get_server_info()
        .await
        .expect("Failed to get server info");

    assert!(
        !info.server_name.is_empty(),
        "server_name should not be empty"
    );
    assert!(!info.version.is_empty(), "version should not be empty");
    assert!(!info.build.is_empty(), "build should not be empty");
}

#[tokio::test]
#[ignore = "requires live Splunk server"]
async fn test_live_get_health() {
    let Some(mut client) = create_test_client_or_skip() else {
        return;
    };
    let health = client.get_health().await.expect("Failed to get health");

    assert!(!health.health.is_empty(), "health should not be empty");
}

#[tokio::test]
#[ignore = "requires live Splunk server"]
async fn test_live_get_license_usage() {
    let Some(mut client) = create_test_client_or_skip() else {
        return;
    };
    let usage = client
        .get_license_usage()
        .await
        .expect("Failed to get license usage");

    assert!(!usage.is_empty(), "license usage should not be empty");
    assert!(
        usage.iter().all(|u| u.quota > 0),
        "all license entries should have a quota"
    );
}

#[tokio::test]
#[ignore = "requires live Splunk server"]
async fn test_live_list_license_pools_and_stacks() {
    let Some(mut client) = create_test_client_or_skip() else {
        return;
    };

    let _pools = client
        .list_license_pools()
        .await
        .expect("Failed to list license pools");
    let _stacks = client
        .list_license_stacks()
        .await
        .expect("Failed to list license stacks");
}

#[tokio::test]
#[ignore = "requires live Splunk server"]
async fn test_live_get_kvstore_status() {
    let Some(mut client) = create_test_client_or_skip() else {
        return;
    };
    let status = client
        .get_kvstore_status()
        .await
        .expect("Failed to get KVStore status");

    assert!(
        !status.current_member.host.is_empty(),
        "KVStore current member host should not be empty"
    );
    assert!(
        status.current_member.port > 0,
        "KVStore current member port should be > 0"
    );
}

#[tokio::test]
#[ignore = "requires live Splunk server"]
async fn test_live_check_log_parsing_health() {
    let Some(mut client) = create_test_client_or_skip() else {
        return;
    };
    let parsing = client
        .check_log_parsing_health()
        .await
        .expect("Failed to check log parsing health");

    assert_eq!(
        parsing.total_errors,
        parsing.errors.len(),
        "total_errors should match the number of error entries"
    );
    assert!(
        !parsing.time_window.is_empty(),
        "time_window should not be empty"
    );
    assert_eq!(
        parsing.is_healthy,
        parsing.total_errors == 0,
        "is_healthy should reflect whether errors were found"
    );
}

#[tokio::test]
#[ignore = "requires live Splunk server"]
async fn test_live_get_internal_logs() {
    let Some(mut client) = create_test_client_or_skip() else {
        return;
    };
    let logs = client
        .get_internal_logs(20, Some("-15m"))
        .await
        .expect("Failed to get internal logs");

    assert!(
        logs.len() <= 20,
        "returned logs should not exceed requested count"
    );
    assert!(
        logs.iter()
            .all(|l| !l.time.is_empty() && !l.message.is_empty()),
        "log entries should have time and message"
    );
}

#[tokio::test]
#[ignore = "requires live Splunk server"]
async fn test_live_list_apps_and_users_and_saved_searches() {
    let Some(mut client) = create_test_client_or_skip() else {
        return;
    };

    let apps = client
        .list_apps(Some(10), Some(0))
        .await
        .expect("Failed to list apps");
    assert!(!apps.is_empty(), "apps list should not be empty");

    let users = client
        .list_users(Some(50), Some(0))
        .await
        .expect("Failed to list users");
    assert!(
        users.iter().any(|u| u.name == "admin"),
        "users should include an 'admin' user"
    );

    // Saved searches may be empty depending on instance configuration; this is a smoke test.
    let _saved_searches = client
        .list_saved_searches()
        .await
        .expect("Failed to list saved searches");
}

#[tokio::test]
#[ignore = "requires live Splunk server"]
async fn test_live_create_list_and_delete_saved_search() {
    let Some(mut client) = create_test_client_or_skip() else {
        return;
    };

    let name = unique_name("codex_saved_search");
    let _cleanup = SavedSearchCleanup::new(name.clone());

    let search = r#"| makeresults | eval foo="saved-search" | table foo"#;
    client
        .create_saved_search(&name, search)
        .await
        .expect("Failed to create saved search");

    let searches = client
        .list_saved_searches()
        .await
        .expect("Failed to list saved searches");
    let created = searches
        .iter()
        .find(|s| s.name == name)
        .expect("created saved search should be listed");
    assert_eq!(
        created.search, search,
        "created saved search should retain its search query"
    );

    client
        .delete_saved_search(&name)
        .await
        .expect("Failed to delete saved search");
}
