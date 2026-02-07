//! Shell completion generation command.
//!
//! Responsibilities:
//! - Generate shell completion scripts for various shells (bash, zsh, fish, powershell, elvish).
//!
//! Does NOT handle:
//! - Direct installation of completions (user must redirect output to appropriate location).
//!
//! Invariants:
//! - Output is always written to stdout.

use anyhow::Result;
use clap::CommandFactory;
use clap_complete::{Shell, generate};
use std::io;

/// Generate shell completions for the specified shell.
///
/// # Arguments
/// * `shell` - The target shell for completion generation
///
/// # Returns
/// Result indicating success or failure of the operation
pub fn run(shell: Shell) -> Result<()> {
    let mut cmd = crate::args::Cli::command();
    generate(shell, &mut cmd, "splunk-cli", &mut io::stdout());
    Ok(())
}
