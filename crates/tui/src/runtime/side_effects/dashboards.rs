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

/// Handle loading dashboards with pagination support.
///
/// Emits `DashboardsLoaded` when offset == 0 (initial load/refresh).
/// Emits `MoreDashboardsLoaded` when offset > 0 (pagination).
pub async fn handle_load_dashboards(
    client: SharedClient,
    tx: Sender<Action>,
    count: usize,
    offset: usize,
) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        match client.list_dashboards(Some(count), Some(offset)).await {
            Ok(dashboards) => {
                if offset == 0 {
                    let _ = tx.send(Action::DashboardsLoaded(Ok(dashboards))).await;
                } else {
                    let _ = tx.send(Action::MoreDashboardsLoaded(Ok(dashboards))).await;
                }
            }
            Err(e) => {
                if offset == 0 {
                    let _ = tx.send(Action::DashboardsLoaded(Err(Arc::new(e)))).await;
                } else {
                    let _ = tx
                        .send(Action::MoreDashboardsLoaded(Err(Arc::new(e))))
                        .await;
                }
            }
        }
    });
}
