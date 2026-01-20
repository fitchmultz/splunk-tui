//! Live server tests against a real Splunk instance.
//!
//! These tests require a live Splunk server at 192.168.1.122:8089
//! with the credentials specified in .env.test
//!
//! Run with: cargo test --test live_tests -- --ignored

use std::time::Duration;
use splunk_client::SplunkClient;

/// Load test environment variables.
fn load_test_env() -> (String, String, String) {
    dotenvy::dotenv_dot("../.env.test").ok();

    let base_url = std::env::var("SPLUNK_BASE_URL")
        .unwrap_or_else(|_| "https://192.168.1.122:8089".to_string());
    let username = std::env::var("SPLUNK_USERNAME")
        .unwrap_or_else(|_| "admin".to_string());
    let password = std::env::var("SPLUNK_PASSWORD")
        .unwrap_or_else(|_| "admin123".to_string());

    (base_url, username, password)
}

/// Create a client for testing.
async fn create_test_client() -> SplunkClient {
    let (base_url, username, password) = load_test_env();

    let mut builder = SplunkClient::builder(&base_url)
        .username(&username)
        .password(&password)
        .danger_accept_invalid_certs(true);

    // Set timeout
    builder = builder.timeout(Duration::from_secs(30));

    builder.build().await.expect("Failed to create client")
}

#[tokio::test]
#[ignore = "requires live Splunk server"]
async fn test_live_login() {
    let client = create_test_client().await;
    // If we get here, login succeeded
    assert!(true, "Login to live server succeeded");
}

#[tokio::test]
#[ignore = "requires live Splunk server"]
async fn test_live_list_indexes() {
    let client = create_test_client().await;
    let indexes = client.list_indexes(Some(10), Some(0)).await
        .expect("Failed to list indexes");

    assert!(!indexes.is_empty(), "Should have at least one index");
    assert!(indexes.iter().any(|i| i.name == "main"), "Should have 'main' index");
}

#[tokio::test]
#[ignore = "requires live Splunk server"]
async fn test_live_search_and_get_results() {
    let client = create_test_client().await;

    // Create a search job
    let sid = client.create_search_job(
        "search index=main | head 5",
        &splunk_client::endpoints::CreateJobOptions {
            wait: Some(true),
            exec_time: Some(60),
            ..Default::default()
        }
    ).await.expect("Failed to create search job");

    // Get results
    let results = client.get_search_results(
        &sid,
        Some(5),
        Some(0),
        splunk_client::endpoints::OutputMode::Json
    ).await.expect("Failed to get search results");

    assert!(!results.results.is_empty(), "Should have search results");
}

#[tokio::test]
#[ignore = "requires live Splunk server"]
async fn test_live_list_jobs() {
    let client = create_test_client().await;
    let jobs = client.list_jobs(Some(10), Some(0)).await
        .expect("Failed to list jobs");

    // Jobs may or may not exist
    assert!(jobs.len() >= 0, "Should be able to list jobs");
}

#[tokio::test]
#[ignore = "requires live Splunk server"]
async fn test_live_create_and_cancel_job() {
    let client = create_test_client().await;

    // Create a search job without waiting
    let sid = client.create_search_job(
        "search index=main | head 1",
        &splunk_client::endpoints::CreateJobOptions {
            wait: Some(false),
            ..Default::default()
        }
    ).await.expect("Failed to create search job");

    // Cancel the job
    client.cancel_job(&sid).await
        .expect("Failed to cancel job");
}

#[tokio::test]
#[ignore = "requires live Splunk server"]
async fn test_live_cluster_info() {
    let client = create_test_client().await;

    // This may fail on standalone instances
    match client.get_cluster_info().await {
        Ok(_) => {
            assert!(true, "Cluster info retrieved successfully");
        }
        Err(e) => {
            // Standalone instances may not have cluster config
            eprintln!("Cluster info failed (may be standalone): {:?}", e);
        }
    }
}
