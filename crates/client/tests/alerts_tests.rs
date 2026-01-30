//! Integration tests for alerts endpoints.

mod common;

use common::*;
use wiremock::matchers::{method, path, query_param};

#[tokio::test]
async fn test_list_fired_alerts() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("alerts/list_fired_alerts.json");

    Mock::given(method("GET"))
        .and(path("/services/alerts/fired_alerts"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::list_fired_alerts(
        &client,
        &mock_server.uri(),
        "test-token",
        None,
        None,
        3,
        None,
    )
    .await;

    assert!(result.is_ok());
    let alerts = result.unwrap();
    assert_eq!(alerts.len(), 3);
    assert_eq!(
        alerts[0].name,
        "scheduler__admin__search__MyAlert_at_1351181001_5.31_1351181987"
    );
    assert_eq!(alerts[0].savedsearch_name, Some("MyAlert".to_string()));
    assert_eq!(alerts[0].severity, Some("High".to_string()));
    assert_eq!(alerts[0].actions, Some("email".to_string()));
}

#[tokio::test]
async fn test_list_fired_alerts_with_pagination() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("alerts/list_fired_alerts.json");

    Mock::given(method("GET"))
        .and(path("/services/alerts/fired_alerts"))
        .and(query_param("output_mode", "json"))
        .and(query_param("count", "2"))
        .and(query_param("offset", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::list_fired_alerts(
        &client,
        &mock_server.uri(),
        "test-token",
        Some(2),
        Some(0),
        3,
        None,
    )
    .await;

    assert!(result.is_ok());
    let alerts = result.unwrap();
    assert!(!alerts.is_empty());
}

#[tokio::test]
async fn test_get_fired_alert() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("alerts/get_fired_alert.json");
    let alert_name = "scheduler__admin__search__MyAlert_at_1351181001_5.31_1351181987";

    Mock::given(method("GET"))
        .and(path(format!(
            "/services/alerts/fired_alerts/{}",
            alert_name
        )))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result: Result<splunk_client::models::FiredAlert, _> = endpoints::get_fired_alert(
        &client,
        &mock_server.uri(),
        "test-token",
        alert_name,
        3,
        None,
    )
    .await;

    assert!(result.is_ok());
    let alert = result.unwrap();
    assert_eq!(alert.name, alert_name);
    assert_eq!(alert.savedsearch_name, Some("MyAlert".to_string()));
    assert_eq!(alert.severity, Some("High".to_string()));
}

#[tokio::test]
async fn test_get_fired_alert_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/alerts/fired_alerts/NonExistentAlert"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result: Result<splunk_client::models::FiredAlert, _> = endpoints::get_fired_alert(
        &client,
        &mock_server.uri(),
        "test-token",
        "NonExistentAlert",
        3,
        None,
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_list_fired_alerts_empty_response() {
    let mock_server = MockServer::start().await;

    let fixture: serde_json::Value = serde_json::from_str(r#"{"entry": []}"#).unwrap();

    Mock::given(method("GET"))
        .and(path("/services/alerts/fired_alerts"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::list_fired_alerts(
        &client,
        &mock_server.uri(),
        "test-token",
        None,
        None,
        3,
        None,
    )
    .await;

    assert!(result.is_ok());
    let alerts = result.unwrap();
    assert!(alerts.is_empty());
}
