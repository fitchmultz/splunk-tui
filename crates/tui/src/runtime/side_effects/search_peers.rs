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
use crate::runtime::side_effects::SharedClient;
use std::sync::Arc;

/// Handle loading search peers with pagination support.
///
/// Emits `SearchPeersLoaded` when offset == 0 (initial load/refresh).
/// Emits `MoreSearchPeersLoaded` when offset > 0 (pagination).
pub async fn handle_load_search_peers(
    client: SharedClient,
    tx: Sender<Action>,
    count: u64,
    offset: u64,
) {
    let result = {
        let mut guard = client.lock().await;
        guard.list_search_peers(Some(count), Some(offset)).await
    };

    let action = match result {
        Ok(peers) => {
            if offset == 0 {
                Action::SearchPeersLoaded(Ok(peers))
            } else {
                Action::MoreSearchPeersLoaded(Ok(peers))
            }
        }
        Err(e) => {
            let arc_err = Arc::new(e);
            if offset == 0 {
                Action::SearchPeersLoaded(Err(arc_err))
            } else {
                Action::MoreSearchPeersLoaded(Err(arc_err))
            }
        }
    };

    let _ = tx.send(action).await;
}
