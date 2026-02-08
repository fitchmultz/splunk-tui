//! Side effects for forwarders operations.
//!
//! Responsibilities:
//! - Handle LoadForwarders action to fetch deployment clients (forwarders)
//! - Handle LoadMoreForwarders action for pagination
//!
//! Does NOT handle:
//! - UI rendering (handled by screen module)
//! - Input handling (handled by input handlers)

use tokio::sync::mpsc::Sender;

use crate::action::Action;
use crate::runtime::side_effects::{SharedClient, TaskTracker};
use std::sync::Arc;

/// Handle loading forwarders with pagination support.
///
/// Emits `ForwardersLoaded` when offset == 0 (initial load/refresh).
/// Emits `MoreForwardersLoaded` when offset > 0 (pagination).
pub async fn handle_load_forwarders(
    client: SharedClient,
    tx: Sender<Action>,
    task_tracker: TaskTracker,
    count: usize,
    offset: usize,
) {
    let _ = tx.send(Action::Loading(true)).await;
    task_tracker.spawn(async move {
        match client.list_forwarders(Some(count), Some(offset)).await {
            Ok(forwarders) => {
                if offset == 0 {
                    let _ = tx.send(Action::ForwardersLoaded(Ok(forwarders))).await;
                } else {
                    let _ = tx.send(Action::MoreForwardersLoaded(Ok(forwarders))).await;
                }
            }
            Err(e) => {
                let arc_err = Arc::new(e);
                if offset == 0 {
                    let _ = tx.send(Action::ForwardersLoaded(Err(arc_err))).await;
                } else {
                    let _ = tx.send(Action::MoreForwardersLoaded(Err(arc_err))).await;
                }
            }
        }
    });
}
