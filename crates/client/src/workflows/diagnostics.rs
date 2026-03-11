//! Shared diagnostics workflow models and execution helpers.
//!
//! Purpose:
//! - Provide shared doctor/diagnostics data models and probe orchestration for CLI and TUI.
//!
//! Responsibilities:
//! - Build frontend-neutral diagnostic reports from configuration and client health probes.
//! - Classify connection diagnostics into reachability/auth/TLS results.
//! - Produce redacted support-bundle report views.
//!
//! Scope:
//! - Shared diagnostics data contracts and probe execution only.
//!
//! Usage:
//! - CLI doctor uses `run_doctor_report`.
//! - TUI connection diagnostics uses `run_connection_diagnostics`.
//!
//! Invariants/Assumptions:
//! - Output timestamps are RFC3339.
//! - Support-bundle reports exclude sensitive server payloads and partial errors.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use splunk_config::Config;
use std::path::PathBuf;
use std::time::Instant;

use crate::models::kvstore::KvStoreMemberStatus;
use crate::workflows::{CancellationProbe, ensure_not_cancelled};
use crate::{ClientError, HealthCheckOutput, SplunkClient};

/// Result of a single shared diagnostic check.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiagnosticCheck {
    pub name: String,
    pub status: CheckStatus,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// Status of a diagnostic check in doctor output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Pass,
    Fail,
    Warning,
    Skipped,
}

impl CheckStatus {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Warning => "warning",
            Self::Skipped => "skipped",
        }
    }

    pub const fn table_badge(self) -> &'static str {
        match self {
            Self::Pass => "[PASS]",
            Self::Fail => "[FAIL]",
            Self::Warning => "[WARN]",
            Self::Skipped => "[SKIP]",
        }
    }

    pub const fn markdown_badge(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Fail => "FAIL",
            Self::Warning => "WARN",
            Self::Skipped => "SKIP",
        }
    }
}

/// Redacted configuration summary for diagnostics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigSummary {
    pub config_source: String,
    pub profile_name: Option<String>,
    pub config_path: Option<PathBuf>,
    pub base_url: String,
    pub auth_strategy: String,
    pub skip_verify: bool,
    pub timeout_secs: u64,
    pub max_retries: usize,
}

/// Full doctor report shared across frontends.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticReport {
    pub cli_version: String,
    pub os_arch: String,
    pub timestamp: String,
    pub config_summary: ConfigSummary,
    pub checks: Vec<DiagnosticCheck>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_output: Option<HealthCheckOutput>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub partial_errors: Vec<(String, String)>,
}

impl DiagnosticReport {
    pub fn to_bundle_report(&self) -> BundleDiagnosticReport {
        BundleDiagnosticReport {
            cli_version: self.cli_version.clone(),
            os_arch: self.os_arch.clone(),
            timestamp: self.timestamp.clone(),
            config_summary: self.config_summary.clone(),
            checks: self.checks.clone(),
        }
    }
}

/// Redacted support-bundle report.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BundleDiagnosticReport {
    pub cli_version: String,
    pub os_arch: String,
    pub timestamp: String,
    pub config_summary: ConfigSummary,
    pub checks: Vec<DiagnosticCheck>,
}

/// TUI/interactive connection diagnostic status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticStatus {
    Pass,
    Fail,
    Skip,
}

/// TUI/interactive diagnostic check result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionCheck {
    pub name: String,
    pub status: DiagnosticStatus,
    pub error: Option<String>,
    pub duration_ms: u64,
}

/// Summarized server metadata for interactive diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerInfoSummary {
    pub version: String,
    pub build: String,
    pub server_name: String,
    pub mode: Option<String>,
}

/// Interactive connection diagnostics result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionDiagnosticsResult {
    pub reachable: ConnectionCheck,
    pub auth: ConnectionCheck,
    pub tls: ConnectionCheck,
    pub server_info: Option<ServerInfoSummary>,
    pub overall_status: DiagnosticStatus,
    pub remediation_hints: Vec<String>,
    pub timestamp: String,
}

