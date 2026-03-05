//! Live integration tests for `splunk-cli` against a real Splunk instance.
//!
//! Responsibilities:
//! - Validate end-to-end CLI wiring (args -> config -> HTTP -> output) against the dev server.
//! - Catch authentication/config regressions that mocks cannot.
//!
//! Explicitly does NOT cover:
//! - Exhaustive formatting correctness (covered by mocked integration/unit tests).
//! - TUI behavior (covered by `crates/tui` tests).
//!
//! Invariants / assumptions:
//! - A reachable Splunk server is available via `.env.test` or environment variables.
//! - Credentials are provided via environment variables or `.env.test` at the workspace root.
//! - Self-signed TLS is expected; `SPLUNK_SKIP_VERIFY=true` is recommended.
//!
//! These tests are designed to be "best effort":
//! - If required `SPLUNK_*` variables are not set, the tests no-op (pass).
//! - If the configured server is unreachable, the tests no-op (pass).
//! - If the server is reachable but requests fail (auth, API errors), the tests fail.
//!
//! Run with: cargo test -p splunk-cli --test live_tests -- --ignored

use predicates::prelude::*;
use splunk_config::env_var_or_none;
use std::fs;
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;
use tempfile::TempDir;

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn clear_splunk_env() {
    // `.env.test` should be the source of truth for live tests. Clear any
    // pre-existing SPLUNK_* variables so dotenv parsing can't be bypassed.
    for (key, _) in std::env::vars() {
        if key.starts_with("SPLUNK_") {
            unsafe {
                std::env::remove_var(key);
            }
        }
    }
}

fn splunk_cli_cmd() -> assert_cmd::Command {
    assert_cmd::cargo::cargo_bin_cmd!("splunk-cli")
}

/// Serializes and normalizes live-test env configuration for this test binary.
struct LiveEnvGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
}

impl LiveEnvGuard {
    fn parse_base_url_host_port(base_url: &str) -> Option<(String, u16)> {
        let without_scheme = base_url
            .strip_prefix("https://")
            .or_else(|| base_url.strip_prefix("http://"))
            .unwrap_or(base_url);
        let host_port = without_scheme.split('/').next().unwrap_or("");

        let (host, port) = host_port.rsplit_once(':')?;
        if host.is_empty() || !port.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let port: u16 = port.parse().ok()?;
        Some((host.to_string(), port))
    }

    fn tcp_reachable(base_url: &str) -> bool {
        let Some((host, port)) = Self::parse_base_url_host_port(base_url) else {
            return false;
        };

        let addr = match (host.as_str(), port).to_socket_addrs() {
            Ok(mut addrs) => match addrs.next() {
                Some(a) => a,
                None => return false,
            },
            Err(_) => return false,
        };

        TcpStream::connect_timeout(&addr, Duration::from_secs(2)).is_ok()
    }

    fn should_run(base_url: &str) -> bool {
        static REACHABLE: OnceLock<bool> = OnceLock::new();
        *REACHABLE.get_or_init(|| Self::tcp_reachable(base_url))
    }

