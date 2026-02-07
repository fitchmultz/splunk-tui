//! Doctor command for comprehensive diagnostics.
//!
//! Responsibilities:
//! - Validate configuration loading and display config sources
//! - Detect authentication strategy (token vs session)
//! - Test connectivity to Splunk server
//! - Run health aggregate checks with partial error reporting
//! - Generate redacted support bundles
//!
//! Does NOT handle:
//! - Direct REST API implementation (handled by client crate)
//! - Output formatting details (see formatters module)

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::PathBuf;
use tracing::info;

use crate::formatters::{OutputFormat, get_formatter, write_to_file};

/// Result of a single diagnostic check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticCheck {
    pub name: String,
    pub status: CheckStatus,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

/// Status of a diagnostic check.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Pass,
    Fail,
    Warning,
    Skipped,
}

/// Complete diagnostic report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticReport {
    pub cli_version: String,
    pub os_arch: String,
    pub timestamp: String,
    pub config_summary: ConfigSummary,
    pub checks: Vec<DiagnosticCheck>,
    /// Health output - contains server information that may be sensitive.
    /// This field is excluded from support bundles for security.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_output: Option<splunk_client::HealthCheckOutput>,
    /// Partial errors from health checks - may contain sensitive info.
    /// This field is excluded from support bundles for security.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub partial_errors: Vec<(String, String)>,
}

impl DiagnosticReport {
    /// Creates a redacted version of the report safe for support bundles.
    /// Excludes potentially sensitive fields like health_output and partial_errors.
    fn to_bundle_report(&self) -> BundleDiagnosticReport {
        BundleDiagnosticReport {
            cli_version: self.cli_version.clone(),
            os_arch: self.os_arch.clone(),
            timestamp: self.timestamp.clone(),
            config_summary: self.config_summary.clone(),
            checks: self.checks.clone(),
        }
    }
}

/// Redacted diagnostic report for support bundles.
/// Excludes fields that may contain sensitive server information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleDiagnosticReport {
    pub cli_version: String,
    pub os_arch: String,
    pub timestamp: String,
    pub config_summary: ConfigSummary,
    pub checks: Vec<DiagnosticCheck>,
}

/// Redacted configuration summary for support bundles.
///
/// All sensitive values are excluded - only configuration metadata is included.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

