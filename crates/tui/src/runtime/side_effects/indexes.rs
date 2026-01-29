//! Index-related side effect handlers.
//!
//! Responsibilities:
//! - Handle async API calls for index operations.
//! - Fetch index lists from the Splunk server.
//!
//! Does NOT handle:
//! - Direct state modification (sends actions for that).
//! - UI rendering.

use crate::action::Action;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

use super::SharedClient;

/// Handle loading indexes.
pub async fn handle_load_indexes(
    client: SharedClient,
    tx: Sender<Action>,
    count: u64,
    offset: u64,
) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut c = client.lock().await;
        match c.list_indexes(Some(count), Some(offset)).await {
            Ok(indexes) => {
                let _ = tx.send(Action::IndexesLoaded(Ok(indexes))).await;
            }
            Err(e) => {
                let _ = tx.send(Action::IndexesLoaded(Err(Arc::new(e)))).await;
            }
        }
    });
}
