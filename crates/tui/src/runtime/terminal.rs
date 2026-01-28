//! Terminal state management and cleanup.
//!
//! Responsibilities:
//! - Ensure terminal state is restored on application exit, even during panics.
//! - Manage raw mode and alternate screen cleanup via Drop trait.
//!
//! Does NOT handle:
//! - Initial terminal setup (done in `main.rs`).
//! - Mouse capture configuration beyond tracking the flag.
//!
//! Invariants / Assumptions:
//! - Must be created after terminal setup is complete.
//! - Must live for the duration of the TUI session.
//! - Drop implementation must not panic.

use crossterm::{
    event::DisableMouseCapture,
    execute,
    terminal::{LeaveAlternateScreen, disable_raw_mode},
};

/// Guard that ensures terminal state is restored on drop.
///
/// This struct captures the terminal state configuration and restores
/// it when dropped, ensuring cleanup happens even during panics.
///
/// # Invariants
/// - Must be created after terminal setup is complete
/// - Must live for the duration of the TUI session
/// - Drop implementation must not panic
pub struct TerminalGuard {
    no_mouse: bool,
}

impl TerminalGuard {
    /// Create a new terminal guard.
    ///
    /// # Arguments
    /// * `no_mouse` - Whether mouse capture was disabled during setup
    pub fn new(no_mouse: bool) -> Self {
        Self { no_mouse }
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Restore terminal state, ignoring errors since we're in drop
        // and must not panic. The explicit cleanup in main() runs first
        // on normal exit; this is a safety net for panics and signals.
        let _ = disable_raw_mode();
        let mut stdout = std::io::stdout();
        if self.no_mouse {
            let _ = execute!(stdout, LeaveAlternateScreen);
        } else {
            let _ = execute!(stdout, LeaveAlternateScreen, DisableMouseCapture);
        }
    }
}