/// Run the doctor diagnostic command.
///
/// # Arguments
/// * `config` - The loaded Splunk configuration
/// * `bundle_path` - Optional path to write a support bundle
/// * `include_logs` - Whether to include TUI logs in the bundle
/// * `output_format` - Output format (json, table, etc.)
/// * `output_file` - Optional file to write output to
/// * `cancel` - Cancellation token for graceful shutdown
///
/// # Returns
/// Result indicating success or failure. Returns error if required checks fail.
pub async fn run(
    config: splunk_config::Config,
    bundle_path: Option<PathBuf>,
    include_logs: bool,
    output_format: &str,
    output_file: Option<PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Running doctor diagnostics...");

    let mut checks = Vec::new();

    // Check 1: Config validation
    let config_check = run_config_check(&config)?;
    checks.push(config_check);

    // Check 2: Auth strategy detection
    let auth_check = run_auth_check(&config)?;
    checks.push(auth_check);

    // Build client and run connectivity/health checks
    let client_result = crate::commands::build_client_from_config(&config);
    let client = match client_result {
        Ok(c) => {
            checks.push(DiagnosticCheck {
                name: "client_build".to_string(),
                status: CheckStatus::Pass,
                message: "Successfully built Splunk client".to_string(),
                details: None,
            });
            Some(c)
        }
        Err(e) => {
            checks.push(DiagnosticCheck {
                name: "client_build".to_string(),
                status: CheckStatus::Fail,
                message: format!("Failed to build client: {}", e),
                details: None,
            });
            None
        }
    };

    // Check 3: Server connectivity and health aggregate
    let (health_output, partial_errors): (
        Option<splunk_client::HealthCheckOutput>,
        Vec<(String, String)>,
    ) = if let Some(client) = client {
        let health_result = tokio::select! {
            res = client.check_health_aggregate() => res,
            _ = cancel.cancelled() => return Err(crate::cancellation::Cancelled.into()),
        };
        match health_result {
            Ok(health) => {
                let server_name = health
                    .output
                    .server_info
                    .as_ref()
                    .map(|s| s.server_name.clone())
                    .unwrap_or_default();
                let version = health
                    .output
                    .server_info
                    .as_ref()
                    .map(|s| s.version.clone())
                    .unwrap_or_default();

                checks.push(DiagnosticCheck {
                    name: "server_connectivity".to_string(),
                    status: CheckStatus::Pass,
                    message: format!("Connected to {} ({})", client.base_url(), server_name),
                    details: Some(serde_json::json!({
                        "server_name": server_name,
                        "version": version,
                    })),
                });

                // Check 4: License status
                if health.output.license_usage.is_some() {
                    checks.push(DiagnosticCheck {
                        name: "license_status".to_string(),
                        status: CheckStatus::Pass,
                        message: "License information retrieved".to_string(),
                        details: None,
                    });
                }

                // Check 5: KVStore status
                if let Some(ref kvstore) = health.output.kvstore_status {
                    let status = if kvstore.current_member.status == "ready" {
                        CheckStatus::Pass
                    } else {
                        CheckStatus::Warning
                    };
                    checks.push(DiagnosticCheck {
                        name: "kvstore_status".to_string(),
                        status,
                        message: format!("KVStore status: {}", kvstore.current_member.status),
                        details: None,
                    });
                }

                let partial_errors: Vec<(String, String)> = health
                    .partial_errors
                    .into_iter()
                    .map(|(name, err): (String, splunk_client::ClientError)| {
                        (name, err.to_string())
                    })
                    .collect();
                (Some(health.output), partial_errors)
            }
            Err(e) => {
                checks.push(DiagnosticCheck {
                    name: "server_connectivity".to_string(),
                    status: CheckStatus::Fail,
                    message: format!("Failed to connect: {}", e),
                    details: None,
                });
                (None, Vec::new())
            }
        }
    } else {
        (None, Vec::new())
    };

    // Build the diagnostic report
    let config_summary = build_config_summary(&config)?;

    let report = DiagnosticReport {
        cli_version: env!("CARGO_PKG_VERSION").to_string(),
        os_arch: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
        timestamp: chrono::Utc::now().to_rfc3339(),
        config_summary,
        checks,
        health_output,
        partial_errors,
    };

    // Handle bundle generation if requested
    if let Some(bundle_path) = bundle_path {
        generate_bundle(&report, &bundle_path, include_logs).await?;
        eprintln!("Support bundle written to: {}", bundle_path.display());
    }

    // Format and output results
    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);
    let output = formatter.format_health_check_report(&report)?;

    if let Some(ref path) = output_file {
        write_to_file(&output, path)
            .with_context(|| format!("Failed to write output to {}", path.display()))?;
        eprintln!(
            "Results written to {} ({:?} format)",
            path.display(),
            format
        );
    } else {
        print!("{}", output);
    }

    // Return error if any checks failed
    let has_failures = report
        .checks
        .iter()
        .any(|c| matches!(c.status, CheckStatus::Fail));

    if has_failures {
        return Err(anyhow::anyhow!("Diagnostic checks failed"));
    }

    Ok(())
}

fn run_config_check(config: &splunk_config::Config) -> Result<DiagnosticCheck> {
    Ok(DiagnosticCheck {
        name: "config_load".to_string(),
        status: CheckStatus::Pass,
        message: "Configuration loaded successfully".to_string(),
        details: Some(serde_json::json!({
            "base_url": config.connection.base_url,
            "timeout_secs": config.connection.timeout.as_secs(),
            "max_retries": config.connection.max_retries,
        })),
    })
}

fn run_auth_check(config: &splunk_config::Config) -> Result<DiagnosticCheck> {
    use splunk_config::AuthStrategy;

    let (strategy, message): (&str, String) = match &config.auth.strategy {
        AuthStrategy::ApiToken { .. } => {
            ("api_token", "Using API token authentication".to_string())
        }
        AuthStrategy::SessionToken { username, .. } => (
            "session_token",
            format!("Using session token authentication (user: {})", username),
        ),
    };

    Ok(DiagnosticCheck {
        name: "auth_strategy".to_string(),
        status: CheckStatus::Pass,
        message,
        details: Some(serde_json::json!({"strategy": strategy})),
    })
}

