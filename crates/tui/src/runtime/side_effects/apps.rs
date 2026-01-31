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

/// Handle loading apps.
pub async fn handle_load_apps(client: SharedClient, tx: Sender<Action>, count: u64, offset: u64) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut c = client.lock().await;
        match c.list_apps(Some(count), Some(offset)).await {
            Ok(apps) => {
                let _ = tx.send(Action::AppsLoaded(Ok(apps))).await;
            }
            Err(e) => {
                let _ = tx.send(Action::AppsLoaded(Err(Arc::new(e)))).await;
            }
        }
    });
}

/// Handle enabling an app.
pub async fn handle_enable_app(client: SharedClient, tx: Sender<Action>, name: String) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut c = client.lock().await;
        match c.enable_app(&name).await {
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
pub async fn handle_disable_app(client: SharedClient, tx: Sender<Action>, name: String) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut c = client.lock().await;
        match c.disable_app(&name).await {
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
pub async fn handle_install_app(client: SharedClient, tx: Sender<Action>, file_path: PathBuf) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut c = client.lock().await;
        match c.install_app(&file_path).await {
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
pub async fn handle_remove_app(client: SharedClient, tx: Sender<Action>, name: String) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut c = client.lock().await;
        match c.remove_app(&name).await {
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
