//! Purpose: Integration tests for audit-event retrieval behavior.
//! Responsibilities: Verify audit listing uses search-job endpoints and parses result rows into `AuditEvent`.
//! Non-scope: Does not validate UI rendering or CLI formatting.
//! Invariants/Assumptions: Mocked Splunk responses represent the wire protocol contracts used by the client.

mod common;

use common::*;
use wiremock::matchers::{method, path};

#[tokio::test]
async fn test_list_audit_events_uses_search_job_flow() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "sid": "audit-sid-123"
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/services/search/jobs/audit-sid-123/results"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [
                {
                    "_time": "2026-02-22T15:00:00.000+00:00",
                    "user": "admin",
                    "action": "login",
                    "result": "success",
                    "client_ip": "127.0.0.1",
                    "_raw": "audit event"
                }
            ],
            "preview": false,
            "offset": 0,
            "total": 1
        })))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let params = splunk_client::models::audit::ListAuditEventsParams {
        earliest: Some("-24h".to_string()),
        latest: Some("now".to_string()),
        count: Some(10),
        offset: None,
        user: None,
        action: None,
    };

    let result = endpoints::list_audit_events(
        &client,
        &mock_server.uri(),
        "test-token",
        &params,
        3,
        None,
        None,
    )
    .await;

    assert!(result.is_ok());
    let events = result.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].user, "admin");
    assert_eq!(events[0].time, "2026-02-22T15:00:00.000+00:00");
    assert_eq!(events[0].client_ip, "127.0.0.1");
}
