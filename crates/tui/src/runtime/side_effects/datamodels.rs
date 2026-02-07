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

/// Handle loading data models with pagination support.
///
/// Emits `DataModelsLoaded` when offset == 0 (initial load/refresh).
/// Emits `MoreDataModelsLoaded` when offset > 0 (pagination).
pub async fn handle_load_datamodels(
    client: SharedClient,
    tx: Sender<Action>,
    count: u64,
    offset: u64,
) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        match client.list_datamodels(Some(count), Some(offset)).await {
            Ok(datamodels) => {
                if offset == 0 {
                    let _ = tx.send(Action::DataModelsLoaded(Ok(datamodels))).await;
                } else {
                    let _ = tx.send(Action::MoreDataModelsLoaded(Ok(datamodels))).await;
                }
            }
            Err(e) => {
                if offset == 0 {
                    let _ = tx.send(Action::DataModelsLoaded(Err(Arc::new(e)))).await;
                } else {
                    let _ = tx
                        .send(Action::MoreDataModelsLoaded(Err(Arc::new(e))))
                        .await;
                }
            }
        }
    });
}
