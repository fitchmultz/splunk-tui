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
//! Note: Health check aggregation logic has been moved to the shared client crate
//! (`splunk_client::SplunkClient::check_health_aggregate`) to avoid duplication
//! between CLI and TUI. This handler maintains TUI-specific behavior of failing
//! if any health check endpoint fails (for backward compatibility with existing tests).

use crate::action::Action;
use splunk_client::ClientError;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

use super::SharedClient;

/// Handle loading health information from multiple endpoints.
pub async fn handle_load_health(client: SharedClient, tx: Sender<Action>) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut c = client.lock().await;

        // Use shared health check aggregation from client crate
        match c.check_health_aggregate().await {
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
