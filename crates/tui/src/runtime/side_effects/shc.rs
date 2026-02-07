//! SHC-related side effect handlers.
//!
//! Responsibilities:
//! - Handle async API calls for SHC operations.
//! - Fetch SHC status, members, captain, and config.
//! - Handle SHC management operations (rolling restart, set captain, add/remove members).
//!
//! Does NOT handle:
//! - Direct state modification (sends actions for that).
//! - UI rendering.

use crate::action::Action;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

use super::SharedClient;

/// Handle loading SHC status.
pub async fn handle_load_shc_status(client: SharedClient, tx: Sender<Action>) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        match client.get_shc_status().await {
            Ok(status) => {
                let _ = tx.send(Action::ShcStatusLoaded(Ok(status))).await;
            }
            Err(e) => {
                let _ = tx.send(Action::ShcStatusLoaded(Err(Arc::new(e)))).await;
            }
        }
    });
}

/// Handle loading SHC members.
pub async fn handle_load_shc_members(client: SharedClient, tx: Sender<Action>) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        match client.get_shc_members().await {
            Ok(members) => {
                let _ = tx.send(Action::ShcMembersLoaded(Ok(members))).await;
            }
            Err(e) => {
                let _ = tx.send(Action::ShcMembersLoaded(Err(Arc::new(e)))).await;
            }
        }
    });
}

/// Handle loading SHC captain.
pub async fn handle_load_shc_captain(client: SharedClient, tx: Sender<Action>) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        match client.get_shc_captain().await {
            Ok(captain) => {
                let _ = tx.send(Action::ShcCaptainLoaded(Ok(captain))).await;
            }
            Err(e) => {
                let _ = tx.send(Action::ShcCaptainLoaded(Err(Arc::new(e)))).await;
            }
        }
    });
}

/// Handle loading SHC config.
pub async fn handle_load_shc_config(client: SharedClient, tx: Sender<Action>) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        match client.get_shc_config().await {
            Ok(config) => {
                let _ = tx.send(Action::ShcConfigLoaded(Ok(config))).await;
            }
            Err(e) => {
                let _ = tx.send(Action::ShcConfigLoaded(Err(Arc::new(e)))).await;
            }
        }
    });
}

/// Handle adding an SHC member.
pub async fn handle_add_shc_member(client: SharedClient, tx: Sender<Action>, target_uri: String) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        match client.add_shc_member(&target_uri).await {
            Ok(_) => {
                let _ = tx.send(Action::ShcMemberAdded { result: Ok(()) }).await;
            }
            Err(e) => {
                let _ = tx
                    .send(Action::ShcMemberAdded {
                        result: Err(e.to_string()),
                    })
                    .await;
            }
        }
    });
}

/// Handle removing an SHC member.
pub async fn handle_remove_shc_member(
    client: SharedClient,
    tx: Sender<Action>,
    member_guid: String,
) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        match client.remove_shc_member(&member_guid).await {
            Ok(_) => {
                let _ = tx.send(Action::ShcMemberRemoved { result: Ok(()) }).await;
            }
            Err(e) => {
                let _ = tx
                    .send(Action::ShcMemberRemoved {
                        result: Err(e.to_string()),
                    })
                    .await;
            }
        }
    });
}

/// Handle triggering a rolling restart.
pub async fn handle_rolling_restart_shc(client: SharedClient, tx: Sender<Action>, force: bool) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        match client.rolling_restart_shc(force).await {
            Ok(_) => {
                let _ = tx
                    .send(Action::ShcRollingRestarted { result: Ok(()) })
                    .await;
            }
            Err(e) => {
                let _ = tx
                    .send(Action::ShcRollingRestarted {
                        result: Err(e.to_string()),
                    })
                    .await;
            }
        }
    });
}

/// Handle setting an SHC captain.
pub async fn handle_set_shc_captain(client: SharedClient, tx: Sender<Action>, member_guid: String) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        match client.set_shc_captain(&member_guid).await {
            Ok(_) => {
                let _ = tx.send(Action::ShcCaptainSet { result: Ok(()) }).await;
            }
            Err(e) => {
                let _ = tx
                    .send(Action::ShcCaptainSet {
                        result: Err(e.to_string()),
                    })
                    .await;
            }
        }
    });
}
