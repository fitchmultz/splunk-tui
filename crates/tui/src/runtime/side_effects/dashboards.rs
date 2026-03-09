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
use tokio::sync::mpsc::Sender;

use super::paginated::build_paginated_action;
use super::{SharedClient, TaskTracker};

/// Handle loading dashboards with pagination support.
///
/// Emits `DashboardsLoaded` when offset == 0 (initial load/refresh).
/// Emits `MoreDashboardsLoaded` when offset > 0 (pagination).
pub async fn handle_load_dashboards(
    client: SharedClient,
    tx: Sender<Action>,
    task_tracker: TaskTracker,
    count: usize,
    offset: usize,
) {
    let _ = tx.send(Action::Loading(true)).await;
    task_tracker.spawn(async move {
        let result = client.list_dashboards(Some(count), Some(offset)).await;
        let action = build_paginated_action(
            result,
            offset,
            Action::DashboardsLoaded,
            Action::MoreDashboardsLoaded,
        );
        let _ = tx.send(action).await;
    });
}
