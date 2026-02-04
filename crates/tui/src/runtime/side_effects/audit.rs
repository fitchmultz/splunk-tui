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
    count: u64,
    offset: u64,
    earliest: String,
    latest: String,
) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut c = client.lock().await;
        let params = ListAuditEventsParams {
            earliest: Some(earliest),
            latest: Some(latest),
            count: Some(count),
            offset: Some(offset),
            user: None,
            action: None,
        };
        match c.list_audit_events(&params).await {
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
pub async fn handle_load_recent_audit_events(client: SharedClient, tx: Sender<Action>, count: u64) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut c = client.lock().await;
        match c.get_recent_audit_events(count).await {
            Ok(events) => {
                let _ = tx.send(Action::AuditEventsLoaded(Ok(events))).await;
            }
            Err(e) => {
                let _ = tx.send(Action::AuditEventsLoaded(Err(Arc::new(e)))).await;
            }
        }
    });
}