/// Run the shared doctor workflow.
pub async fn run_doctor_report(
    config: &Config,
    cli_version: impl Into<String>,
    no_cache: bool,
    cancel: Option<&dyn CancellationProbe>,
) -> Result<DiagnosticReport> {
    ensure_not_cancelled(cancel)?;
    let mut checks = Vec::new();
    checks.push(run_config_check(config));
    checks.push(run_auth_check(config));

    let client = SplunkClient::builder()
        .from_config(config)
        .maybe_no_cache(no_cache)
        .build();

    let client = match client {
        Ok(client) => {
            checks.push(DiagnosticCheck {
                name: "client_build".to_string(),
                status: CheckStatus::Pass,
                message: "Successfully built Splunk client".to_string(),
                details: None,
            });
            Some(client)
        }
        Err(error) => {
            checks.push(DiagnosticCheck {
                name: "client_build".to_string(),
                status: CheckStatus::Fail,
                message: format!("Failed to build client: {error}"),
                details: None,
            });
            None
        }
    };

    let (health_output, partial_errors) = if let Some(client) = client {
        ensure_not_cancelled(cancel)?;
        match client.check_health_aggregate().await {
            Ok(health) => {
                ensure_not_cancelled(cancel)?;
                let server_name = health
                    .output
                    .server_info
                    .as_ref()
                    .map(|server| server.server_name.clone())
                    .unwrap_or_default();
                let version = health
                    .output
                    .server_info
                    .as_ref()
                    .map(|server| server.version.clone())
                    .unwrap_or_default();

                checks.push(DiagnosticCheck {
                    name: "server_connectivity".to_string(),
                    status: CheckStatus::Pass,
                    message: format!("Connected to {} ({server_name})", client.base_url()),
                    details: Some(serde_json::json!({
                        "server_name": server_name,
                        "version": version,
                    })),
                });

                if health.output.license_usage.is_some() {
                    checks.push(DiagnosticCheck {
                        name: "license_status".to_string(),
                        status: CheckStatus::Pass,
                        message: "License information retrieved".to_string(),
                        details: None,
                    });
                }

                if let Some(ref kvstore) = health.output.kvstore_status {
                    checks.push(DiagnosticCheck {
                        name: "kvstore_status".to_string(),
                        status: if kvstore.current_member.status == KvStoreMemberStatus::Ready {
                            CheckStatus::Pass
                        } else {
                            CheckStatus::Warning
                        },
                        message: format!("KVStore status: {}", kvstore.current_member.status),
                        details: None,
                    });
                }

                let partial_errors = health
                    .partial_errors
                    .into_iter()
                    .map(|(name, err)| (name, err.to_string()))
                    .collect();

                (Some(health.output), partial_errors)
            }
            Err(error) => {
                checks.push(DiagnosticCheck {
                    name: "server_connectivity".to_string(),
                    status: CheckStatus::Fail,
                    message: format!("Failed to connect: {error}"),
                    details: None,
                });
                (None, Vec::new())
            }
        }
    } else {
        (None, Vec::new())
    };

    ensure_not_cancelled(cancel)?;
    Ok(DiagnosticReport {
        cli_version: cli_version.into(),
        os_arch: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
        timestamp: chrono::Utc::now().to_rfc3339(),
        config_summary: summarize_config(config),
        checks,
        health_output,
        partial_errors,
    })
}

