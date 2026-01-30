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

/// Handle loading search peers.
///
/// Fetches the list of distributed search peers from the Splunk server.
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
        Ok(peers) => Action::SearchPeersLoaded(Ok(peers)),
        Err(e) => Action::SearchPeersLoaded(Err(std::sync::Arc::new(e))),
    };

    let _ = tx.send(action).await;
}

/// Handle loading more search peers (pagination).
///
/// Fetches the next page of search peers from the Splunk server.
#[allow(dead_code)]
pub async fn handle_load_more_search_peers(
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
        Ok(peers) => Action::MoreSearchPeersLoaded(Ok(peers)),
        Err(e) => Action::MoreSearchPeersLoaded(Err(std::sync::Arc::new(e))),
    };

    let _ = tx.send(action).await;
}
