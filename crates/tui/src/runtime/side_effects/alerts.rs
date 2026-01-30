//! Alert-related side effect handlers.
//!
//! Responsibilities:
//! - Handle async API calls for alert operations.
//! - Load fired alerts and pagination results.
//!
//! Does NOT handle:
//! - Direct state modification (sends actions for that).
//! - UI rendering.

use crate::action::Action;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

use super::SharedClient;

/// Handle loading fired alerts.
pub async fn handle_load_fired_alerts(client: SharedClient, tx: Sender<Action>) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut c = client.lock().await;
        match c.list_fired_alerts(None, None).await {
            Ok(alerts) => {
                let _ = tx.send(Action::FiredAlertsLoaded(Ok(alerts))).await;
            }
            Err(e) => {
                let _ = tx.send(Action::FiredAlertsLoaded(Err(Arc::new(e)))).await;
            }
        }
    });
}

/// Handle loading more fired alerts (pagination).
#[allow(dead_code)]
pub async fn handle_load_more_fired_alerts(
    client: SharedClient,
    tx: Sender<Action>,
    count: u64,
    offset: u64,
) {
    tokio::spawn(async move {
        let mut c = client.lock().await;
        match c.list_fired_alerts(Some(count), Some(offset)).await {
            Ok(alerts) => {
                let _ = tx.send(Action::MoreFiredAlertsLoaded(Ok(alerts))).await;
            }
            Err(e) => {
                let _ = tx
                    .send(Action::MoreFiredAlertsLoaded(Err(Arc::new(e))))
                    .await;
            }
        }
    });
}
