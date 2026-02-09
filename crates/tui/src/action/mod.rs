//! Action protocol for async TUI event handling.
//!
//! This module defines the unified Action enum that replaces simple events.
//! Actions represent both user inputs and async API operation results.
//!
//! # Module Structure
//!
//! - `format`: Export format types (`ExportFormat`)
//! - `redaction`: Security-focused logging wrapper (`RedactedAction`)
//! - `variants`: Action enum definitions (`Action`)
//! - `tests`: Redaction and security tests
//!
//! # Security Note
//!
//! When logging Actions, use `RedactedAction(&action)` wrapper instead of
//! `?action` Debug formatting to prevent sensitive payloads from being written
//! to log files. See `RedactedAction` documentation for details.
//!
//! # What This Module Does NOT Handle
//!
//! - Action handling logic (handled by the app state machine in `App`)
//! - Async task execution (handled by the runtime module)
//! - UI rendering (handled by the ui module)
//! - Direct user input processing (handled by input handlers)

use tokio::sync::mpsc::{Sender, error::TrySendError};

pub mod format;
pub mod redaction;
pub mod variants;

pub use format::ExportFormat;
pub use redaction::RedactedAction;
pub use variants::{
    Action, InstanceOverview, InstanceStatus, LicenseData, MultiInstanceOverviewData, OverviewData,
    OverviewResource,
};

#[cfg(test)]
mod tests;

/// Creates a progress callback that bridges the client's synchronous `FnMut(f64)`
/// to the TUI's async `Sender<Action>` channel.
///
/// This allows the client's `search_with_progress` method to send progress updates
/// to the TUI event loop without blocking. Progress values are clamped to [0.0, 1.0]
/// and sent as `Action::Progress` messages.
///
/// # Arguments
///
/// * `tx` - The action sender channel to send progress updates to
///
/// # Returns
///
/// A closure that can be passed to `client.search_with_progress()` as the progress callback.
///
/// # Example
///
/// ```ignore
/// use splunk_client::SearchRequest;
///
/// let progress_tx = tx.clone();
/// let mut progress_callback = progress_callback_to_action_sender(progress_tx);
///
/// let request = SearchRequest::new(query, true)
///     .time_bounds(earliest, latest)
///     .max_results(max_results);
/// let (results, sid, total) = client
///     .search_with_progress(request, Some(&mut progress_callback))
///     .await?;
/// ```
pub fn progress_callback_to_action_sender(tx: Sender<Action>) -> impl FnMut(f64) + Send {
    move |progress: f64| {
        // Clamp progress to valid range [0.0, 1.0]
        let clamped = progress.clamp(0.0, 1.0);
        // Use try_send for synchronous callback - drop progress update if channel is full
        // This is acceptable for progress updates as they're not critical
        match tx.try_send(Action::Progress(clamped as f32)) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => {
                // Channel full - drop progress update (backpressure)
            }
            Err(TrySendError::Closed(_)) => {
                // Channel closed - nothing we can do in synchronous callback
            }
        }
    }
}
