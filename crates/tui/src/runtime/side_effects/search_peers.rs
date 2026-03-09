//! Side effects for search peers operations.
//!
//! Responsibilities:
//! - Handle LoadSearchPeers action to fetch distributed search peers
//! - Handle LoadMoreSearchPeers action for pagination
//!
//! Does NOT handle:
//! - UI rendering (handled by screen module)
//! - Input handling (handled by input handlers)

use tokio::sync::mpsc::Sender;

use crate::action::Action;
use crate::runtime::side_effects::{SharedClient, TaskTracker, paginated::build_paginated_action};

/// Handle loading search peers with pagination support.
///
/// Emits `SearchPeersLoaded` when offset == 0 (initial load/refresh).
/// Emits `MoreSearchPeersLoaded` when offset > 0 (pagination).
pub async fn handle_load_search_peers(
    client: SharedClient,
    tx: Sender<Action>,
    task_tracker: TaskTracker,
    count: usize,
    offset: usize,
) {
    let _ = tx.send(Action::Loading(true)).await;
    task_tracker.spawn(async move {
        let result = client.list_search_peers(Some(count), Some(offset)).await;
        let action = build_paginated_action(
            result,
            offset,
            Action::SearchPeersLoaded,
            Action::MoreSearchPeersLoaded,
        );
        let _ = tx.send(action).await;
    });
}
