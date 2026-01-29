//! User-related side effect handlers.
//!
//! Responsibilities:
//! - Handle async API calls for user operations.
//! - Fetch user lists from the Splunk server.
//!
//! Does NOT handle:
//! - Direct state modification (sends actions for that).
//! - UI rendering.

use crate::action::Action;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

use super::SharedClient;

/// Handle loading users.
pub async fn handle_load_users(client: SharedClient, tx: Sender<Action>, count: u64, offset: u64) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut c = client.lock().await;
        match c.list_users(Some(count), Some(offset)).await {
            Ok(users) => {
                let _ = tx.send(Action::UsersLoaded(Ok(users))).await;
            }
            Err(e) => {
                let _ = tx.send(Action::UsersLoaded(Err(Arc::new(e)))).await;
            }
        }
    });
}
