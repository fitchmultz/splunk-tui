//! Data model-related side effect handlers.
//!
//! Responsibilities:
//! - Handle async API calls for data model operations.
//! - Fetch data model lists from the Splunk server.
//!
//! Does NOT handle:
//! - Direct state modification (sends actions for that).
//! - UI rendering.

use crate::action::Action;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

use super::SharedClient;

/// Handle loading data models.
pub async fn handle_load_datamodels(
    client: SharedClient,
    tx: Sender<Action>,
    count: u64,
    offset: u64,
) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut c = client.lock().await;
        match c.list_datamodels(Some(count), Some(offset)).await {
            Ok(datamodels) => {
                let _ = tx.send(Action::DataModelsLoaded(Ok(datamodels))).await;
            }
            Err(e) => {
                let _ = tx.send(Action::DataModelsLoaded(Err(Arc::new(e)))).await;
            }
        }
    });
}

/// Handle loading more data models (pagination).
#[allow(dead_code)]
pub async fn handle_load_more_datamodels(
    client: SharedClient,
    tx: Sender<Action>,
    count: u64,
    offset: u64,
) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut c = client.lock().await;
        match c.list_datamodels(Some(count), Some(offset)).await {
            Ok(datamodels) => {
                let _ = tx.send(Action::MoreDataModelsLoaded(Ok(datamodels))).await;
            }
            Err(e) => {
                let _ = tx
                    .send(Action::MoreDataModelsLoaded(Err(Arc::new(e))))
                    .await;
            }
        }
    });
}
