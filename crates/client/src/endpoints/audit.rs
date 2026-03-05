//! Purpose: Audit event management endpoints.
//! Responsibilities: Fetch audit-like event data using Splunk search endpoints with optional filters.
//! Non-scope: Splunk audit configuration management.
//! Invariants/Assumptions: Query execution uses search jobs and tolerates partial parse failures.

use reqwest::Client;
use tracing::{debug, warn};

use crate::client::circuit_breaker::CircuitBreaker;
use crate::endpoints::search::{CreateJobOptions, OutputMode, create_job, get_results};
use crate::error::Result;
use crate::metrics::MetricsCollector;
use crate::models::audit::{AuditEvent, ListAuditEventsParams};

/// List audit events with optional filters.
#[allow(clippy::too_many_arguments)]
pub async fn list_audit_events(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    params: &ListAuditEventsParams,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<Vec<AuditEvent>> {
    debug!("Fetching audit events via search API");
    let mut query = String::from("search index=_audit");

    if let Some(user) = params.user.as_deref() {
        query.push(' ');
        query.push_str("user=\"");
        query.push_str(&escape_spl_string(user));
        query.push('"');
    }
    if let Some(action) = params.action.as_deref() {
        query.push(' ');
        query.push_str("action=\"");
        query.push_str(&escape_spl_string(action));
        query.push('"');
    }

    query.push_str(" | sort -_time, -_indextime, -_serial");
    if let Some(count) = params.count {
        query.push_str(&format!(" | head {}", count));
    }

    let options = CreateJobOptions {
        earliest_time: params.earliest.clone(),
        latest_time: params.latest.clone(),
        output_mode: Some(OutputMode::Json),
        exec_time: Some(30),
        wait: Some(true),
        ..Default::default()
    };

    let sid = create_job(
        client,
        base_url,
        auth_token,
        &query,
        &options,
        max_retries,
        metrics,
        circuit_breaker,
    )
    .await?;

    let results = get_results(
        client,
        base_url,
        auth_token,
        &sid,
        params.count,
        params.offset,
        OutputMode::Json,
        max_retries,
        metrics,
        circuit_breaker,
    )
    .await?;

    let endpoint = "/services/search/jobs/{sid}/results";
    let mut parse_failures = 0usize;
    let events: Vec<AuditEvent> = results
        .results
        .into_iter()
        .filter_map(|v| match serde_json::from_value::<AuditEvent>(v.clone()) {
            Ok(event) => Some(event),
            Err(e) => {
                parse_failures += 1;
                warn!(
                    "Failed to deserialize AuditEvent from {}: error={}, value_preview={}",
                    endpoint,
                    e,
                    serde_json::to_string(&v).unwrap_or_else(|_| format!("{:?}", v))
                );
                if let Some(m) = metrics {
                    m.record_deserialization_failure(endpoint, "AuditEvent");
                }
                None
            }
        })
        .collect();

    if parse_failures > 0 {
        debug!(
            "Completed list_audit_events with {} events and {} parse failures",
            events.len(),
            parse_failures
        );
    }

    Ok(events)
}

/// Get recent audit events (convenience wrapper).
#[allow(clippy::too_many_arguments)]
pub async fn get_recent_audit_events(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    count: usize,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<Vec<AuditEvent>> {
    let params = ListAuditEventsParams {
        earliest: Some("-24h".to_string()),
        latest: Some("now".to_string()),
        count: Some(count),
        offset: None,
        user: None,
        action: None,
    };
    list_audit_events(
        client,
        base_url,
        auth_token,
        &params,
        max_retries,
        metrics,
        circuit_breaker,
    )
    .await
}

fn escape_spl_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
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

    #[test]
    fn test_escape_spl_string_escapes_backslash_and_quotes() {
        let input = r#"user"with\chars"#;
        let escaped = escape_spl_string(input);
        assert_eq!(escaped, r#"user\"with\\chars"#);
    }
}
