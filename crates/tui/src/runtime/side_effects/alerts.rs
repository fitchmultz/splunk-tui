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
use tokio::sync::mpsc::Sender;

use super::paginated::build_paginated_action;
use super::{SharedClient, TaskTracker};

/// Handle loading fired alerts with pagination support.
///
/// Emits `FiredAlertsLoaded` when offset == 0 (initial load/refresh).
/// Emits `MoreFiredAlertsLoaded` when offset > 0 (pagination).
pub async fn handle_load_fired_alerts(
    client: SharedClient,
    tx: Sender<Action>,
    task_tracker: TaskTracker,
    count: usize,
    offset: usize,
) {
    let _ = tx.send(Action::Loading(true)).await;
    task_tracker.spawn(async move {
        let result = client.list_fired_alerts(Some(count), Some(offset)).await;
        let action = build_paginated_action(
            result,
            offset,
            Action::FiredAlertsLoaded,
            Action::MoreFiredAlertsLoaded,
        );
        let _ = tx.send(action).await;
    });
}
