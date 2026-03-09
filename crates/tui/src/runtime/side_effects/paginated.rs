//! Purpose: Shared helpers for paginated TUI side effects.
//! Responsibilities: Build the correct initial-load vs pagination result actions from list fetches.
//! Non-scope: Does not perform network I/O, mutate app state, or spawn tasks.
//! Invariants/Assumptions: `offset == 0` maps to replace-mode actions and `offset > 0` maps to append-mode actions.

use crate::action::Action;
use std::sync::Arc;

pub(crate) fn build_paginated_action<T>(
    result: Result<Vec<T>, splunk_client::ClientError>,
    offset: usize,
    loaded: fn(Result<Vec<T>, Arc<splunk_client::ClientError>>) -> Action,
    more_loaded: fn(Result<Vec<T>, Arc<splunk_client::ClientError>>) -> Action,
) -> Action {
    let result = result.map_err(Arc::new);

    if offset == 0 {
        loaded(result)
    } else {
        more_loaded(result)
    }
}
