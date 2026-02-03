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
use crate::runtime::side_effects::SharedClient;
use std::sync::Arc;

/// Handle loading forwarders with pagination support.
///
/// Emits `ForwardersLoaded` when offset == 0 (initial load/refresh).
/// Emits `MoreForwardersLoaded` when offset > 0 (pagination).
pub async fn handle_load_forwarders(
    client: SharedClient,
    tx: Sender<Action>,
    count: u64,
    offset: u64,
) {
    let result = {
        let mut guard = client.lock().await;
        guard.list_forwarders(Some(count), Some(offset)).await
    };

    let action = match result {
        Ok(forwarders) => {
            if offset == 0 {
                Action::ForwardersLoaded(Ok(forwarders))
            } else {
                Action::MoreForwardersLoaded(Ok(forwarders))
            }
        }
        Err(e) => {
            let arc_err = Arc::new(e);
            if offset == 0 {
                Action::ForwardersLoaded(Err(arc_err))
            } else {
                Action::MoreForwardersLoaded(Err(arc_err))
            }
        }
    };

    let _ = tx.send(action).await;
}
