//! Live server tests against a real Splunk instance.
//!
//! These tests require a reachable Splunk server configured via environment
//! variables or `.env.test` (workspace root).
//!
//! Run with: cargo test -p splunk-client --test live_tests -- --ignored

use secrecy::SecretString;
use splunk_client::AuthStrategy;
use splunk_client::SplunkClient;
use splunk_client::endpoints::search::CreateJobOptions;

/// Load test environment variables.
fn load_test_env() -> (String, String, String) {
    // Resolve path to .env.test from CARGO_MANIFEST_DIR
    // CARGO_MANIFEST_DIR for this test file is crates/client
    // .env.test is at the workspace root, two levels up
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let env_path = std::path::Path::new(manifest_dir)
        .join("..")
        .join("..")
        .join(".env.test");

    // Override any pre-existing SPLUNK_* variables so `.env.test` is the source of truth.
    dotenvy::from_path_override(env_path).ok();

    let base_url = std::env::var("SPLUNK_BASE_URL")
        .expect("SPLUNK_BASE_URL must be set (use .env.test or environment variables)");
    let username = std::env::var("SPLUNK_USERNAME")
        .expect("SPLUNK_USERNAME must be set (use .env.test or environment variables)");
    let password = std::env::var("SPLUNK_PASSWORD")
        .expect("SPLUNK_PASSWORD must be set (use .env.test or environment variables)");

    (base_url, username, password)
}

/// Create a client for testing.
fn create_test_client() -> SplunkClient {
    let (base_url, username, password) = load_test_env();

    let auth_strategy = AuthStrategy::SessionToken {
        username,
        password: SecretString::new(password.into()),
    };

    SplunkClient::builder()
        .base_url(base_url)
        .auth_strategy(auth_strategy)
        .skip_verify(true)
        .build()
        .expect("Failed to create client")
}

#[tokio::test]
#[ignore = "requires live Splunk server"]
async fn test_live_login() {
    let mut client = create_test_client();
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
    let mut client = create_test_client();
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
    let mut client = create_test_client();

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
async fn test_live_list_jobs() {
    let mut client = create_test_client();
    // Just verify we can list jobs successfully
    let _jobs = client
        .list_jobs(Some(10), Some(0))
        .await
        .expect("Failed to list jobs");
}

#[tokio::test]
#[ignore = "requires live Splunk server"]
async fn test_live_create_and_cancel_job() {
    let mut client = create_test_client();

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
    let mut client = create_test_client();

    // This may fail on standalone instances - just verify we can make the call
    let _result = client.get_cluster_info().await;
}

#[tokio::test]
#[ignore = "requires live Splunk server"]
async fn test_live_get_server_info() {
    let mut client = create_test_client();
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
    let mut client = create_test_client();
    let health = client.get_health().await.expect("Failed to get health");

    assert!(!health.health.is_empty(), "health should not be empty");
}

#[tokio::test]
#[ignore = "requires live Splunk server"]
async fn test_live_get_license_usage() {
    let mut client = create_test_client();
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
async fn test_live_get_kvstore_status() {
    let mut client = create_test_client();
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
    let mut client = create_test_client();
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
    let mut client = create_test_client();
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
    let mut client = create_test_client();

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
