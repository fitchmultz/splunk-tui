//! Cluster-related side effect handlers.
//!
//! Responsibilities:
//! - Handle async API calls for cluster operations.
//! - Fetch cluster info and peer information.
//! - Handle cluster management operations (maintenance mode, rebalance, decommission, remove).
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

/// Handle setting maintenance mode.
pub async fn handle_set_maintenance_mode(client: SharedClient, tx: Sender<Action>, enable: bool) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut c = client.lock().await;
        match c.set_maintenance_mode(enable).await {
            Ok(_) => {
                let _ = tx.send(Action::MaintenanceModeSet { result: Ok(()) }).await;
            }
            Err(e) => {
                let _ = tx
                    .send(Action::MaintenanceModeSet {
                        result: Err(e.to_string()),
                    })
                    .await;
            }
        }
    });
}

/// Handle rebalancing the cluster.
pub async fn handle_rebalance_cluster(client: SharedClient, tx: Sender<Action>) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut c = client.lock().await;
        match c.rebalance_cluster().await {
            Ok(_) => {
                let _ = tx.send(Action::ClusterRebalanced { result: Ok(()) }).await;
            }
            Err(e) => {
                let _ = tx
                    .send(Action::ClusterRebalanced {
                        result: Err(e.to_string()),
                    })
                    .await;
            }
        }
    });
}

/// Handle decommissioning a peer.
pub async fn handle_decommission_peer(client: SharedClient, tx: Sender<Action>, peer_guid: String) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut c = client.lock().await;
        match c.decommission_peer(&peer_guid).await {
            Ok(_) => {
                let _ = tx.send(Action::PeerDecommissioned { result: Ok(()) }).await;
            }
            Err(e) => {
                let _ = tx
                    .send(Action::PeerDecommissioned {
                        result: Err(e.to_string()),
                    })
                    .await;
            }
        }
    });
}

/// Handle removing a peer from the cluster.
pub async fn handle_remove_peer(client: SharedClient, tx: Sender<Action>, peer_guid: String) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut c = client.lock().await;
        let peer_guids = vec![peer_guid];
        match c.remove_peers(&peer_guids).await {
            Ok(_) => {
                let _ = tx.send(Action::PeerRemoved { result: Ok(()) }).await;
            }
            Err(e) => {
                let _ = tx
                    .send(Action::PeerRemoved {
                        result: Err(e.to_string()),
                    })
                    .await;
            }
        }
    });
}
