//! Role-related side effect handlers.
//!
//! Responsibilities:
//! - Handle async API calls for role operations.
//! - Fetch role lists from the Splunk server.
//! - Create, modify, and delete roles.
//!
//! Does NOT handle:
//! - Direct state modification (sends actions for that).
//! - UI rendering.

use crate::action::Action;
use crate::ui::ToastLevel;
use splunk_client::{CreateRoleParams, ModifyRoleParams};
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

use super::{SharedClient, TaskTracker};

/// Handle loading roles.
pub async fn handle_load_roles(
    client: SharedClient,
    tx: Sender<Action>,
    task_tracker: TaskTracker,
    count: usize,
    offset: usize,
) {
    let _ = tx.send(Action::Loading(true)).await;
    task_tracker.spawn(async move {
        match client.list_roles(Some(count), Some(offset)).await {
            Ok(roles) => {
                let _ = tx.send(Action::RolesLoaded(Ok(roles))).await;
            }
            Err(e) => {
                let _ = tx.send(Action::RolesLoaded(Err(Arc::new(e)))).await;
            }
        }
    });
}

/// Handle loading capabilities.
pub async fn handle_load_capabilities(
    client: SharedClient,
    tx: Sender<Action>,
    task_tracker: TaskTracker,
) {
    let _ = tx.send(Action::Loading(true)).await;
    task_tracker.spawn(async move {
        match client.list_capabilities().await {
            Ok(capabilities) => {
                let _ = tx.send(Action::CapabilitiesLoaded(Ok(capabilities))).await;
            }
            Err(e) => {
                let _ = tx.send(Action::CapabilitiesLoaded(Err(Arc::new(e)))).await;
            }
        }
    });
}

/// Handle creating a new role.
pub async fn handle_create_role(
    client: SharedClient,
    tx: Sender<Action>,
    task_tracker: TaskTracker,
    params: CreateRoleParams,
) {
    let _ = tx.send(Action::Loading(true)).await;
    task_tracker.spawn(async move {
        match client.create_role(&params).await {
            Ok(role) => {
                let _ = tx
                    .send(Action::Notify(
                        ToastLevel::Success,
                        format!("Role '{}' created successfully", role.name),
                    ))
                    .await;
                let _ = tx.send(Action::RoleCreated(Ok(role))).await;
                // Refresh roles list
                let _ = tx
                    .send(Action::LoadRoles {
                        count: 100,
                        offset: 0,
                    })
                    .await;
            }
            Err(e) => {
                let _ = tx
                    .send(Action::Notify(
                        ToastLevel::Error,
                        format!("Failed to create role '{}': {}", params.name, e),
                    ))
                    .await;
                let _ = tx.send(Action::RoleCreated(Err(Arc::new(e)))).await;
                let _ = tx.send(Action::Loading(false)).await;
            }
        }
    });
}

/// Handle modifying an existing role.
pub async fn handle_modify_role(
    client: SharedClient,
    tx: Sender<Action>,
    task_tracker: TaskTracker,
    name: String,
    params: ModifyRoleParams,
) {
    let _ = tx.send(Action::Loading(true)).await;
    task_tracker.spawn(async move {
        match client.modify_role(&name, &params).await {
            Ok(role) => {
                let _ = tx
                    .send(Action::Notify(
                        ToastLevel::Success,
                        format!("Role '{}' modified successfully", role.name),
                    ))
                    .await;
                let _ = tx.send(Action::RoleModified(Ok(role))).await;
                // Refresh roles list
                let _ = tx
                    .send(Action::LoadRoles {
                        count: 100,
                        offset: 0,
                    })
                    .await;
            }
            Err(e) => {
                let _ = tx
                    .send(Action::Notify(
                        ToastLevel::Error,
                        format!("Failed to modify role '{}': {}", name, e),
                    ))
                    .await;
                let _ = tx.send(Action::RoleModified(Err(Arc::new(e)))).await;
                let _ = tx.send(Action::Loading(false)).await;
            }
        }
    });
}

/// Handle deleting a role.
pub async fn handle_delete_role(
    client: SharedClient,
    tx: Sender<Action>,
    task_tracker: TaskTracker,
    name: String,
) {
    let _ = tx.send(Action::Loading(true)).await;
    task_tracker.spawn(async move {
        match client.delete_role(&name).await {
            Ok(()) => {
                let _ = tx
                    .send(Action::Notify(
                        ToastLevel::Success,
                        format!("Role '{}' deleted successfully", name),
                    ))
                    .await;
                let _ = tx.send(Action::RoleDeleted(Ok(name))).await;
                // Refresh roles list
                let _ = tx
                    .send(Action::LoadRoles {
                        count: 100,
                        offset: 0,
                    })
                    .await;
            }
            Err(e) => {
                let _ = tx
                    .send(Action::Notify(
                        ToastLevel::Error,
                        format!("Failed to delete role '{}': {}", name, e),
                    ))
                    .await;
                let _ = tx.send(Action::RoleDeleted(Err(Arc::new(e)))).await;
                let _ = tx.send(Action::Loading(false)).await;
            }
        }
    });
}