fn build_config_summary(config: &splunk_config::Config) -> Result<ConfigSummary> {
    use splunk_config::AuthStrategy;

    let auth_strategy = match &config.auth.strategy {
        AuthStrategy::ApiToken { .. } => "api_token",
        AuthStrategy::SessionToken { .. } => "session_token",
    };

    Ok(ConfigSummary {
        config_source: "resolved".to_string(),
        profile_name: None,
        config_path: None,
        base_url: config.connection.base_url.clone(),
        auth_strategy: auth_strategy.to_string(),
        skip_verify: config.connection.skip_verify,
        timeout_secs: config.connection.timeout.as_secs(),
        max_retries: config.connection.max_retries,
    })
}

async fn generate_bundle(
    report: &DiagnosticReport,
    bundle_path: &PathBuf,
    include_logs: bool,
) -> Result<()> {
    use std::fs::File;
    use zip::write::SimpleFileOptions;

    // Create parent directories if they don't exist
    if let Some(parent) = bundle_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file = File::create(bundle_path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options: SimpleFileOptions =
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // Add diagnostic report (using redacted version for bundle)
    zip.start_file("diagnostic_report.json", options)?;
    let bundle_report = report.to_bundle_report();
    zip.write_all(serde_json::to_string_pretty(&bundle_report)?.as_bytes())?;

    // Add redacted environment info
    let env_info = collect_redacted_env_info();
    zip.start_file("environment.txt", options)?;
    zip.write_all(env_info.as_bytes())?;

    // Optionally include logs
    if include_logs {
        match collect_tui_logs().await {
            Ok(logs) => {
                zip.start_file("splunk-tui-logs.log", options)?;
                zip.write_all(logs.as_bytes())?;
            }
            Err(e) => {
                eprintln!("Warning: Failed to collect TUI logs: {}", e);
            }
        }
    }

    zip.finish()?;
    Ok(())
}

fn collect_redacted_env_info() -> String {
    // Collect environment variable names only (values redacted for security)
    let mut output = String::from("Environment Variables (names only, values redacted):\n");
    let mut splunk_vars: Vec<String> = std::env::vars()
        .filter(|(key, _)| key.starts_with("SPLUNK_"))
        .map(|(key, _)| key)
        .collect();
    splunk_vars.sort();

    for key in splunk_vars {
        output.push_str(&format!("{}=***REDACTED***\n", key));
    }

    // Add system info (non-sensitive)
    output.push_str("\nSystem Information:\n");
    output.push_str(&format!("OS: {}\n", std::env::consts::OS));
    output.push_str(&format!("Arch: {}\n", std::env::consts::ARCH));
    output.push_str(&format!("Family: {}\n", std::env::consts::FAMILY));

    output
}

async fn collect_tui_logs() -> Result<String> {
    // Try to find and read recent splunk-tui logs from standard locations
    // Note: Using standard directories without dirs crate to reduce dependencies
    let home_dir = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE"));
    let log_dirs: Vec<std::path::PathBuf> = if let Ok(home) = home_dir {
        vec![
            std::path::PathBuf::from(&home).join(".local/share/splunk-tui/logs"),
            std::path::PathBuf::from(&home).join("AppData/Local/splunk-tui/logs"),
        ]
    } else {
        vec![]
    };

    for log_dir in log_dirs {
        if log_dir.exists() {
            // Read most recent log file
            let mut entries: Vec<_> = std::fs::read_dir(&log_dir)?.flatten().collect();
            entries.sort_by_key(|e| {
                e.metadata()
                    .and_then(|m| m.modified())
                    .ok()
                    .unwrap_or(std::time::UNIX_EPOCH)
            });

            if let Some(latest) = entries.last() {
                let path = latest.path();
                let metadata = std::fs::metadata(&path)?;
                let file_size = metadata.len();

                // Limit log file size to 10MB to prevent memory exhaustion
                const MAX_LOG_SIZE: u64 = 10 * 1024 * 1024;
                if file_size > MAX_LOG_SIZE {
                    return Ok(format!(
                        "Log file too large ({} bytes). Skipping.",
                        file_size
                    ));
                }

                let content = std::fs::read_to_string(&path)?;
                // Truncate to last 1000 lines to keep bundle size reasonable
                let lines: Vec<_> = content.lines().rev().take(1000).collect();
                return Ok(lines.into_iter().rev().collect::<Vec<_>>().join("\n"));
            }
        }
    }

    Ok("No splunk-tui logs found".to_string())
}