/// Run interactive connection diagnostics.
pub async fn run_connection_diagnostics(
    client: &SplunkClient,
    cancel: Option<&dyn CancellationProbe>,
) -> ConnectionDiagnosticsResult {
    let start = Instant::now();
    let mut remediation_hints = Vec::new();

    if ensure_not_cancelled(cancel).is_err() {
        return cancelled_diagnostics_result(start);
    }

    match client.check_health_aggregate().await {
        Ok(health) => {
            if ensure_not_cancelled(cancel).is_err() {
                return cancelled_diagnostics_result(start);
            }
            let server_info = health.output.server_info.map(|server| ServerInfoSummary {
                version: server.version,
                build: server.build,
                server_name: server.server_name,
                mode: server.mode.map(|mode| mode.to_string()),
            });

            for (endpoint, error) in &health.partial_errors {
                remediation_hints.push(format!("{endpoint} endpoint returned: {error}"));
            }

            ConnectionDiagnosticsResult {
                reachable: ConnectionCheck {
                    name: "Reachability".to_string(),
                    status: DiagnosticStatus::Pass,
                    error: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                },
                auth: ConnectionCheck {
                    name: "Authentication".to_string(),
                    status: DiagnosticStatus::Pass,
                    error: None,
                    duration_ms: 0,
                },
                tls: ConnectionCheck {
                    name: "TLS Certificate".to_string(),
                    status: DiagnosticStatus::Pass,
                    error: None,
                    duration_ms: 0,
                },
                server_info,
                overall_status: if health.partial_errors.is_empty() {
                    DiagnosticStatus::Pass
                } else {
                    DiagnosticStatus::Fail
                },
                remediation_hints,
                timestamp: chrono::Utc::now().to_rfc3339(),
            }
        }
        Err(error) => {
            let (reachable_status, auth_status, tls_status) = categorize_connection_error(&error);

            if reachable_status == DiagnosticStatus::Fail {
                remediation_hints.push("Check that the Splunk server is running".to_string());
                remediation_hints.push("Verify the URL and port are correct".to_string());
            }
            if auth_status == DiagnosticStatus::Fail {
                remediation_hints.push("Verify your username and password".to_string());
                remediation_hints.push("Check that your API token is valid".to_string());
            }
            if tls_status == DiagnosticStatus::Fail {
                remediation_hints.push(
                    "If using self-signed certs, enable skip TLS verification in profile settings"
                        .to_string(),
                );
            }

            ConnectionDiagnosticsResult {
                reachable: ConnectionCheck {
                    name: "Reachability".to_string(),
                    status: reachable_status,
                    error: Some(error.to_string()),
                    duration_ms: start.elapsed().as_millis() as u64,
                },
                auth: ConnectionCheck {
                    name: "Authentication".to_string(),
                    status: auth_status,
                    error: (auth_status == DiagnosticStatus::Fail).then(|| error.to_string()),
                    duration_ms: 0,
                },
                tls: ConnectionCheck {
                    name: "TLS Certificate".to_string(),
                    status: tls_status,
                    error: (tls_status == DiagnosticStatus::Fail).then(|| error.to_string()),
                    duration_ms: 0,
                },
                server_info: None,
                overall_status: DiagnosticStatus::Fail,
                remediation_hints,
                timestamp: chrono::Utc::now().to_rfc3339(),
            }
        }
    }
}

fn cancelled_diagnostics_result(start: Instant) -> ConnectionDiagnosticsResult {
    ConnectionDiagnosticsResult {
        reachable: ConnectionCheck {
            name: "Reachability".to_string(),
            status: DiagnosticStatus::Skip,
            error: Some("workflow cancelled".to_string()),
            duration_ms: start.elapsed().as_millis() as u64,
        },
        auth: ConnectionCheck {
            name: "Authentication".to_string(),
            status: DiagnosticStatus::Skip,
            error: None,
            duration_ms: 0,
        },
        tls: ConnectionCheck {
            name: "TLS Certificate".to_string(),
            status: DiagnosticStatus::Skip,
            error: None,
            duration_ms: 0,
        },
        server_info: None,
        overall_status: DiagnosticStatus::Skip,
        remediation_hints: vec!["Operation cancelled before diagnostics completed".to_string()],
        timestamp: chrono::Utc::now().to_rfc3339(),
    }
}

fn summarize_config(config: &Config) -> ConfigSummary {
    ConfigSummary {
        config_source: "resolved".to_string(),
        profile_name: None,
        config_path: None,
        base_url: config.connection.base_url.clone(),
        auth_strategy: match &config.auth.strategy {
            splunk_config::AuthStrategy::ApiToken { .. } => "api_token".to_string(),
            splunk_config::AuthStrategy::SessionToken { .. } => "session_token".to_string(),
        },
        skip_verify: config.connection.skip_verify,
        timeout_secs: config.connection.timeout.as_secs(),
        max_retries: config.connection.max_retries,
    }
}

fn run_config_check(config: &Config) -> DiagnosticCheck {
    DiagnosticCheck {
        name: "config".to_string(),
        status: CheckStatus::Pass,
        message: "Configuration loaded successfully".to_string(),
        details: Some(serde_json::json!({
            "base_url": config.connection.base_url,
            "profile": serde_json::Value::Null,
            "skip_verify": config.connection.skip_verify,
        })),
    }
}

fn run_auth_check(config: &Config) -> DiagnosticCheck {
    let (auth_strategy, details) = match &config.auth.strategy {
        splunk_config::AuthStrategy::ApiToken { .. } => (
            "API token",
            serde_json::json!({
                "strategy": "api_token",
            }),
        ),
        splunk_config::AuthStrategy::SessionToken { username, .. } => (
            "Username/password",
            serde_json::json!({
                "strategy": "session_token",
                "username": username,
            }),
        ),
    };

    DiagnosticCheck {
        name: "auth_strategy".to_string(),
        status: CheckStatus::Pass,
        message: format!("Using {auth_strategy} authentication"),
        details: Some(details),
    }
}

