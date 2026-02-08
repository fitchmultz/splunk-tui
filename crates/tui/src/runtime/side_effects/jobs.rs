//! Job-related side effect handlers.
//!
//! Responsibilities:
//! - Handle async API calls for job operations.
//! - Fetch job lists, cancel jobs, delete jobs, and batch operations.
//!
//! Does NOT handle:
//! - Direct state modification (sends actions for that).
//! - UI rendering.

use crate::action::Action;
use crate::ui::ToastLevel;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

use super::{SharedClient, TaskTracker};

/// Handle loading jobs with pagination support.
///
/// Emits `JobsLoaded` when offset == 0 (initial load/refresh).
/// Emits `MoreJobsLoaded` when offset > 0 (pagination).
pub async fn handle_load_jobs(
    client: SharedClient,
    tx: Sender<Action>,
    task_tracker: TaskTracker,
    count: usize,
    offset: usize,
) {
    let _ = tx.send(Action::Loading(true)).await;
    task_tracker.spawn(async move {
        match client.list_jobs(Some(count), Some(offset)).await {
            Ok(jobs) => {
                if offset == 0 {
                    let _ = tx.send(Action::JobsLoaded(Ok(jobs))).await;
                } else {
                    let _ = tx.send(Action::MoreJobsLoaded(Ok(jobs))).await;
                }
            }
            Err(e) => {
                if offset == 0 {
                    let _ = tx.send(Action::JobsLoaded(Err(Arc::new(e)))).await;
                } else {
                    let _ = tx.send(Action::MoreJobsLoaded(Err(Arc::new(e)))).await;
                }
            }
        }
    });
}

/// Handle canceling a single job.
pub async fn handle_cancel_job(
    client: SharedClient,
    tx: Sender<Action>,
    task_tracker: TaskTracker,
    sid: String,
) {
    let _ = tx.send(Action::Loading(true)).await;
    task_tracker.spawn(async move {
        match client.cancel_job(&sid).await {
            Ok(_) => {
                let _ = tx
                    .send(Action::JobOperationComplete(format!(
                        "Cancelled job: {}",
                        sid
                    )))
                    .await;
                // Reload the job list (reset pagination)
                let _ = tx
                    .send(Action::LoadJobs {
                        count: 100,
                        offset: 0,
                    })
                    .await;
            }
            Err(e) => {
                let _ = tx
                    .send(Action::Notify(
                        ToastLevel::Error,
                        format!("Failed to cancel job: {}", e),
                    ))
                    .await;
                let _ = tx.send(Action::Loading(false)).await;
            }
        }
    });
}

/// Handle deleting a single job.
pub async fn handle_delete_job(
    client: SharedClient,
    tx: Sender<Action>,
    task_tracker: TaskTracker,
    sid: String,
) {
    let _ = tx.send(Action::Loading(true)).await;
    task_tracker.spawn(async move {
        match client.delete_job(&sid).await {
            Ok(_) => {
                let _ = tx
                    .send(Action::JobOperationComplete(format!(
                        "Deleted job: {}",
                        sid
                    )))
                    .await;
                // Reload the job list (reset pagination)
                let _ = tx
                    .send(Action::LoadJobs {
                        count: 100,
                        offset: 0,
                    })
                    .await;
            }
            Err(e) => {
                let _ = tx
                    .send(Action::Notify(
                        ToastLevel::Error,
                        format!("Failed to delete job: {}", e),
                    ))
                    .await;
                let _ = tx.send(Action::Loading(false)).await;
            }
        }
    });
}

/// Handle canceling multiple jobs in a batch.
pub async fn handle_cancel_jobs_batch(
    client: SharedClient,
    tx: Sender<Action>,
    task_tracker: TaskTracker,
    sids: Vec<String>,
) {
    let _ = tx.send(Action::Loading(true)).await;
    let tx_clone = tx.clone();
    task_tracker.spawn(async move {
        let mut success_count = 0;
        let mut error_messages = Vec::new();

        // Process jobs sequentially to avoid overwhelming the API
        // and to provide clear per-job error reporting.
        // Parallelizing with join_all would require careful rate limiting
        // to avoid triggering Splunk's API throttling.
        for sid in sids {
            match client.cancel_job(&sid).await {
                Ok(_) => {
                    success_count += 1;
                }
                Err(e) => {
                    error_messages.push(format!("{}: {}", sid, e));
                }
            }
        }

        let msg = if success_count > 0 {
            format!("Cancelled {} job(s)", success_count)
        } else {
            "No jobs cancelled".to_string()
        };

        if !error_messages.is_empty() {
            for err in error_messages {
                let _ = tx_clone.send(Action::Notify(ToastLevel::Error, err)).await;
            }
        }

        let _ = tx_clone.send(Action::JobOperationComplete(msg)).await;
        let _ = tx_clone
            .send(Action::LoadJobs {
                count: 100,
                offset: 0,
            })
            .await;
    });
}

/// Handle deleting multiple jobs in a batch.
pub async fn handle_delete_jobs_batch(
    client: SharedClient,
    tx: Sender<Action>,
    task_tracker: TaskTracker,
    sids: Vec<String>,
) {
    let _ = tx.send(Action::Loading(true)).await;
    let tx_clone = tx.clone();
    task_tracker.spawn(async move {
        let mut success_count = 0;
        let mut error_messages = Vec::new();

        // Process jobs sequentially to avoid overwhelming the API
        // and to provide clear per-job error reporting.
        // See CancelJobsBatch for parallelization considerations.
        for sid in sids {
            match client.delete_job(&sid).await {
                Ok(_) => {
                    success_count += 1;
                }
                Err(e) => {
                    error_messages.push(format!("{}: {}", sid, e));
                }
            }
        }

        let msg = if success_count > 0 {
            format!("Deleted {} job(s)", success_count)
        } else {
            "No jobs deleted".to_string()
        };

        if !error_messages.is_empty() {
            for err in error_messages {
                let _ = tx_clone.send(Action::Notify(ToastLevel::Error, err)).await;
            }
        }

        let _ = tx_clone.send(Action::JobOperationComplete(msg)).await;
        let _ = tx_clone
            .send(Action::LoadJobs {
                count: 100,
                offset: 0,
            })
            .await;
    });
}
