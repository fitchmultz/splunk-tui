//! Tests for action redaction and security.
//!
//! This module contains comprehensive tests for the `RedactedAction` wrapper
//! to ensure sensitive data is never logged.

use crate::action::redaction::RedactedAction;
use crate::action::variants::Action;

/// Helper function to get redacted debug output for an action.
pub fn redacted_debug(action: &Action) -> String {
    format!("{:?}", RedactedAction(action))
}

mod clipboard_export;
mod cluster_health;
mod data_loading;
mod errors;
mod jobs;
mod profiles;
mod search;
mod simple;
