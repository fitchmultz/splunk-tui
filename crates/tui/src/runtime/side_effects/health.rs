//! Health check side effect handlers.
//!
//! Responsibilities:
//! - Handle async API calls for health check operations.
//! - Collect health information from multiple endpoints.
//!
//! Does NOT handle:
//! - Direct state modification (sends actions for that).
//! - UI rendering.
//!
//! Note: Health check aggregation logic lives in the shared client workflow layer.
//! This handler preserves TUI-specific behavior that treats partial health failures
//! as screen errors while sharing the underlying diagnostics implementation.

use crate::action::Action;
use splunk_client::ClientError;
use splunk_client::workflows::diagnostics::run_connection_diagnostics;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

use super::{SharedClient, TaskTracker};

/// Handle loading health information from multiple endpoints.
pub async fn handle_load_health(
    client: SharedClient,
    tx: Sender<Action>,
    task_tracker: TaskTracker,
) {
    let _ = tx.send(Action::Loading(true)).await;
    task_tracker.spawn(async move {
        // Use shared health check aggregation from client crate
        match client.check_health_aggregate().await {
            Ok(health_result) => {
                // Check if there were any partial errors
                // TUI behavior: if any endpoint failed, return the first error
                // for backward compatibility with existing tests
                if let Some((endpoint, err)) = health_result.partial_errors.into_iter().next() {
                    // Convert the partial error to a ClientError for the TUI
                    let error_msg = format!("{}: {}", endpoint, err);
                    let client_error = ClientError::InvalidResponse(error_msg);
                    let _ = tx
                        .send(Action::HealthLoaded(Box::new(Err(Arc::new(client_error)))))
                        .await;
                } else {
                    // All health checks succeeded
                    let _ = tx
                        .send(Action::HealthLoaded(Box::new(Ok(health_result.output))))
                        .await;
                }
            }
            Err(e) => {
                // Server info failed - this is a critical error
                let _ = tx
                    .send(Action::HealthLoaded(Box::new(Err(Arc::new(e)))))
                    .await;
            }
        }
    });
}

/// Run comprehensive connection diagnostics.
///
/// This performs a shared diagnostics probe checking:
/// - Network reachability (server_info call)
/// - Authentication validity
/// - TLS certificate acceptance
/// - Basic endpoint metadata
pub async fn handle_run_connection_diagnostics(
    client: SharedClient,
    tx: Sender<Action>,
    task_tracker: TaskTracker,
) {
    let _ = tx.send(Action::Loading(true)).await;
    task_tracker.spawn(async move {
        let diagnostics = run_connection_diagnostics(&client, None).await;

        let _ = tx
            .send(Action::ConnectionDiagnosticsLoaded(Ok(diagnostics)))
            .await;
    });
}
