//! Overview screen side effect handlers.
//!
//! Responsibilities:
//! - Handle async API calls for overview resource aggregation.
//! - Fetch all resource types and compile into OverviewData.
//!
//! Does NOT handle:
//! - Direct state modification (sends actions for that).
//! - UI rendering.

use crate::action::{Action, OverviewData};
use tokio::sync::mpsc::Sender;

use super::{SharedClient, TaskTracker, overview_fetch};

/// Handle loading overview information from all resource endpoints.
pub async fn handle_load_overview(
    client: SharedClient,
    tx: Sender<Action>,
    task_tracker: TaskTracker,
) {
    let _ = tx.send(Action::Loading(true)).await;
    task_tracker.spawn(async move {
        let mut resources = Vec::new();

        // Fetch each resource type with timeout
        // Individual failures are converted to error entries rather than failing the entire overview

        // indexes
        match overview_fetch::fetch_indexes(&client).await {
            Ok(r) => resources.push(r),
            Err(e) => resources.push(overview_fetch::resource_error("indexes", e)),
        }

        // jobs
        match overview_fetch::fetch_jobs(&client).await {
            Ok(r) => resources.push(r),
            Err(e) => resources.push(overview_fetch::resource_error("jobs", e)),
        }

        // apps
        match overview_fetch::fetch_apps(&client).await {
            Ok(r) => resources.push(r),
            Err(e) => resources.push(overview_fetch::resource_error("apps", e)),
        }

        // users
        match overview_fetch::fetch_users(&client).await {
            Ok(r) => resources.push(r),
            Err(e) => resources.push(overview_fetch::resource_error("users", e)),
        }

        // cluster
        match overview_fetch::fetch_cluster(&client).await {
            Ok(r) => resources.push(r),
            Err(e) => resources.push(overview_fetch::resource_error("cluster", e)),
        }

        // health
        match overview_fetch::fetch_health(&client).await {
            Ok(r) => resources.push(r),
            Err(e) => resources.push(overview_fetch::resource_error("health", e)),
        }

        // kvstore
        match overview_fetch::fetch_kvstore(&client).await {
            Ok(r) => resources.push(r),
            Err(e) => resources.push(overview_fetch::resource_error("kvstore", e)),
        }

        // license
        match overview_fetch::fetch_license(&client).await {
            Ok(r) => resources.push(r),
            Err(e) => resources.push(overview_fetch::resource_error("license", e)),
        }

        // saved-searches
        match overview_fetch::fetch_saved_searches(&client).await {
            Ok(r) => resources.push(r),
            Err(e) => resources.push(overview_fetch::resource_error("saved-searches", e)),
        }

        let overview_data = OverviewData { resources };
        let _ = tx.send(Action::OverviewLoaded(overview_data)).await;
    });
}
