//! Shared workflow modules built on top of the raw Splunk client.
//!
//! Responsibilities:
//! - Provide reusable cross-frontend workflows above endpoint-level client calls.
//! - Centralize shared data-shaping and file-output behavior.
//!
//! Does NOT handle:
//! - Binary-specific presentation concerns.
//! - Terminal UI rendering or CLI table formatting.
//!
//! Invariants:
//! - Workflow modules are frontend-neutral.

pub mod diagnostics;
pub mod export;
pub mod multi_profile;

/// Cancellation probe used by shared workflows without depending on frontend crates.
pub trait CancellationProbe: Send + Sync {
    /// Returns true when the caller has requested cancellation.
    fn is_cancelled(&self) -> bool;
}

/// Error returned when a shared workflow observes cancellation.
#[derive(Debug, thiserror::Error)]
#[error("workflow cancelled")]
pub struct WorkflowCancelled;

pub(crate) fn ensure_not_cancelled(cancel: Option<&dyn CancellationProbe>) -> anyhow::Result<()> {
    if cancel.is_some_and(CancellationProbe::is_cancelled) {
        return Err(WorkflowCancelled.into());
    }

    Ok(())
}
