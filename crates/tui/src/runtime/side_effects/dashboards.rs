//! Dashboard-related side effect handlers.
//!
//! Responsibilities:
//! - Handle async API calls for dashboard operations.
//! - Fetch dashboard lists from the Splunk server.
//!
//! Does NOT handle:
//! - Direct state modification (sends actions for that).
//! - UI rendering.

use crate::action::Action;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

use super::SharedClient;

/// Handle loading dashboards.
pub async fn handle_load_dashboards(
    client: SharedClient,
    tx: Sender<Action>,
    count: u64,
    offset: u64,
) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut c = client.lock().await;
        match c.list_dashboards(Some(count), Some(offset)).await {
            Ok(dashboards) => {
                let _ = tx.send(Action::DashboardsLoaded(Ok(dashboards))).await;
            }
            Err(e) => {
                let _ = tx.send(Action::DashboardsLoaded(Err(Arc::new(e)))).await;
            }
        }
    });
}

/// Handle loading more dashboards (pagination).
#[allow(dead_code)]
pub async fn handle_load_more_dashboards(
    client: SharedClient,
    tx: Sender<Action>,
    count: u64,
    offset: u64,
) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut c = client.lock().await;
        match c.list_dashboards(Some(count), Some(offset)).await {
            Ok(dashboards) => {
                let _ = tx.send(Action::MoreDashboardsLoaded(Ok(dashboards))).await;
            }
            Err(e) => {
                let _ = tx
                    .send(Action::MoreDashboardsLoaded(Err(Arc::new(e))))
                    .await;
            }
        }
    });
}