fn categorize_connection_error(
    error: &ClientError,
) -> (DiagnosticStatus, DiagnosticStatus, DiagnosticStatus) {
    let error_str = error.to_string().to_lowercase();

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
        || error.is_auth_error()
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

trait MaybeNoCache {
    fn maybe_no_cache(self, no_cache: bool) -> Self;
}

impl MaybeNoCache for crate::client::builder::SplunkClientBuilder {
    fn maybe_no_cache(self, no_cache: bool) -> Self {
        if no_cache { self.no_cache() } else { self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AuthStrategy;
    use crate::testing::load_fixture;
    use secrecy::SecretString;
    use splunk_config::{AuthConfig, ConnectionConfig};
    use std::time::Duration;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[derive(Debug)]
    struct TestCancel(bool);

    impl CancellationProbe for TestCancel {
        fn is_cancelled(&self) -> bool {
            self.0
        }
    }

    fn api_token_config(base_url: String) -> Config {
        Config {
            connection: ConnectionConfig {
                base_url,
                skip_verify: true,
                timeout: Duration::from_secs(30),
                max_retries: 3,
                session_expiry_buffer_seconds: 60,
                session_ttl_seconds: 3600,
                health_check_interval_seconds: 30,
                circuit_breaker_enabled: true,
                circuit_failure_threshold: 5,
                circuit_failure_window_seconds: 60,
                circuit_reset_timeout_seconds: 30,
                circuit_half_open_requests: 1,
            },
            auth: AuthConfig {
                strategy: splunk_config::AuthStrategy::ApiToken {
                    token: SecretString::new("test-token".to_string().into()),
                },
            },
        }
    }

    async fn mount_connection_diagnostics_mocks(
        mock_server: &MockServer,
        with_partial_errors: bool,
    ) {
        Mock::given(method("GET"))
            .and(path("/services/server/info"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(load_fixture("server/get_server_info.json")),
            )
            .mount(mock_server)
            .await;

        let health_response = if with_partial_errors {
            ResponseTemplate::new(503).set_body_json(serde_json::json!({
                "messages": [{"type": "ERROR", "text": "splunkd health unavailable"}]
            }))
        } else {
            ResponseTemplate::new(200).set_body_json(load_fixture("server/get_health.json"))
        };
        Mock::given(method("GET"))
            .and(path("/services/server/health/splunkd"))
            .respond_with(health_response)
            .mount(mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/services/licenser/usage"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(load_fixture("license/get_usage.json")),
            )
            .mount(mock_server)
            .await;

        let kvstore_response = if with_partial_errors {
            ResponseTemplate::new(503).set_body_json(serde_json::json!({
                "messages": [{"type": "ERROR", "text": "kvstore unavailable"}]
            }))
        } else {
            ResponseTemplate::new(200).set_body_json(load_fixture("kvstore/status.json"))
        };
        Mock::given(method("GET"))
            .and(path("/services/kvstore/status"))
            .respond_with(kvstore_response)
            .mount(mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/services/search/jobs"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "entry": [{"content": {"sid": "diag-job"}}]
            })))
            .mount(mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/services/search/jobs/diag-job"))
            .and(query_param("output_mode", "json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "entry": [{
                    "content": {
                        "sid": "diag-job",
                        "isDone": true,
                        "doneProgress": 1.0,
                        "runDuration": 0.2,
                        "scanCount": 0,
                        "eventCount": 0,
                        "resultCount": 0,
                        "diskUsage": 0
                    }
                }]
            })))
            .mount(mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/services/search/jobs/diag-job/results"))
            .and(query_param("output_mode", "json"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(load_fixture("logs/get_internal_logs_empty.json")),
            )
            .mount(mock_server)
            .await;
    }

    #[test]
    fn bundle_report_omits_sensitive_health_payload_and_partial_errors() {
        let report = DiagnosticReport {
            cli_version: "1.0.0".to_string(),
            os_arch: "macos-aarch64".to_string(),
            timestamp: "2026-03-11T00:00:00Z".to_string(),
            config_summary: ConfigSummary {
                config_source: "resolved".to_string(),
                profile_name: Some("prod".to_string()),
                config_path: Some("/tmp/config.toml".into()),
                base_url: "https://splunk.example.com:8089".to_string(),
                auth_strategy: "api_token".to_string(),
                skip_verify: true,
                timeout_secs: 30,
                max_retries: 3,
            },
            checks: vec![DiagnosticCheck {
                name: "config".to_string(),
                status: CheckStatus::Pass,
                message: "ok".to_string(),
                details: None,
            }],
            health_output: Some(HealthCheckOutput {
                server_info: None,
                splunkd_health: None,
                license_usage: None,
                kvstore_status: None,
                log_parsing_health: None,
                circuit_breaker_states: None,
            }),
            partial_errors: vec![("kvstore_status".to_string(), "boom".to_string())],
        };

        let bundle = report.to_bundle_report();

        assert_eq!(bundle.checks.len(), 1);
        let serialized = serde_json::to_value(bundle).expect("bundle should serialize");
        assert!(serialized.get("health_output").is_none());
        assert!(serialized.get("partial_errors").is_none());
    }

    #[test]
    fn categorize_connection_errors_by_domain() {
        assert_eq!(
            categorize_connection_error(&ClientError::Unauthorized("denied".to_string())),
            (
                DiagnosticStatus::Pass,
                DiagnosticStatus::Fail,
                DiagnosticStatus::Pass
            )
        );
        assert_eq!(
            categorize_connection_error(&ClientError::TlsError("certificate expired".to_string())),
            (
                DiagnosticStatus::Pass,
                DiagnosticStatus::Skip,
                DiagnosticStatus::Fail
            )
        );
        assert_eq!(
            categorize_connection_error(&ClientError::ConnectionRefused(
                "localhost:8089".to_string()
            )),
            (
                DiagnosticStatus::Fail,
                DiagnosticStatus::Skip,
                DiagnosticStatus::Skip
            )
        );
    }

    #[tokio::test]
    async fn run_connection_diagnostics_respects_cancellation() {
        let client = SplunkClient::builder()
            .base_url("https://splunk.example.com:8089".to_string())
            .auth_strategy(AuthStrategy::ApiToken {
                token: SecretString::new("test-token".to_string().into()),
            })
            .skip_verify(true)
            .build()
            .expect("client should build");
        let cancel = TestCancel(true);

        let result = run_connection_diagnostics(&client, Some(&cancel)).await;

        assert_eq!(result.overall_status, DiagnosticStatus::Skip);
        assert_eq!(result.reachable.status, DiagnosticStatus::Skip);
        assert!(
            result
                .remediation_hints
                .iter()
                .any(|hint| hint.contains("cancelled"))
        );
    }

    #[tokio::test]
    async fn run_doctor_report_collects_partial_errors_without_exposing_them_in_bundle() {
        let mock_server = MockServer::start().await;
        mount_connection_diagnostics_mocks(&mock_server, true).await;
        let config = api_token_config(mock_server.uri());

        let report = run_doctor_report(&config, "1.0.0", false, None)
            .await
            .expect("doctor report should succeed with partial errors");

        assert!(!report.partial_errors.is_empty());
        assert!(report.health_output.is_some());
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.name == "server_connectivity"
                    && check.status == CheckStatus::Pass)
        );

        let bundle = report.to_bundle_report();
        let serialized = serde_json::to_value(bundle).expect("bundle should serialize");
        assert!(serialized.get("partial_errors").is_none());
    }

    #[tokio::test]
    async fn run_connection_diagnostics_marks_partial_errors_as_failure() {
        let mock_server = MockServer::start().await;
        mount_connection_diagnostics_mocks(&mock_server, true).await;
        let client = SplunkClient::builder()
            .base_url(mock_server.uri())
            .auth_strategy(AuthStrategy::ApiToken {
                token: SecretString::new("test-token".to_string().into()),
            })
            .skip_verify(true)
            .build()
            .expect("client should build");

        let result = run_connection_diagnostics(&client, None).await;

        assert_eq!(result.overall_status, DiagnosticStatus::Fail);
        assert_eq!(result.reachable.status, DiagnosticStatus::Pass);
        assert!(
            result
                .remediation_hints
                .iter()
                .any(|hint| hint.contains("splunkd_health") || hint.contains("kvstore_status"))
        );
    }
}
