//! User-related side effect handlers.
//!
//! Responsibilities:
//! - Handle async API calls for user operations.
//! - Fetch user lists from the Splunk server.
//! - Create, modify, and delete users.
//!
//! Does NOT handle:
//! - Direct state modification (sends actions for that).
//! - UI rendering.

use crate::action::Action;
use crate::ui::ToastLevel;
use splunk_client::{CreateUserParams, ModifyUserParams};
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

use super::{SharedClient, TaskTracker};

/// Handle loading users with pagination support.
///
/// Emits `UsersLoaded` when offset == 0 (initial load/refresh).
/// Emits `MoreUsersLoaded` when offset > 0 (pagination).
pub async fn handle_load_users(
    client: SharedClient,
    tx: Sender<Action>,
    task_tracker: TaskTracker,
    count: usize,
    offset: usize,
) {
    let _ = tx.send(Action::Loading(true)).await;
    task_tracker.spawn(async move {
        match client.list_users(Some(count), Some(offset)).await {
            Ok(users) => {
                if offset == 0 {
                    let _ = tx.send(Action::UsersLoaded(Ok(users))).await;
                } else {
                    let _ = tx.send(Action::MoreUsersLoaded(Ok(users))).await;
                }
            }
            Err(e) => {
                if offset == 0 {
                    let _ = tx.send(Action::UsersLoaded(Err(Arc::new(e)))).await;
                } else {
                    let _ = tx.send(Action::MoreUsersLoaded(Err(Arc::new(e)))).await;
                }
            }
        }
    });
}

/// Handle creating a new user.
pub async fn handle_create_user(
    client: SharedClient,
    tx: Sender<Action>,
    task_tracker: TaskTracker,
    params: CreateUserParams,
) {
    let _ = tx.send(Action::Loading(true)).await;
    task_tracker.spawn(async move {
        match client.create_user(&params).await {
            Ok(user) => {
                let _ = tx
                    .send(Action::Notify(
                        ToastLevel::Success,
                        format!("User '{}' created successfully", user.name),
                    ))
                    .await;
                let _ = tx.send(Action::UserCreated(Ok(user))).await;
                // Refresh users list
                let _ = tx
                    .send(Action::LoadUsers {
                        count: 100,
                        offset: 0,
                    })
                    .await;
            }
            Err(e) => {
                let _ = tx
                    .send(Action::Notify(
                        ToastLevel::Error,
                        format!("Failed to create user '{}': {}", params.name, e),
                    ))
                    .await;
                let _ = tx.send(Action::UserCreated(Err(Arc::new(e)))).await;
                let _ = tx.send(Action::Loading(false)).await;
            }
        }
    });
}

/// Handle modifying an existing user.
pub async fn handle_modify_user(
    client: SharedClient,
    tx: Sender<Action>,
    task_tracker: TaskTracker,
    name: String,
    params: ModifyUserParams,
) {
    let _ = tx.send(Action::Loading(true)).await;
    task_tracker.spawn(async move {
        match client.modify_user(&name, &params).await {
            Ok(user) => {
                let _ = tx
                    .send(Action::Notify(
                        ToastLevel::Success,
                        format!("User '{}' modified successfully", user.name),
                    ))
                    .await;
                let _ = tx.send(Action::UserModified(Ok(user))).await;
                // Refresh users list
                let _ = tx
                    .send(Action::LoadUsers {
                        count: 100,
                        offset: 0,
                    })
                    .await;
            }
            Err(e) => {
                let _ = tx
                    .send(Action::Notify(
                        ToastLevel::Error,
                        format!("Failed to modify user '{}': {}", name, e),
                    ))
                    .await;
                let _ = tx.send(Action::UserModified(Err(Arc::new(e)))).await;
                let _ = tx.send(Action::Loading(false)).await;
            }
        }
    });
}

/// Handle deleting a user.
pub async fn handle_delete_user(
    client: SharedClient,
    tx: Sender<Action>,
    task_tracker: TaskTracker,
    name: String,
) {
    let _ = tx.send(Action::Loading(true)).await;
    task_tracker.spawn(async move {
        match client.delete_user(&name).await {
            Ok(()) => {
                let _ = tx
                    .send(Action::Notify(
                        ToastLevel::Success,
                        format!("User '{}' deleted successfully", name),
                    ))
                    .await;
                let _ = tx.send(Action::UserDeleted(Ok(name))).await;
                // Refresh users list
                let _ = tx
                    .send(Action::LoadUsers {
                        count: 100,
                        offset: 0,
                    })
                    .await;
            }
            Err(e) => {
                let _ = tx
                    .send(Action::Notify(
                        ToastLevel::Error,
                        format!("Failed to delete user '{}': {}", name, e),
                    ))
                    .await;
                let _ = tx.send(Action::UserDeleted(Err(Arc::new(e)))).await;
                let _ = tx.send(Action::Loading(false)).await;
            }
        }
    });
}
