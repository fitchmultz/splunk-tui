//! KVStore side effect handlers.
//!
//! Responsibilities:
//! - Handle async API calls for KVStore operations.
//!
//! Does NOT handle:
//! - Direct state modification (sends actions for that).
//! - UI rendering.

use crate::action::Action;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

use super::SharedClient;

/// Handle loading KVStore status.
pub async fn handle_load_kvstore(client: SharedClient, tx: Sender<Action>) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut c = client.lock().await;

        match c.get_kvstore_status().await {
            Ok(status) => {
                let _ = tx.send(Action::KvstoreLoaded(Ok(status))).await;
            }
            Err(e) => {
                let _ = tx.send(Action::KvstoreLoaded(Err(Arc::new(e)))).await;
            }
        }
    });
}
