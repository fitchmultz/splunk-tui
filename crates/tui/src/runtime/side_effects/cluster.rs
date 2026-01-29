//! Cluster-related side effect handlers.
//!
//! Responsibilities:
//! - Handle async API calls for cluster operations.
//! - Fetch cluster info and peer information.
//!
//! Does NOT handle:
//! - Direct state modification (sends actions for that).
//! - UI rendering.

use crate::action::Action;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

use super::SharedClient;

/// Handle loading cluster info.
pub async fn handle_load_cluster_info(client: SharedClient, tx: Sender<Action>) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut c = client.lock().await;
        match c.get_cluster_info().await {
            Ok(info) => {
                let _ = tx.send(Action::ClusterInfoLoaded(Ok(info))).await;
            }
            Err(e) => {
                let _ = tx.send(Action::ClusterInfoLoaded(Err(Arc::new(e)))).await;
            }
        }
    });
}

/// Handle loading cluster peers.
pub async fn handle_load_cluster_peers(client: SharedClient, tx: Sender<Action>) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut c = client.lock().await;
        match c.get_cluster_peers().await {
            Ok(peers) => {
                let _ = tx.send(Action::ClusterPeersLoaded(Ok(peers))).await;
            }
            Err(e) => {
                let _ = tx.send(Action::ClusterPeersLoaded(Err(Arc::new(e)))).await;
            }
        }
    });
}
