//! License side effect handlers.
//!
//! Responsibilities:
//! - Handle async API calls for license operations.
//! - Collect license information from multiple endpoints (usage, pools, stacks).
//! - Handle license installation, pool management, and activation.
//!
//! Does NOT handle:
//! - Direct state modification (sends actions for that).
//! - UI rendering.

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::mpsc::Sender;

use crate::action::{Action, LicenseData};
use splunk_client::{ClientError, CreatePoolParams, ModifyPoolParams};

use super::SharedClient;

/// Handle loading license information from multiple endpoints.
pub async fn handle_load_license(client: SharedClient, tx: Sender<Action>) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        // Collect license data from all three endpoints
        let mut license_data = LicenseData {
            usage: Vec::new(),
            pools: Vec::new(),
            stacks: Vec::new(),
        };

        let mut first_error: Option<ClientError> = None;

        // Fetch license usage
        match client.get_license_usage().await {
            Ok(usage) => license_data.usage = usage,
            Err(e) => {
                if first_error.is_none() {
                    first_error = Some(e);
                }
            }
        }

        // Fetch license pools
        match client.list_license_pools().await {
            Ok(pools) => license_data.pools = pools,
            Err(e) => {
                if first_error.is_none() {
                    first_error = Some(e);
                }
            }
        }

        // Fetch license stacks
        match client.list_license_stacks().await {
            Ok(stacks) => license_data.stacks = stacks,
            Err(e) => {
                if first_error.is_none() {
                    first_error = Some(e);
                }
            }
        }

        if let Some(e) = first_error {
            let _ = tx
                .send(Action::LicenseLoaded(Box::new(Err(Arc::new(e)))))
                .await;
        } else {
            let _ = tx
                .send(Action::LicenseLoaded(Box::new(Ok(license_data))))
                .await;
        }
    });
}

/// Handle installing a license file.
pub async fn handle_install_license(client: SharedClient, file_path: PathBuf, tx: Sender<Action>) {
    tokio::spawn(async move {
        let result = client.install_license(&file_path).await;
        let _ = tx
            .send(Action::LicenseInstalled(result.map_err(Arc::new)))
            .await;
    });
}

/// Handle creating a license pool.
pub async fn handle_create_license_pool(
    client: SharedClient,
    params: CreatePoolParams,
    tx: Sender<Action>,
) {
    tokio::spawn(async move {
        let result = client.create_license_pool(&params).await;
        let _ = tx
            .send(Action::LicensePoolCreated(result.map_err(Arc::new)))
            .await;
    });
}

/// Handle modifying a license pool.
pub async fn handle_modify_license_pool(
    client: SharedClient,
    name: String,
    params: ModifyPoolParams,
    tx: Sender<Action>,
) {
    tokio::spawn(async move {
        let result = client.modify_license_pool(&name, &params).await;
        let _ = tx
            .send(Action::LicensePoolModified(result.map_err(Arc::new)))
            .await;
    });
}

/// Handle deleting a license pool.
pub async fn handle_delete_license_pool(client: SharedClient, name: String, tx: Sender<Action>) {
    tokio::spawn(async move {
        let result = client.delete_license_pool(&name).await;
        let _ = tx
            .send(Action::LicensePoolDeleted(
                result.map(|_| name).map_err(Arc::new),
            ))
            .await;
    });
}

/// Handle activating a license.
pub async fn handle_activate_license(client: SharedClient, name: String, tx: Sender<Action>) {
    tokio::spawn(async move {
        let result = client.activate_license(&name).await;
        let _ = tx
            .send(Action::LicenseActivated(result.map_err(Arc::new)))
            .await;
    });
}

/// Handle deactivating a license.
pub async fn handle_deactivate_license(client: SharedClient, name: String, tx: Sender<Action>) {
    tokio::spawn(async move {
        let result = client.deactivate_license(&name).await;
        let _ = tx
            .send(Action::LicenseDeactivated(result.map_err(Arc::new)))
            .await;
    });
}
