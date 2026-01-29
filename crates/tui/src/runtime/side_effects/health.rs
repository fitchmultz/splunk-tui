//! Health check side effect handlers.
//!
//! Responsibilities:
//! - Handle async API calls for health check operations.
//! - Collect health information from multiple endpoints.
//!
//! Does NOT handle:
//! - Direct state modification (sends actions for that).
//! - UI rendering.

use crate::action::Action;
use splunk_client::ClientError;
use splunk_client::models::HealthCheckOutput;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

use super::SharedClient;

/// Handle loading health information from multiple endpoints.
pub async fn handle_load_health(client: SharedClient, tx: Sender<Action>) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut c = client.lock().await;

        // Construct the HealthCheckOutput
        let mut health_output = HealthCheckOutput {
            server_info: None,
            splunkd_health: None,
            license_usage: None,
            kvstore_status: None,
            log_parsing_health: None,
        };

        let mut first_error: Option<ClientError> = None;

        // Collect health info sequentially due to the &mut self requirement
        // on client methods. Each call may need to refresh the session token,
        // requiring exclusive access to the client.
        //
        // Parallelization options:
        // 1. Spawn 5 separate tasks (each waits for the same mutex - minimal gain)
        // 2. Refactor client to support concurrent calls (significant effort)
        // 3. Use a connection pool (adds complexity for health checks only)
        //
        // Given that health checks run infrequently and network latency
        // dominates, sequential execution is the pragmatic choice.
        match c.get_server_info().await {
            Ok(info) => health_output.server_info = Some(info),
            Err(e) => {
                if first_error.is_none() {
                    first_error = Some(e);
                }
            }
        }

        match c.get_health().await {
            Ok(health) => health_output.splunkd_health = Some(health),
            Err(e) => {
                if first_error.is_none() {
                    first_error = Some(e);
                }
            }
        }

        match c.get_license_usage().await {
            Ok(license) => health_output.license_usage = Some(license),
            Err(e) => {
                if first_error.is_none() {
                    first_error = Some(e);
                }
            }
        }

        match c.get_kvstore_status().await {
            Ok(kvstore) => health_output.kvstore_status = Some(kvstore),
            Err(e) => {
                if first_error.is_none() {
                    first_error = Some(e);
                }
            }
        }

        match c.check_log_parsing_health().await {
            Ok(log_parsing) => health_output.log_parsing_health = Some(log_parsing),
            Err(e) => {
                if first_error.is_none() {
                    first_error = Some(e);
                }
            }
        }

        if let Some(e) = first_error {
            let _ = tx
                .send(Action::HealthLoaded(Box::new(Err(Arc::new(e)))))
                .await;
        } else {
            let _ = tx
                .send(Action::HealthLoaded(Box::new(Ok(health_output))))
                .await;
        }
    });
}
