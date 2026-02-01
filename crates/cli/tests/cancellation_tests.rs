//! Integration tests for graceful Ctrl+C/SIGINT handling.
//!
//! These tests are Unix-only because they send SIGINT to child process.
//! We assert:
//! - exit code is 130
//! - stderr contains cancellation message

#![cfg(unix)]

use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Notify;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TEST_QUERY: &str = "search index=_internal | head 1";

fn splunk_cli_bin() -> &'static std::path::Path {
    assert_cmd::cargo::cargo_bin!("splunk-cli")
}

fn send_sigint(pid: u32) {
    // SAFETY: standard Unix kill syscall
    unsafe {
        libc::kill(pid as i32, libc::SIGINT);
    }
}

#[tokio::test]
async fn test_search_wait_ctrl_c_exits_130_with_message() {
    let server = MockServer::start().await;

    // Create job (immediate)
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entry": [{ "content": { "sid": "test-sid" } }]
        })))
        .mount(&server)
        .await;

    // Job status: delay long enough so we can SIGINT while it's "running"
    let request_seen = Arc::new(Notify::new());
    let request_seen_clone = Arc::clone(&request_seen);

    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid"))
        .and(header("Authorization", "Bearer test-token"))
        .and(query_param("output_mode", "json"))
        .respond_with(move |_req: &wiremock::Request| {
            request_seen_clone.notify_one();
            ResponseTemplate::new(200)
                .set_delay(Duration::from_secs(60))
                .set_body_json(serde_json::json!({
                    "entry": [{
                        "content": {
                            "sid": "test-sid",
                            "isDone": false,
                            "isFinalized": false,
                            "doneProgress": 0.1,
                            "runDuration": 1.0,
                            "cursorTime": null,
                            "scanCount": 0,
                            "eventCount": 0,
                            "resultCount": 0,
                            "diskUsage": 0,
                            "priority": null,
                            "label": null
                        }
                    }]
                }))
        })
        .mount(&server)
        .await;

    let child = tokio::process::Command::new(splunk_cli_bin())
        .env("SPLUNK_BASE_URL", server.uri())
        .env("SPLUNK_API_TOKEN", "test-token")
        .args(["--output", "json", "search", TEST_QUERY, "--wait"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn splunk-cli");

    let pid = child.id().expect("child pid");
    tokio::time::timeout(Duration::from_secs(5), request_seen.notified())
        .await
        .expect("expected job status request before SIGINT");
    send_sigint(pid);

    let output = tokio::time::timeout(Duration::from_secs(5), child.wait_with_output())
        .await
        .expect("process should exit promptly")
        .expect("wait_with_output ok");

    assert_eq!(output.status.code(), Some(130));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Operation cancelled by user"));
}

#[tokio::test]
async fn test_logs_tail_ctrl_c_exits_130_with_message() {
    let server = MockServer::start().await;

    // `logs --tail` uses internal logs endpoint which internally creates a search job and fetches results.
    // Mock two endpoints so the command gets into its tail loop.
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entry": [{ "content": { "sid": "test-sid" } }]
        })))
        .mount(&server)
        .await;

    let request_seen = Arc::new(Notify::new());
    let request_seen_clone = Arc::clone(&request_seen);

    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .and(header("Authorization", "Bearer test-token"))
        .and(query_param("output_mode", "json"))
        .respond_with(move |_req: &wiremock::Request| {
            request_seen_clone.notify_one();
            ResponseTemplate::new(200)
                .set_delay(Duration::from_secs(60))
                .set_body_json(serde_json::json!({
                    "results": [{
                        "_time": "2025-01-20T10:30:00.000Z",
                        "_indextime": "2025-01-20T10:30:01.000Z",
                        "_serial": 1,
                        "log_level": "INFO",
                        "component": "test",
                        "_raw": "hello"
                    }],
                    "preview": false,
                    "total": 1
                }))
        })
        .mount(&server)
        .await;

    let child = tokio::process::Command::new(splunk_cli_bin())
        .env("SPLUNK_BASE_URL", server.uri())
        .env("SPLUNK_API_TOKEN", "test-token")
        .args([
            "--output",
            "json",
            "logs",
            "--tail",
            "--count",
            "1",
            "--earliest",
            "-1m",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn splunk-cli");

    let pid = child.id().expect("child pid");
    tokio::time::timeout(Duration::from_secs(5), request_seen.notified())
        .await
        .expect("expected tail request before SIGINT");
    send_sigint(pid);

    let output = tokio::time::timeout(Duration::from_secs(5), child.wait_with_output())
        .await
        .expect("process should exit promptly")
        .expect("wait_with_output ok");

    assert_eq!(output.status.code(), Some(130));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Operation cancelled by user"));
}
