//! App-related side effect handlers.
//!
//! Responsibilities:
//! - Handle async API calls for app operations.
//! - Fetch app lists, enable apps, disable apps, install apps, remove apps.
//!
//! Does NOT handle:
//! - Direct state modification (sends actions for that).
//! - UI rendering.

use crate::action::Action;
use crate::ui::ToastLevel;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

use super::SharedClient;
use super::TaskTracker;

/// Handle loading apps with pagination support.
///
/// Emits `AppsLoaded` when offset == 0 (initial load/refresh).
/// Emits `MoreAppsLoaded` when offset > 0 (pagination).
pub async fn handle_load_apps(
    client: SharedClient,
    tx: Sender<Action>,
    task_tracker: TaskTracker,
    count: usize,
    offset: usize,
) {
    let _ = tx.send(Action::Loading(true)).await;
    task_tracker.spawn(async move {
        match client.list_apps(Some(count), Some(offset)).await {
            Ok(apps) => {
                if offset == 0 {
                    let _ = tx.send(Action::AppsLoaded(Ok(apps))).await;
                } else {
                    let _ = tx.send(Action::MoreAppsLoaded(Ok(apps))).await;
                }
            }
            Err(e) => {
                if offset == 0 {
                    let _ = tx.send(Action::AppsLoaded(Err(Arc::new(e)))).await;
                } else {
                    let _ = tx.send(Action::MoreAppsLoaded(Err(Arc::new(e)))).await;
                }
            }
        }
    });
}

/// Handle enabling an app.
pub async fn handle_enable_app(
    client: SharedClient,
    tx: Sender<Action>,
    task_tracker: TaskTracker,
    name: String,
) {
    let _ = tx.send(Action::Loading(true)).await;
    task_tracker.spawn(async move {
        match client.enable_app(&name).await {
            Ok(_) => {
                let _ = tx
                    .send(Action::Notify(
                        ToastLevel::Success,
                        format!("App '{}' enabled successfully", name),
                    ))
                    .await;
                // Refresh apps list (reset pagination)
                let _ = tx
                    .send(Action::LoadApps {
                        count: 100,
                        offset: 0,
                    })
                    .await;
            }
            Err(e) => {
                let _ = tx
                    .send(Action::Notify(
                        ToastLevel::Error,
                        format!("Failed to enable app '{}': {}", name, e),
                    ))
                    .await;
                let _ = tx.send(Action::Loading(false)).await;
            }
        }
    });
}

/// Handle disabling an app.
pub async fn handle_disable_app(
    client: SharedClient,
    tx: Sender<Action>,
    task_tracker: TaskTracker,
    name: String,
) {
    let _ = tx.send(Action::Loading(true)).await;
    task_tracker.spawn(async move {
        match client.disable_app(&name).await {
            Ok(_) => {
                let _ = tx
                    .send(Action::Notify(
                        ToastLevel::Success,
                        format!("App '{}' disabled successfully", name),
                    ))
                    .await;
                // Refresh apps list (reset pagination)
                let _ = tx
                    .send(Action::LoadApps {
                        count: 100,
                        offset: 0,
                    })
                    .await;
            }
            Err(e) => {
                let _ = tx
                    .send(Action::Notify(
                        ToastLevel::Error,
                        format!("Failed to disable app '{}': {}", name, e),
                    ))
                    .await;
                let _ = tx.send(Action::Loading(false)).await;
            }
        }
    });
}

/// Handle installing an app.
pub async fn handle_install_app(
    client: SharedClient,
    tx: Sender<Action>,
    task_tracker: TaskTracker,
    file_path: PathBuf,
) {
    let _ = tx.send(Action::Loading(true)).await;
    task_tracker.spawn(async move {
        match client.install_app(&file_path).await {
            Ok(app) => {
                let _ = tx
                    .send(Action::Notify(
                        ToastLevel::Success,
                        format!("App '{}' installed successfully", app.name),
                    ))
                    .await;
                // Refresh apps list (reset pagination)
                let _ = tx
                    .send(Action::LoadApps {
                        count: 100,
                        offset: 0,
                    })
                    .await;
            }
            Err(e) => {
                let _ = tx
                    .send(Action::Notify(
                        ToastLevel::Error,
                        format!("Failed to install app: {}", e),
                    ))
                    .await;
                let _ = tx.send(Action::Loading(false)).await;
            }
        }
    });
}

/// Handle removing an app.
pub async fn handle_remove_app(
    client: SharedClient,
    tx: Sender<Action>,
    task_tracker: TaskTracker,
    name: String,
) {
    let _ = tx.send(Action::Loading(true)).await;
    task_tracker.spawn(async move {
        match client.remove_app(&name).await {
            Ok(_) => {
                let _ = tx
                    .send(Action::Notify(
                        ToastLevel::Success,
                        format!("App '{}' removed successfully", name),
                    ))
                    .await;
                // Refresh apps list (reset pagination)
                let _ = tx
                    .send(Action::LoadApps {
                        count: 100,
                        offset: 0,
                    })
                    .await;
            }
            Err(e) => {
                let _ = tx
                    .send(Action::Notify(
                        ToastLevel::Error,
                        format!("Failed to remove app '{}': {}", name, e),
                    ))
                    .await;
                let _ = tx.send(Action::Loading(false)).await;
            }
        }
    });
}
