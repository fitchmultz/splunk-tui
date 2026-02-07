//! Audit event management endpoints.

use reqwest::Client;

use crate::endpoints::send_request_with_retry;
use crate::error::Result;
use crate::metrics::MetricsCollector;
use crate::models::audit::{AuditEvent, AuditEventListResponse, ListAuditEventsParams};
use crate::name_merge::attach_entry_name;

/// List audit events with optional filters.
pub async fn list_audit_events(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    params: &ListAuditEventsParams,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<Vec<AuditEvent>> {
    let url = format!("{}/services/admin/audit", base_url);

    let mut query_params: Vec<(String, String)> =
        vec![("output_mode".to_string(), "json".to_string())];

    if let Some(count) = params.count {
        query_params.push(("count".to_string(), count.to_string()));
    }
    if let Some(offset) = params.offset {
        query_params.push(("offset".to_string(), offset.to_string()));
    }
    if let Some(earliest) = &params.earliest {
        query_params.push(("earliest".to_string(), earliest.clone()));
    }
    if let Some(latest) = &params.latest {
        query_params.push(("latest".to_string(), latest.clone()));
    }

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&query_params);

    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/admin/audit",
        "GET",
        metrics,
    )
    .await?;

    let resp: AuditEventListResponse = response.json().await?;

    Ok(resp
        .entry
        .into_iter()
        .map(|e| attach_entry_name(e.name, e.content))
        .collect())
}

/// Get recent audit events (convenience wrapper).
pub async fn get_recent_audit_events(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    count: usize,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<Vec<AuditEvent>> {
    let params = ListAuditEventsParams {
        earliest: Some("-24h".to_string()),
        latest: Some("now".to_string()),
        count: Some(count),
        offset: None,
        user: None,
        action: None,
    };
    list_audit_events(client, base_url, auth_token, &params, max_retries, metrics).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_audit_events_params() {
        let params = ListAuditEventsParams {
            earliest: Some("-24h".to_string()),
            latest: Some("now".to_string()),
            count: Some(50),
            offset: Some(0),
            user: Some("admin".to_string()),
            action: Some("login".to_string()),
        };

        assert_eq!(params.earliest, Some("-24h".to_string()));
        assert_eq!(params.latest, Some("now".to_string()));
        assert_eq!(params.count, Some(50));
        assert_eq!(params.offset, Some(0));
        assert_eq!(params.user, Some("admin".to_string()));
        assert_eq!(params.action, Some("login".to_string()));
    }
}
