//! Multi-instance dashboard side effect handler.
//!
//! Purpose:
//! - Bridge the TUI action loop to the shared multi-profile workflow.
//!
//! Responsibilities:
//! - Load multi-instance overview data from shared client workflows.
//! - Emit incremental and aggregate TUI actions from the shared payload.
//!
//! Scope:
//! - TUI action dispatch only; aggregation lives in `splunk-client`.
//!
//! Usage:
//! - Called by the side-effect dispatcher for multi-instance refresh and retry actions.
//!
//! Invariants/Assumptions:
//! - Shared multi-profile workflow is the source of truth for dashboard aggregation.

use crate::action::Action;
use splunk_client::workflows::multi_profile::{
    fetch_instance_overview, fetch_multi_instance_overview,
};
use splunk_config::ConfigManager;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc::Sender;

use crate::ui::ToastLevel;

use super::TaskTracker;

/// Handle loading multi-instance overview from all configured profiles.
pub async fn handle_load_multi_instance_overview(
    config_manager: Arc<Mutex<ConfigManager>>,
    tx: Sender<Action>,
    task_tracker: TaskTracker,
) {
    let _ = tx.send(Action::Loading(true)).await;

    task_tracker.spawn(async move {
        let profiles = {
            let cm = config_manager.lock().await;
            cm.list_profiles()
                .iter()
                .map(|(name, profile)| (name.clone(), profile.clone()))
                .collect::<Vec<_>>()
        };

        match fetch_multi_instance_overview(profiles, None).await {
            Ok(data) => {
                for instance in data.instances.iter().cloned() {
                    let _ = tx.send(Action::MultiInstanceInstanceLoaded(instance)).await;
                }

                let _ = tx.send(Action::MultiInstanceOverviewLoaded(data)).await;
            }
            Err(error) => {
                let _ = tx
                    .send(Action::Notify(
                        ToastLevel::Error,
                        format!("Failed to refresh multi-instance overview: {error}"),
                    ))
                    .await;
            }
        }
    });
}

/// Handle retrying a specific instance.
pub async fn handle_retry_instance(
    profile_name: String,
    config_manager: Arc<Mutex<ConfigManager>>,
    tx: Sender<Action>,
    task_tracker: TaskTracker,
) {
    task_tracker.spawn(async move {
        let profile = {
            let cm = config_manager.lock().await;
            cm.list_profiles().get(&profile_name).cloned()
        };

        let Some(profile) = profile else {
            let _ = tx
                .send(Action::Notify(
                    ToastLevel::Error,
                    format!("Profile '{profile_name}' not found"),
                ))
                .await;
            return;
        };

        let mut last_error = None;
        for attempt in 0..3 {
            match fetch_instance_overview(profile_name.clone(), profile.clone(), None).await {
                Ok(instance) if instance.error.is_none() || attempt == 2 => {
                    let _ = tx.send(Action::MultiInstanceInstanceLoaded(instance)).await;
                    return;
                }
                Ok(instance) => {
                    last_error = instance.error.clone();
                    tokio::time::sleep(std::time::Duration::from_millis(250 * (1 << attempt)))
                        .await;
                }
                Err(error) if attempt == 2 => {
                    let _ = tx
                        .send(Action::Notify(
                            ToastLevel::Error,
                            format!("Failed to refresh profile '{profile_name}': {error}"),
                        ))
                        .await;
                    return;
                }
                Err(_) => {
                    tokio::time::sleep(std::time::Duration::from_millis(250 * (1 << attempt)))
                        .await;
                }
            }
        }

        let _ = tx
            .send(Action::Notify(
                ToastLevel::Error,
                format!(
                    "Failed to refresh profile '{profile_name}': {}",
                    last_error.unwrap_or_else(|| "unknown error".to_string())
                ),
            ))
            .await;
    });
}
