//! Command-line argument parsing for splunk-tui.
//!
//! Responsibilities:
//! - Define CLI argument structure using clap derive macros.
//! - Provide parsed CLI arguments to the main application.
//!
//! Does NOT handle:
//! - Configuration loading or validation (see `runtime::config`).
//! - Terminal state management (see `runtime::terminal`).
//! - Environment variable parsing (handled by `splunk_config`).
//!
//! Invariants:
//! - CLI arguments are parsed once at startup via `Cli::parse()`.
//! - All path arguments are resolved relative to the current working directory.

use clap::Parser;
use std::path::PathBuf;

/// Command-line arguments for splunk-tui.
///
/// Configuration precedence (highest to lowest):
/// 1. CLI arguments (e.g., --profile, --config-path)
/// 2. Environment variables (e.g., SPLUNK_PROFILE, SPLUNK_BASE_URL)
/// 3. Profile configuration (from config.json)
/// 4. Default values
#[derive(Debug, Parser)]
#[command(
    name = "splunk-tui",
    about = "Terminal user interface for Splunk Enterprise",
    version,
    after_help = "Examples:\n  splunk-tui\n  splunk-tui --profile production\n  splunk-tui --config-path /etc/splunk-tui/config.json\n  splunk-tui --log-dir /var/log/splunk-tui --no-mouse\n"
)]
pub struct Cli {
    /// Config profile name to load
    #[arg(long, short = 'p')]
    pub profile: Option<String>,

    /// Path to a custom configuration file
    #[arg(long)]
    pub config_path: Option<PathBuf>,

    /// Directory for log files
    #[arg(long, default_value = "logs")]
    pub log_dir: PathBuf,

    /// Disable mouse support
    #[arg(long)]
    pub no_mouse: bool,
}
