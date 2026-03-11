//! Doctor command wired to the shared diagnostics workflow.
//!
//! Purpose:
//! - Execute comprehensive diagnostics from the CLI while delegating shared logic to `splunk-client`.
//!
//! Responsibilities:
//! - Run the shared doctor workflow.
//! - Format and emit doctor output.
//! - Generate redacted support bundles from the shared report.
//!
//! Scope:
//! - CLI orchestration only; diagnostics models and probe logic live in `splunk-client`.
//!
//! Usage:
//! - Routed from `dispatch.rs` for `splunk-cli doctor`.
//!
//! Invariants/Assumptions:
//! - Support bundles always use the redacted shared report view.

use anyhow::{Context, Result};
use std::io::Write;
use std::path::PathBuf;
use tracing::info;

use crate::formatters::{OutputFormat, get_formatter, output_result};

pub type CheckStatus = splunk_client::workflows::diagnostics::CheckStatus;
#[allow(dead_code)]
pub type ConfigSummary = splunk_client::workflows::diagnostics::ConfigSummary;
pub type DiagnosticReport = splunk_client::workflows::diagnostics::DiagnosticReport;

/// Run the doctor diagnostic command.
pub async fn run(
    config: splunk_config::Config,
    bundle_path: Option<PathBuf>,
    include_logs: bool,
    output_format: &str,
    output_file: Option<PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
    no_cache: bool,
) -> Result<()> {
    info!("Running doctor diagnostics...");

    if cancel.is_cancelled() {
        anyhow::bail!("Diagnostic checks cancelled");
    }

    let report = splunk_client::workflows::diagnostics::run_doctor_report(
        &config,
        env!("CARGO_PKG_VERSION"),
        no_cache,
        Some(cancel),
    )
    .await?;

    if let Some(bundle_path) = bundle_path {
        generate_bundle(&report, &bundle_path, include_logs)
            .await
            .with_context(|| {
                format!(
                    "Failed to generate support bundle at {}",
                    bundle_path.display()
                )
            })?;
        eprintln!("Support bundle written to: {}", bundle_path.display());
    }

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);
    let output = formatter.format_health_check_report(&report)?;
    output_result(&output, format, output_file.as_ref())?;

    if report
        .checks
        .iter()
        .any(|check| matches!(check.status, CheckStatus::Fail))
    {
        anyhow::bail!("Diagnostic checks failed");
    }

    Ok(())
}

fn write_zip_entry<W: std::io::Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    entry_name: &str,
    options: zip::write::SimpleFileOptions,
    contents: &[u8],
) -> Result<()> {
    zip.start_file(entry_name, options)
        .with_context(|| format!("Failed to start zip entry {entry_name}"))?;
    zip.write_all(contents)
        .with_context(|| format!("Failed to write zip entry {entry_name}"))?;
    Ok(())
}

async fn generate_bundle(
    report: &DiagnosticReport,
    bundle_path: &PathBuf,
    include_logs: bool,
) -> Result<()> {
    use std::fs::File;
    use zip::write::SimpleFileOptions;

    if let Some(parent) = bundle_path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create support bundle directory at {}",
                parent.display()
            )
        })?;
    }

    let file = File::create(bundle_path).with_context(|| {
        format!(
            "Failed to create support bundle file at {}",
            bundle_path.display()
        )
    })?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let bundle_report = report.to_bundle_report();
    let report_json = serde_json::to_string_pretty(&bundle_report)
        .context("Failed to serialize diagnostic report for support bundle")?;
    write_zip_entry(
        &mut zip,
        "diagnostic_report.json",
        options,
        report_json.as_bytes(),
    )?;

    let env_info = collect_redacted_env_info();
    write_zip_entry(&mut zip, "environment.txt", options, env_info.as_bytes())?;

    if include_logs {
        match collect_tui_logs().await {
            Ok(logs) => {
                write_zip_entry(&mut zip, "splunk-tui-logs.log", options, logs.as_bytes())?;
            }
            Err(error) => {
                eprintln!("Warning: Failed to collect TUI logs: {error}");
            }
        }
    }

    zip.finish().with_context(|| {
        format!(
            "Failed to finalize support bundle archive at {}",
            bundle_path.display()
        )
    })?;

    Ok(())
}

fn collect_redacted_env_info() -> String {
    let mut output = String::from("Environment Variables (names only, values redacted):\n");
    let mut splunk_vars: Vec<String> = std::env::vars()
        .filter(|(key, _)| key.starts_with("SPLUNK_"))
        .map(|(key, _)| key)
        .collect();
    splunk_vars.sort();

    for key in splunk_vars {
        output.push_str(&format!("{key}=***REDACTED***\n"));
    }

    output.push_str("\nSystem Information:\n");
    output.push_str(&format!("OS: {}\n", std::env::consts::OS));
    output.push_str(&format!("Arch: {}\n", std::env::consts::ARCH));
    output.push_str(&format!("Family: {}\n", std::env::consts::FAMILY));
    output
}

async fn collect_tui_logs() -> Result<String> {
    let home_dir = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE"));
    let log_dirs: Vec<std::path::PathBuf> = if let Ok(home) = home_dir {
        vec![
            std::path::PathBuf::from(&home).join(".local/share/splunk-tui/logs"),
            std::path::PathBuf::from(&home).join("AppData/Local/splunk-tui/logs"),
        ]
    } else {
        Vec::new()
    };

    for log_dir in log_dirs {
        if !log_dir.exists() {
            continue;
        }

        let mut entries: Vec<_> = std::fs::read_dir(&log_dir)
            .with_context(|| format!("Failed to read TUI log directory at {}", log_dir.display()))?
            .flatten()
            .collect();
        entries.sort_by_key(|entry| {
            entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .ok()
                .unwrap_or(std::time::UNIX_EPOCH)
        });

        if let Some(latest) = entries.last() {
            let path = latest.path();
            let metadata = std::fs::metadata(&path).with_context(|| {
                format!("Failed to read TUI log metadata at {}", path.display())
            })?;
            const MAX_LOG_SIZE: u64 = 10 * 1024 * 1024;
            if metadata.len() > MAX_LOG_SIZE {
                return Ok(format!(
                    "Log file too large ({} bytes). Skipping.",
                    metadata.len()
                ));
            }

            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("Failed to read TUI log file at {}", path.display()))?;
            let lines: Vec<_> = content.lines().rev().take(1000).collect();
            return Ok(lines.into_iter().rev().collect::<Vec<_>>().join("\n"));
        }
    }

    Ok("No splunk-tui logs found".to_string())
}
