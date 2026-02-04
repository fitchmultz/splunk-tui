//! Tests for search-related action redaction.

use splunk_client::SearchMode;
use splunk_config::SearchDefaults;

use crate::action::tests::redacted_debug;
use crate::action::variants::Action;

#[test]
fn test_redact_run_search() {
    let action = Action::RunSearch {
        query: "SELECT * FROM users WHERE password='secret'".to_string(),
        search_defaults: SearchDefaults::default(),
        search_mode: SearchMode::Normal,
        realtime_window: None,
    };
    let output = redacted_debug(&action);

    assert!(output.contains("RunSearch"), "Should contain action name");
    assert!(
        output.contains("SELECT * FROM users"),
        "Should contain query content"
    );
    assert!(
        output.contains("password='secret'"),
        "Should contain full query including password string"
    );
}

#[test]
fn test_redact_search_started() {
    let action = Action::SearchStarted("SELECT * FROM users WHERE password='secret'".to_string());
    let output = redacted_debug(&action);

    assert!(
        output.contains("SearchStarted"),
        "Should contain action name"
    );
    assert!(
        output.contains("SELECT * FROM users"),
        "Should contain query content"
    );
    assert!(
        output.contains("password='secret'"),
        "Should contain full query"
    );
}

#[test]
fn test_redact_search_complete_ok() {
    let results = vec![
        serde_json::json!({"password": "secret123", "user": "admin"}),
        serde_json::json!({"token": "abc456", "user": "bob"}),
    ];
    let action = Action::SearchComplete(Ok((results, "search_job_12345".to_string(), Some(100))));
    let output = redacted_debug(&action);

    assert!(
        !output.contains("secret123"),
        "Should not contain sensitive data from results"
    );
    assert!(!output.contains("abc456"), "Should not contain token");
    assert!(
        !output.contains("admin"),
        "Should not contain user names from results"
    );
    assert!(
        output.contains("SearchComplete"),
        "Should contain action name"
    );
    assert!(output.contains("2 results"), "Should show result count");
    assert!(output.contains("sid=search_job_12345"), "Should show SID");
    assert!(
        output.contains("total=Some(100)"),
        "Should show total count"
    );
}

#[test]
fn test_redact_search_complete_err() {
    let action = Action::SearchComplete(Err((
        "Authentication failed for user admin".to_string(),
        crate::error_details::ErrorDetails::from_error_string("auth failed"),
    )));
    let output = redacted_debug(&action);

    assert!(
        !output.contains("Authentication failed"),
        "Should not contain error message"
    );
    assert!(!output.contains("admin"), "Should not contain user name");
    assert!(
        output.contains("SearchComplete"),
        "Should contain action name"
    );
    assert!(output.contains("<error>"), "Should show error indicator");
}

#[test]
fn test_redact_more_search_results_loaded_ok() {
    let results = vec![serde_json::json!({"password": "secret789", "data": "sensitive"})];
    let action = Action::MoreSearchResultsLoaded(Ok((results, 50, Some(200))));
    let output = redacted_debug(&action);

    assert!(
        !output.contains("secret789"),
        "Should not contain sensitive data"
    );
    assert!(
        !output.contains("sensitive"),
        "Should not contain data content"
    );
    assert!(
        output.contains("MoreSearchResultsLoaded"),
        "Should contain action name"
    );
    assert!(output.contains("1 results"), "Should show result count");
    assert!(output.contains("offset=50"), "Should show offset");
    assert!(output.contains("total=Some(200)"), "Should show total");
}

#[test]
fn test_redact_more_search_results_loaded_err() {
    use std::sync::Arc;

    let error = splunk_client::ClientError::ConnectionRefused("test".to_string());
    let action = Action::MoreSearchResultsLoaded(Err(Arc::new(error)));
    let output = redacted_debug(&action);

    assert!(
        !output.contains("Failed to fetch"),
        "Should not contain error message"
    );
    assert!(!output.contains("bob"), "Should not contain user name");
    assert!(
        output.contains("MoreSearchResultsLoaded"),
        "Should contain action name"
    );
    assert!(output.contains("<error>"), "Should show error indicator");
}