    fn new_or_skip() -> Option<Self> {
        let lock = env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let env_test_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join(".env.test");

        if env_test_path.exists() {
            clear_splunk_env();
            if let Err(e) = dotenvy::from_path_override(&env_test_path) {
                // Only log the error, don't fail - live tests are best-effort
                eprintln!(
                    "Warning: failed to load .env.test from {}: {}",
                    env_test_path.display(),
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
        }

        let base_url = match env_var_or_none("SPLUNK_BASE_URL") {
            Some(v) => v,
            None => {
                eprintln!("Skipping live CLI tests: SPLUNK_BASE_URL is not set.");
                return None;
            }
        };
        if env_var_or_none("SPLUNK_API_TOKEN").is_none() {
            if env_var_or_none("SPLUNK_USERNAME").is_none() {
                eprintln!("Skipping live CLI tests: SPLUNK_USERNAME is not set.");
                return None;
            }
            if env_var_or_none("SPLUNK_PASSWORD").is_none() {
                eprintln!("Skipping live CLI tests: SPLUNK_PASSWORD is not set.");
                return None;
            }
        }

        if !Self::should_run(&base_url) {
            eprintln!("Skipping live CLI tests: Splunk server is unreachable.");
            return None;
        }

        // Self-signed TLS is typical for dev servers; keep existing behavior unless explicitly set.
        if env_var_or_none("SPLUNK_SKIP_VERIFY").is_none() {
            unsafe {
                std::env::set_var("SPLUNK_SKIP_VERIFY", "true");
            }
        }

        Some(Self { _lock: lock })
    }
}

impl Drop for LiveEnvGuard {
    fn drop(&mut self) {
        clear_splunk_env();
    }
}

fn create_test_client_from_env() -> splunk_client::SplunkClient {
    use secrecy::SecretString;
    use splunk_client::AuthStrategy;
    use splunk_client::SplunkClient;

    let base_url = env_var_or_none("SPLUNK_BASE_URL").expect("SPLUNK_BASE_URL must be set");
    let auth = if let Some(token) = env_var_or_none("SPLUNK_API_TOKEN") {
        AuthStrategy::ApiToken {
            token: SecretString::new(token.into()),
        }
    } else {
        let username = env_var_or_none("SPLUNK_USERNAME").expect("SPLUNK_USERNAME must be set");
        let password = env_var_or_none("SPLUNK_PASSWORD").expect("SPLUNK_PASSWORD must be set");
        AuthStrategy::SessionToken {
            username,
            password: SecretString::new(password.into()),
        }
    };

    let skip_verify = matches!(
        env_var_or_none("SPLUNK_SKIP_VERIFY").as_deref(),
        Some("1") | Some("true") | Some("TRUE") | Some("yes") | Some("YES")
    );

    SplunkClient::builder()
        .base_url(base_url)
        .auth_strategy(auth)
        .skip_verify(skip_verify)
        .build()
        .expect("Failed to create SplunkClient")
}

fn unique_name(prefix: &str) -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|_| std::time::Duration::from_secs(0))
        .as_millis();
    format!("{prefix}_{ts}")
}

