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

use super::SharedClient;

/// Handle loading internal logs.
pub async fn handle_load_internal_logs(client: SharedClient, tx: Sender<Action>) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut c = client.lock().await;
        // Default to last 15 minutes of logs, 100 entries
        match c.get_internal_logs(100, Some("-15m")).await {
            Ok(logs) => {
                let _ = tx.send(Action::InternalLogsLoaded(Ok(logs))).await;
            }
            Err(e) => {
                let _ = tx.send(Action::InternalLogsLoaded(Err(Arc::new(e)))).await;
            }
        }
    });
}
