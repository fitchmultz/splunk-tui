//! Progress indicator utilities for the Splunk CLI.
//!
//! Responsibilities:
//! - Provide reusable progress indicators (search progress bar + generic spinners).
//! - Ensure ALL progress output is written to STDERR (never stdout), so machine-readable
//!   command output (json/table/csv/xml) is not contaminated.
//! - Allow global suppression via a caller-provided `enabled` boolean (driven by `--quiet`).
//!
//! Non-responsibilities:
//! - This module does not decide *when* progress should be shown; callers do.
//! - This module does not print command results; stdout remains reserved for results.

use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use std::time::Duration;

/// A percent-based progress bar for search jobs (0–100%).
///
/// Uses a spinner + percentage display, and always draws to STDERR.
/// When disabled, it becomes a no-op.
pub(crate) struct SearchProgress {
    label: String,
    pb: Option<ProgressBar>,
}

impl SearchProgress {
    /// Create a new search progress indicator.
    ///
    /// `enabled` should be `!quiet`.
    pub(crate) fn new(enabled: bool, label: impl Into<String>) -> Self {
        let label = label.into();

        if !enabled {
            return Self { label, pb: None };
        }

        let pb = ProgressBar::new(100);
        pb.set_draw_target(ProgressDrawTarget::stderr());
        pb.set_style(
            ProgressStyle::with_template("{spinner} {msg} [{bar:40.cyan/blue}] {pos:>3}%")
                .expect("template is a compile-time constant with valid syntax")
                .progress_chars("=>-"),
        );
        pb.set_message(label.clone());
        pb.enable_steady_tick(Duration::from_millis(100));

        Self {
            label,
            pb: Some(pb),
        }
    }

    /// Update progress from Splunk's `done_progress` fraction (0.0–1.0).
    pub(crate) fn set_fraction(&self, done_progress: f64) {
        let Some(pb) = &self.pb else {
            return;
        };

        let clamped = done_progress.clamp(0.0, 1.0);
        let percent = (clamped * 100.0).round() as u64;
        pb.set_position(percent);
    }

    /// Finish the progress indicator with a stable message (on STDERR).
    pub(crate) fn finish(&self) {
        let Some(pb) = &self.pb else {
            return;
        };

        pb.set_position(100);
        pb.finish_with_message(format!("{} done", self.label));
    }
}

impl Drop for SearchProgress {
    fn drop(&mut self) {
        // If an error occurs and we didn't explicitly finish, clear the progress line
        // to avoid messy interleaving with error output.
        if let Some(pb) = &self.pb
            && !pb.is_finished()
        {
            pb.finish_and_clear();
        }
    }
}

/// An indefinite spinner for short/unknown-duration operations (cancel/delete).
///
/// Always draws to STDERR; no-op when disabled.
pub(crate) struct Spinner {
    label: String,
    pb: Option<ProgressBar>,
}

impl Spinner {
    /// Create a new spinner.
    ///
    /// `enabled` should be `!quiet`.
    pub(crate) fn new(enabled: bool, label: impl Into<String>) -> Self {
        let label = label.into();

        if !enabled {
            return Self { label, pb: None };
        }

        let pb = ProgressBar::new_spinner();
        pb.set_draw_target(ProgressDrawTarget::stderr());
        pb.set_style(
            ProgressStyle::with_template("{spinner} {msg}")
                .expect("template is a compile-time constant with valid syntax"),
        );
        pb.set_message(label.clone());
        pb.enable_steady_tick(Duration::from_millis(100));

        Self {
            label,
            pb: Some(pb),
        }
    }

    /// Finish the spinner with a stable message (on STDERR).
    pub(crate) fn finish(&self) {
        let Some(pb) = &self.pb else {
            return;
        };

        pb.finish_with_message(format!("{} done", self.label));
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        if let Some(pb) = &self.pb
            && !pb.is_finished()
        {
            pb.finish_and_clear();
        }
    }
}
