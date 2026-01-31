//! Users side effect handler tests.
//!
//! This module tests the LoadUsers side effect handler which fetches
//! user information from the Splunk REST API.

mod common;

use common::*;
use wiremock::matchers::{method, path};

#[tokio::test]
async fn test_load_users_success() {
    let mut harness = SideEffectsTestHarness::new().await;

    let fixture = serde_json::json!({
        "entry": [
            {
                "name": "admin",
                "content": {
                    "realname": "Administrator",
                    "email": "admin@example.com",
                    "roles": ["admin"]
                }
            }
        ]
    });

    Mock::given(method("GET"))
        .and(path("/services/authentication/users"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(
            Action::LoadUsers {
                count: 100,
                offset: 0,
            },
            2,
        )
        .await;

    assert!(
        actions.iter().any(|a| matches!(a, Action::Loading(true))),
        "Should send Loading(true)"
    );
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::UsersLoaded(Ok(_)))),
        "Should send UsersLoaded(Ok)"
    );
}
