//! CLI cancellation utilities.
//!
//! Responsibilities:
//! - Provide a lightweight, dependency-free cancellation token that can be cloned
//!   and passed through command handlers.
//! - Define a single, recognizable `Cancelled` error used to signal user-initiated
//!   cancellation (Ctrl+C/SIGINT) through `anyhow::Result`.
//! - Centralize cancellation message and Unix-standard SIGINT exit code (130).
//!
//! Does NOT handle:
//! - This module does not install signal handlers by itself.
//! - This module does not decide *when* to check for cancellation; callers must do so.
//!
//! Invariants:
//! - Once cancelled, token remains cancelled forever.

use std::fmt;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use tokio::sync::Notify;

/// Standard Unix exit code for SIGINT: 128 + 2.
pub const SIGINT_EXIT_CODE: u8 = 130;

/// Cancellation token usable across async tasks.
///
/// This is intentionally small and dependency-free (vs `tokio_util::sync::CancellationToken`).
#[derive(Clone, Debug)]
pub struct CancellationToken {
    cancelled: Arc<AtomicBool>,
    notify: Arc<Notify>,
}

impl CancellationToken {
    /// Create a new, non-cancelled token.
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            notify: Arc::new(Notify::new()),
        }
    }

    /// Cancel token (idempotent).
    pub fn cancel(&self) {
        let was_cancelled = self.cancelled.swap(true, Ordering::SeqCst);
        if !was_cancelled {
            self.notify.notify_waiters();
        }
    }

    /// True if cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// Await cancellation.
    ///
    /// Safe against missed notifications by creating `notified()` future first,
    /// then checking atomic state.
    pub async fn cancelled(&self) {
        let notified = self.notify.notified();
        if self.is_cancelled() {
            return;
        }
        notified.await;
    }
}

/// Marker error used to indicate user-driven cancellation.
#[derive(Debug, Clone, Copy)]
pub struct Cancelled;

impl fmt::Display for Cancelled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "cancelled")
    }
}

impl std::error::Error for Cancelled {}

/// Returns true if this anyhow error represents a cancellation.
pub fn is_cancelled_error(err: &anyhow::Error) -> bool {
    err.is::<Cancelled>()
}

/// Print standard cancellation message to stderr.
pub fn print_cancelled_message() {
    eprintln!("^C\nOperation cancelled by user");
}
