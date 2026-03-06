//! Export-related side effect handlers.
//!
//! Responsibilities:
//! - Handle async data export operations.
//!
//! Does NOT handle:
//! - Direct state modification (sends actions for that).
//! - UI rendering.

use crate::action::Action;
use crate::action::ExportFormat;
use crate::runtime::side_effects::TaskTracker;
use crate::ui::ToastLevel;
use serde_json::Value;
use std::path::PathBuf;
use tokio::sync::mpsc::Sender;

/// Handle exporting data to a file.
pub async fn handle_export_data(
    data: Value,
    path: PathBuf,
    format: ExportFormat,
    tx: Sender<Action>,
    task_tracker: TaskTracker,
) {
    task_tracker.spawn(async move {
        match crate::export::export_value(&data, &path, format).await {
            Ok(_) => {
                let _ = tx
                    .send(Action::Notify(
                        ToastLevel::Info,
                        format!("Exported to {}", path.display()),
                    ))
                    .await;
                let _ = tx.send(Action::ExportSuccess(path)).await;
            }
            Err(e) => {
                let _ = tx
                    .send(Action::Notify(
                        ToastLevel::Error,
                        format!("Export failed: {}", e),
                    ))
                    .await;
            }
        }
    });
}
