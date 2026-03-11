//! Purpose: Public error entry point for shared client error kinds, classification, and user-facing rendering.
//! Responsibilities: Re-export stable client error types, keep classification logic split by concern, and host error-focused tests.
//! Scope: Shared client error semantics only; CLI exit-code policy lives in `crates/cli`.
//! Usage: Import `splunk_client::error::{ClientError, Result, UserFacingFailure}` from this module.
//! Invariants/Assumptions: The public `splunk_client::error` API remains stable even as internal files are reorganized.

mod classify;
mod kinds;
mod user_facing;

pub use kinds::{
    ClientError, FailureCategory, HttpErrorSnapshot, Result, RollbackFailure, UserFacingFailure,
};

#[cfg(test)]
mod tests;
