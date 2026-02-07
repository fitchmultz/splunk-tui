//! Manpage generation command.
//!
//! Responsibilities:
//! - Generate manpage output for splunk-cli.
//!
//! Does NOT handle:
//! - Direct installation of manpages (user must redirect output to appropriate location).
//!
//! Invariants:
//! - Output is always written to stdout.

use anyhow::Result;
use clap::CommandFactory;
use std::io;

/// Generate manpage for splunk-cli.
///
/// # Returns
/// Result indicating success or failure of the operation
pub fn run() -> Result<()> {
    let cmd = crate::args::Cli::command();
    let man = clap_mangen::Man::new(cmd);
    man.render(&mut io::stdout())?;
    Ok(())
}
