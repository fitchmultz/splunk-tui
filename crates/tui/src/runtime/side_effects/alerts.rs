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

/// Handle loading fired alerts with pagination support.
///
/// Emits `FiredAlertsLoaded` when offset == 0 (initial load/refresh).
/// Emits `MoreFiredAlertsLoaded` when offset > 0 (pagination).
pub async fn handle_load_fired_alerts(
    client: SharedClient,
    tx: Sender<Action>,
    count: u64,
    offset: u64,
) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut c = client.lock().await;
        match c.list_fired_alerts(Some(count), Some(offset)).await {
            Ok(alerts) => {
                if offset == 0 {
                    let _ = tx.send(Action::FiredAlertsLoaded(Ok(alerts))).await;
                } else {
                    let _ = tx.send(Action::MoreFiredAlertsLoaded(Ok(alerts))).await;
                }
            }
            Err(e) => {
                if offset == 0 {
                    let _ = tx.send(Action::FiredAlertsLoaded(Err(Arc::new(e)))).await;
                } else {
                    let _ = tx
                        .send(Action::MoreFiredAlertsLoaded(Err(Arc::new(e))))
                        .await;
                }
            }
        }
    });
}
