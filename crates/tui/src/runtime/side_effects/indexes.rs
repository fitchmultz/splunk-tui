//! Index-related side effect handlers.
//!
//! Responsibilities:
//! - Handle async API calls for index operations.
//! - Fetch index lists from the Splunk server.
//! - Create, modify, and delete indexes.
//!
//! Does NOT handle:
//! - Direct state modification (sends actions for that).
//! - UI rendering.

use crate::action::Action;
use crate::ui::ToastLevel;
use splunk_client::{CreateIndexParams, ModifyIndexParams};
use splunk_config::constants::DEFAULT_LIST_PAGE_SIZE;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

use super::paginated::build_paginated_action;
use super::{SharedClient, TaskTracker};

/// Handle loading indexes with pagination support.
///
/// Emits `IndexesLoaded` when offset == 0 (initial load/refresh).
/// Emits `MoreIndexesLoaded` when offset > 0 (pagination).
pub async fn handle_load_indexes(
    client: SharedClient,
    tx: Sender<Action>,
    task_tracker: TaskTracker,
    count: usize,
    offset: usize,
) {
    let _ = tx.send(Action::Loading(true)).await;
    task_tracker.spawn(async move {
        let result = client.list_indexes(Some(count), Some(offset)).await;
        let action = build_paginated_action(
            result,
            offset,
            Action::IndexesLoaded,
            Action::MoreIndexesLoaded,
        );
        let _ = tx.send(action).await;
    });
}

/// Handle creating a new index.
pub async fn handle_create_index(
    client: SharedClient,
    tx: Sender<Action>,
    task_tracker: TaskTracker,
    params: CreateIndexParams,
) {
    let _ = tx.send(Action::Loading(true)).await;
    task_tracker.spawn(async move {
        match client.create_index(&params).await {
            Ok(index) => {
                let _ = tx
                    .send(Action::Notify(
                        ToastLevel::Success,
                        format!("Index '{}' created successfully", index.name),
                    ))
                    .await;
                let _ = tx.send(Action::IndexCreated(Ok(index))).await;
                // Refresh indexes list
                let _ = tx
                    .send(Action::LoadIndexes {
                        count: DEFAULT_LIST_PAGE_SIZE,
                        offset: 0,
                    })
                    .await;
            }
            Err(e) => {
                let _ = tx
                    .send(Action::Notify(
                        ToastLevel::Error,
                        format!("Failed to create index '{}': {}", params.name, e),
                    ))
                    .await;
                let _ = tx.send(Action::IndexCreated(Err(Arc::new(e)))).await;
                let _ = tx.send(Action::Loading(false)).await;
            }
        }
    });
}

/// Handle modifying an existing index.
pub async fn handle_modify_index(
    client: SharedClient,
    tx: Sender<Action>,
    task_tracker: TaskTracker,
    name: String,
    params: ModifyIndexParams,
) {
    let _ = tx.send(Action::Loading(true)).await;
    task_tracker.spawn(async move {
        match client.modify_index(&name, &params).await {
            Ok(index) => {
                let _ = tx
                    .send(Action::Notify(
                        ToastLevel::Success,
                        format!("Index '{}' modified successfully", index.name),
                    ))
                    .await;
                let _ = tx.send(Action::IndexModified(Ok(index))).await;
                // Refresh indexes list
                let _ = tx
                    .send(Action::LoadIndexes {
                        count: DEFAULT_LIST_PAGE_SIZE,
                        offset: 0,
                    })
                    .await;
            }
            Err(e) => {
                let _ = tx
                    .send(Action::Notify(
                        ToastLevel::Error,
                        format!("Failed to modify index '{}': {}", name, e),
                    ))
                    .await;
                let _ = tx.send(Action::IndexModified(Err(Arc::new(e)))).await;
                let _ = tx.send(Action::Loading(false)).await;
            }
        }
    });
}

/// Handle deleting an index.
pub async fn handle_delete_index(
    client: SharedClient,
    tx: Sender<Action>,
    task_tracker: TaskTracker,
    name: String,
) {
    let _ = tx.send(Action::Loading(true)).await;
    task_tracker.spawn(async move {
        match client.delete_index(&name).await {
            Ok(()) => {
                let _ = tx
                    .send(Action::Notify(
                        ToastLevel::Success,
                        format!("Index '{}' deleted successfully", name),
                    ))
                    .await;
                let _ = tx.send(Action::IndexDeleted(Ok(name))).await;
                // Refresh indexes list
                let _ = tx
                    .send(Action::LoadIndexes {
                        count: DEFAULT_LIST_PAGE_SIZE,
                        offset: 0,
                    })
                    .await;
            }
            Err(e) => {
                let _ = tx
                    .send(Action::Notify(
                        ToastLevel::Error,
                        format!("Failed to delete index '{}': {}", name, e),
                    ))
                    .await;
                let _ = tx.send(Action::IndexDeleted(Err(Arc::new(e)))).await;
                let _ = tx.send(Action::Loading(false)).await;
            }
        }
    });
}
