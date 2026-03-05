//! Internal logs side effect handlers.
//!
//! Responsibilities:
//! - Handle async API calls for internal log fetching.
//!
//! Does NOT handle:
//! - Direct state modification (sends actions for that).
//! - UI rendering.

use crate::action::Action;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

use super::{SharedClient, TaskTracker};

/// Handle loading internal logs.
pub async fn handle_load_internal_logs(
    client: SharedClient,
    tx: Sender<Action>,
    task_tracker: TaskTracker,
    count: usize,
    earliest: String,
) {
    let _ = tx.send(Action::Loading(true)).await;
    task_tracker.spawn(async move {
        match client.get_internal_logs(count, Some(&earliest)).await {
            Ok(logs) => {
                let _ = tx.send(Action::InternalLogsLoaded(Ok(logs))).await;
            }
            Err(e) => {
                let _ = tx.send(Action::InternalLogsLoaded(Err(Arc::new(e)))).await;
            }
        }
    });
}
