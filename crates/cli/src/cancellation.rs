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

/// Macro for running an async operation with cancellation support.
///
/// This macro wraps the common `tokio::select!` pattern that combines
/// an API call with a cancellation token check.
///
/// # Pattern 1: Simple Assignment (returns the result)
/// ```ignore
/// let usage = cancellable!(client.get_license_usage(), cancel)?;
/// ```
///
/// # Pattern 2: Execute without storing (trailing ?)
/// ```ignore
/// cancellable!(client.delete_license_pool(name), cancel)?;
/// ```
#[macro_export]
macro_rules! cancellable {
    ($future:expr, $token:expr) => {{
        let __res: anyhow::Result<_> = tokio::select! {
            res = $future => res.map_err(|e| -> anyhow::Error { e.into() }),
            _ = $token.cancelled() => Err($crate::cancellation::Cancelled.into()),
        };
        __res
    }};
}

/// Macro for running an async operation with a custom handler on success.
///
/// This is used when you need to process the result and perform side effects
/// (like printing a success message) before returning.
///
/// # Example
/// ```ignore
/// cancellable_with!(client.create_role(&params), cancel, |role| {
///     println!("Role '{}' created successfully.", role.name);
///     Ok(())
/// })?;
/// ```
#[macro_export]
macro_rules! cancellable_with {
    ($future:expr, $token:expr, |$res:ident| $handler:expr) => {{
        let __res: anyhow::Result<_> = tokio::select! {
            res = $future => {
                match res {
                    Ok($res) => $handler,
                    Err(e) => Err(e.into()),
                }
            }
            _ = $token.cancelled() => Err($crate::cancellation::Cancelled.into()),
        };
        __res
    }};
}

// Macros are exported at crate root via #[macro_use] in main.rs
// Use `use crate::{cancellable, cancellable_with};` or the full path to import them
