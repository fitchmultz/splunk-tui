//! License side effect handlers.
//!
//! Responsibilities:
//! - Handle async API calls for license operations.
//! - Collect license information from multiple endpoints (usage, pools, stacks).
//!
//! Does NOT handle:
//! - Direct state modification (sends actions for that).
//! - UI rendering.

use crate::action::{Action, LicenseData};
use splunk_client::ClientError;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

use super::SharedClient;

/// Handle loading license information from multiple endpoints.
pub async fn handle_load_license(client: SharedClient, tx: Sender<Action>) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut c = client.lock().await;

        // Collect license data from all three endpoints
        let mut license_data = LicenseData {
            usage: Vec::new(),
            pools: Vec::new(),
            stacks: Vec::new(),
        };

        let mut first_error: Option<ClientError> = None;

        // Fetch license usage
        match c.get_license_usage().await {
            Ok(usage) => license_data.usage = usage,
            Err(e) => {
                if first_error.is_none() {
                    first_error = Some(e);
                }
            }
        }

        // Fetch license pools
        match c.list_license_pools().await {
            Ok(pools) => license_data.pools = pools,
            Err(e) => {
                if first_error.is_none() {
                    first_error = Some(e);
                }
            }
        }

        // Fetch license stacks
        match c.list_license_stacks().await {
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
