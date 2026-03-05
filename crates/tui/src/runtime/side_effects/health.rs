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

use crate::action::{
    Action,
    variants::{ConnectionDiagnosticsResult, DiagnosticCheck, DiagnosticStatus, ServerInfoSummary},
};
use splunk_client::ClientError;
use std::sync::Arc;
use std::time::Instant;
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
/// This performs a single controlled probe (no retries) checking:
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
        let start = Instant::now();
        let mut remediation_hints = Vec::new();

        let result = client.check_health_aggregate().await;

        let diagnostics = match result {
            Ok(health) => {
                let server_info = health.output.server_info.map(|si| ServerInfoSummary {
                    version: si.version,
                    build: si.build,
                    server_name: si.server_name,
                    mode: si.mode.map(|m| m.to_string()),
                });

                let reachable = DiagnosticCheck {
                    name: "Reachability".to_string(),
                    status: DiagnosticStatus::Pass,
                    error: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                };

                let auth = DiagnosticCheck {
                    name: "Authentication".to_string(),
                    status: DiagnosticStatus::Pass,
                    error: None,
                    duration_ms: 0,
                };

                let tls = DiagnosticCheck {
                    name: "TLS Certificate".to_string(),
                    status: DiagnosticStatus::Pass,
                    error: None,
                    duration_ms: 0,
                };

                for (endpoint, err) in &health.partial_errors {
                    remediation_hints.push(format!("{} endpoint returned: {}", endpoint, err));
                }

                let overall_status = if health.partial_errors.is_empty() {
                    DiagnosticStatus::Pass
                } else {
                    DiagnosticStatus::Fail
                };

                ConnectionDiagnosticsResult {
                    reachable,
                    auth,
                    tls,
                    server_info,
                    overall_status,
                    remediation_hints,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                }
            }
            Err(e) => {
                let (reachable_status, auth_status, tls_status) =
                    categorize_connection_error(&e);

                if reachable_status == DiagnosticStatus::Fail {
                    remediation_hints.push("Check that the Splunk server is running".to_string());
                    remediation_hints.push("Verify the URL and port are correct".to_string());
                }
                if auth_status == DiagnosticStatus::Fail {
                    remediation_hints
                        .push("Verify your username and password".to_string());
                    remediation_hints.push("Check that your API token is valid".to_string());
                }
                if tls_status == DiagnosticStatus::Fail {
                    remediation_hints.push(
                        "If using self-signed certs, enable 'Skip TLS Verification' in profile settings".to_string(),
                    );
                }

                ConnectionDiagnosticsResult {
                    reachable: DiagnosticCheck {
                        name: "Reachability".to_string(),
                        status: reachable_status,
                        error: Some(e.to_string()),
                        duration_ms: start.elapsed().as_millis() as u64,
                    },
                    auth: DiagnosticCheck {
                        name: "Authentication".to_string(),
                        status: auth_status,
                        error: if auth_status == DiagnosticStatus::Fail {
                            Some(e.to_string())
                        } else {
                            None
                        },
                        duration_ms: 0,
                    },
                    tls: DiagnosticCheck {
                        name: "TLS Certificate".to_string(),
                        status: tls_status,
                        error: if tls_status == DiagnosticStatus::Fail {
                            Some(e.to_string())
                        } else {
                            None
                        },
                        duration_ms: 0,
                    },
                    server_info: None,
                    overall_status: DiagnosticStatus::Fail,
                    remediation_hints,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                }
            }
        };

        let _ = tx
            .send(Action::ConnectionDiagnosticsLoaded(Ok(diagnostics)))
            .await;
    });
}

/// Categorize a connection error into reachability, auth, and TLS status.
fn categorize_connection_error(
    e: &ClientError,
) -> (DiagnosticStatus, DiagnosticStatus, DiagnosticStatus) {
    let error_str = e.to_string().to_lowercase();

    if error_str.contains("certificate") || error_str.contains("tls") || error_str.contains("ssl") {
        return (
            DiagnosticStatus::Pass,
            DiagnosticStatus::Skip,
            DiagnosticStatus::Fail,
        );
    }

    if error_str.contains("401")
        || error_str.contains("unauthorized")
        || error_str.contains("authentication")
        || e.is_auth_error()
    {
        return (
            DiagnosticStatus::Pass,
            DiagnosticStatus::Fail,
            DiagnosticStatus::Pass,
        );
    }

    if error_str.contains("connect")
        || error_str.contains("timeout")
        || error_str.contains("refused")
        || error_str.contains("dns")
    {
        return (
            DiagnosticStatus::Fail,
            DiagnosticStatus::Skip,
            DiagnosticStatus::Skip,
        );
    }

    (
        DiagnosticStatus::Fail,
        DiagnosticStatus::Skip,
        DiagnosticStatus::Skip,
    )
}
