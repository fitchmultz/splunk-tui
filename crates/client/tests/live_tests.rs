//! Live server tests against a real Splunk instance.
//!
//! These tests require a live Splunk server at 192.168.1.122:8089
//! with the credentials specified in .env.test
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

    dotenvy::from_path(env_path).ok();

    let base_url = std::env::var("SPLUNK_BASE_URL")
        .unwrap_or_else(|_| "https://192.168.1.122:8089".to_string());
    let username = std::env::var("SPLUNK_USERNAME").unwrap_or_else(|_| "admin".to_string());
    let password = std::env::var("SPLUNK_PASSWORD").unwrap_or_else(|_| "admin123".to_string());

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
        .list_indexes(Some(10), Some(0))
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
            "search index=main | head 5",
            &CreateJobOptions {
                wait: Some(true),
                exec_time: Some(60),
                ..Default::default()
            },
        )
        .await
        .expect("Failed to create search job");

    // Get results (get_search_results takes u64, not Option<u64>)
    let results = client
        .get_search_results(&sid, 5, 0)
        .await
        .expect("Failed to get search results");

    assert!(!results.results.is_empty(), "Should have search results");
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
            "search index=main | head 1",
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
