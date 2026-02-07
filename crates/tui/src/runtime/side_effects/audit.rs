//! Audit event-related side effect handlers.
//!
//! Responsibilities:
//! - Handle async API calls for audit event operations.
//! - Fetch audit events from the Splunk server.
//!
//! Does NOT handle:
//! - Direct state modification (sends actions for that).
//! - UI rendering.

use crate::action::Action;
use splunk_client::models::ListAuditEventsParams;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

use super::SharedClient;

/// Handle loading audit events.
pub async fn handle_load_audit_events(
    client: SharedClient,
    tx: Sender<Action>,
    count: usize,
    offset: usize,
    earliest: String,
    latest: String,
) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let params = ListAuditEventsParams {
            earliest: Some(earliest),
            latest: Some(latest),
            count: Some(count),
            offset: Some(offset),
            user: None,
            action: None,
        };
        match client.list_audit_events(&params).await {
            Ok(events) => {
                let _ = tx.send(Action::AuditEventsLoaded(Ok(events))).await;
            }
            Err(e) => {
                let _ = tx.send(Action::AuditEventsLoaded(Err(Arc::new(e)))).await;
            }
        }
    });
}

/// Handle loading recent audit events.
pub async fn handle_load_recent_audit_events(
    client: SharedClient,
    tx: Sender<Action>,
    count: usize,
) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        match client.get_recent_audit_events(count).await {
            Ok(events) => {
                let _ = tx.send(Action::AuditEventsLoaded(Ok(events))).await;
            }
            Err(e) => {
                let _ = tx.send(Action::AuditEventsLoaded(Err(Arc::new(e)))).await;
            }
        }
    });
}