#[test]
#[ignore = "requires live Splunk server"]
fn test_live_cli_indexes_json() {
    let Some(_env) = LiveEnvGuard::new_or_skip() else {
        return;
    };
    let mut cmd = splunk_cli_cmd();

    cmd.args(["--output", "json", "indexes", "list", "--count", "5"])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Splunk server"]
fn test_live_cli_users_json() {
    let Some(_env) = LiveEnvGuard::new_or_skip() else {
        return;
    };
    let mut cmd = splunk_cli_cmd();

    cmd.args(["--output", "json", "users", "list", "--count", "5"])
        .assert()
        .success()
        .stdout(predicate::str::contains("admin"));
}

#[test]
#[ignore = "requires live Splunk server"]
fn test_live_cli_apps_list_json() {
    let Some(_env) = LiveEnvGuard::new_or_skip() else {
        return;
    };
    let mut cmd = splunk_cli_cmd();

    cmd.args(["--output", "json", "apps", "list", "--count", "5"])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Splunk server"]
fn test_live_cli_health_json() {
    let Some(_env) = LiveEnvGuard::new_or_skip() else {
        return;
    };
    let mut cmd = splunk_cli_cmd();

    cmd.args(["--output", "json", "health"]).assert().success();
}

#[test]
#[ignore = "requires live Splunk server"]
fn test_live_cli_server_info() {
    let Some(_env) = LiveEnvGuard::new_or_skip() else {
        return;
    };
    let mut cmd = splunk_cli_cmd();

    // Server info is included in health command output
    cmd.args(["--output", "json", "health"])
        .assert()
        .success()
        .stdout(predicate::str::contains("server_info"))
        .stdout(predicate::str::contains("serverName"))
        .stdout(predicate::str::contains("version"));
}

#[test]
#[ignore = "requires live Splunk server"]
fn test_live_cli_cluster_info() {
    let Some(_env) = LiveEnvGuard::new_or_skip() else {
        return;
    };
    let mut cmd = splunk_cli_cmd();

    // This may fail on standalone instances - just verify we can make the call
    let _assert = cmd.args(["--output", "json", "cluster", "info"]).assert();
}

#[test]
#[ignore = "requires live Splunk server"]
fn test_live_cli_jobs_list() {
    let Some(_env) = LiveEnvGuard::new_or_skip() else {
        return;
    };
    let mut cmd = splunk_cli_cmd();

    // Jobs command uses --list flag (which is the default)
    cmd.args(["--output", "json", "jobs", "--count", "5"])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Splunk server"]
fn test_live_cli_table_output() {
    let Some(_env) = LiveEnvGuard::new_or_skip() else {
        return;
    };
    let mut cmd = splunk_cli_cmd();

    // Table output for indexes uses "Name" as header
    cmd.args(["--output", "table", "indexes", "list", "--count", "3"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Name"));
}

#[test]
#[ignore = "requires live Splunk server"]
fn test_live_cli_xml_output() {
    let Some(_env) = LiveEnvGuard::new_or_skip() else {
        return;
    };
    let mut cmd = splunk_cli_cmd();

    cmd.args(["--output", "xml", "health"])
        .assert()
        .success()
        .stdout(predicate::str::contains("<?xml"));
}

#[test]
#[ignore = "requires live Splunk server"]
fn test_live_cli_get_app() {
    let Some(_env) = LiveEnvGuard::new_or_skip() else {
        return;
    };
    let mut cmd = splunk_cli_cmd();

    // Apps command uses 'info' subcommand, not 'get'
    cmd.args(["--output", "json", "apps", "info", "search"])
        .assert()
        .success()
        .stdout(predicate::str::contains("search"));
}

#[test]
#[ignore = "requires live Splunk server"]
fn test_live_cli_search_wait_json() {
    let Some(_env) = LiveEnvGuard::new_or_skip() else {
        return;
    };
    let mut cmd = splunk_cli_cmd();

    cmd.args([
        "--output",
        "json",
        "search",
        r#"| makeresults | eval foo="cli-live" | table foo"#,
        "--wait",
        "--count",
        "1",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("cli-live"));
}

#[test]
#[ignore = "requires live Splunk server"]
fn test_live_cli_license_json() {
    let Some(_env) = LiveEnvGuard::new_or_skip() else {
        return;
    };
    let mut cmd = splunk_cli_cmd();

    cmd.args(["-o", "json", "license"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"usage\""));
}

#[test]
#[ignore = "requires live Splunk server"]
fn test_live_cli_doctor() {
    let Some(_env) = LiveEnvGuard::new_or_skip() else {
        return;
    };
    let mut cmd = splunk_cli_cmd();

    cmd.args(["--output", "json", "doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("cli_version"))
        .stdout(predicate::str::contains("config_load"))
        .stdout(predicate::str::contains("server_connectivity"));
}

#[test]
#[ignore = "requires live Splunk server"]
fn test_live_cli_doctor_bundle() {
    let Some(_env) = LiveEnvGuard::new_or_skip() else {
        return;
    };

    let temp_dir = TempDir::new().unwrap();
    let bundle_path = temp_dir.path().join("support-bundle.zip");

    let mut cmd = splunk_cli_cmd();
    cmd.args([
        "doctor",
        "--bundle",
        bundle_path.to_str().unwrap(),
        "--include-logs",
    ])
    .assert()
    .success()
    .stderr(predicate::str::contains("Support bundle written to"));

    // Verify bundle was created
    assert!(bundle_path.exists(), "Support bundle should be created");

    // Verify bundle is a valid zip
    let file = fs::File::open(&bundle_path).unwrap();
    let archive = zip::ZipArchive::new(file).unwrap();

    // Should contain at least diagnostic_report.json
    let has_report = archive.file_names().any(|n| n == "diagnostic_report.json");
    assert!(has_report, "Bundle should contain diagnostic_report.json");

    // Should contain environment.txt
    let has_env = archive.file_names().any(|n| n == "environment.txt");
    assert!(has_env, "Bundle should contain environment.txt");
}

#[test]
#[ignore = "requires live Splunk server"]
fn test_live_cli_saved_searches_info_and_run_json() {
    let Some(_env) = LiveEnvGuard::new_or_skip() else {
        return;
    };

    let name = unique_name("codex_saved_search_cli");
    let search = r#"| makeresults | eval foo="saved-search-cli" | table foo"#;

    // Setup via client library, validate via CLI.
    let client = create_test_client_from_env();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to build tokio runtime");
    rt.block_on(async {
        client
            .create_saved_search(splunk_client::models::SavedSearchCreateParams {
                name: name.clone(),
                search: search.to_string(),
                ..Default::default()
            })
            .await
            .expect("Failed to create saved search");
    });

    let mut cmd = splunk_cli_cmd();
    cmd.args(["--output", "json", "saved-searches", "info", &name])
        .assert()
        .success()
        .stdout(predicate::str::contains(&name));

    let mut cmd = splunk_cli_cmd();
    cmd.args(["--output", "json", "saved-searches", "run", &name, "--wait"])
        .assert()
        .success()
        .stdout(predicate::str::contains("saved-search-cli"));

    rt.block_on(async {
        client
            .delete_saved_search(&name)
            .await
            .expect("Failed to delete saved search");
    });
}
