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
use std::io::{self, Write};

/// Generate manpage for splunk-cli.
///
/// # Returns
/// Result indicating success or failure of the operation
pub fn run() -> Result<()> {
    let cmd = crate::args::Cli::command();
    let man = clap_mangen::Man::new(cmd);

    // Render the standard manpage content
    let mut buffer: Vec<u8> = Vec::new();
    man.render(&mut buffer)?;

    // Convert to string to append custom EXIT STATUS section
    let mut manpage = String::from_utf8(buffer)?;

    // Append exit codes section
    manpage.push_str(EXIT_STATUS_SECTION);

    // Write to stdout
    io::stdout().write_all(manpage.as_bytes())?;

    Ok(())
}

/// Exit status section for the manpage.
const EXIT_STATUS_SECTION: &str = r".SH EXIT STATUS
.TP
\fB0\fR
Success - command completed successfully.
.TP
\fB1\fR
General error - unhandled or generic failure.
.TP
\fB2\fR
Authentication failure - invalid credentials or expired session.
Scripts should refresh credentials or prompt for re-authentication.
.TP
\fB3\fR
Connection error - network, timeout, or DNS failure.
Scripts may retry with exponential backoff.
.TP
\fB4\fR
Resource not found - job, index, saved search, etc.
Scripts should verify resource identifiers.
.TP
\fB5\fR
Validation error - invalid SPL, bad parameters.
Scripts should fix the input and not retry the same request.
.TP
\fB6\fR
Permission denied - insufficient privileges.
Scripts should escalate permissions or use different credentials.
.TP
\fB7\fR
Rate limited - HTTP 429 Too Many Requests.
Scripts should back off and retry later.
.TP
\fB8\fR
Service unavailable - HTTP 503, maintenance mode.
Scripts should back off and retry later.
.TP
\fB130\fR
Interrupted - SIGINT/Ctrl+C (Unix standard: 128 + 2).
";
