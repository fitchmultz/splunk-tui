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
use tokio::sync::mpsc::Sender;

use super::paginated::build_paginated_action;
use super::{SharedClient, TaskTracker};

/// Handle loading data models with pagination support.
///
/// Emits `DataModelsLoaded` when offset == 0 (initial load/refresh).
/// Emits `MoreDataModelsLoaded` when offset > 0 (pagination).
pub async fn handle_load_datamodels(
    client: SharedClient,
    tx: Sender<Action>,
    task_tracker: TaskTracker,
    count: usize,
    offset: usize,
) {
    let _ = tx.send(Action::Loading(true)).await;
    task_tracker.spawn(async move {
        let result = client.list_datamodels(Some(count), Some(offset)).await;
        let action = build_paginated_action(
            result,
            offset,
            Action::DataModelsLoaded,
            Action::MoreDataModelsLoaded,
        );
        let _ = tx.send(action).await;
    });
}
