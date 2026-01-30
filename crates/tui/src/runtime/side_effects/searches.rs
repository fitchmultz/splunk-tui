//! Search-related side effect handlers.
//!
//! Responsibilities:
//! - Handle async API calls for search operations.
//! - Execute searches with progress callbacks.
//! - Load saved searches and pagination results.
//! - SPL syntax validation.
//!
//! Does NOT handle:
//! - Direct state modification (sends actions for that).
//! - UI rendering.

use crate::action::{Action, progress_callback_to_action_sender};
use crate::error_details::{build_search_error_details, search_error_message};
use splunk_config::SearchDefaults;
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

use super::SharedClient;

/// Handle loading saved searches.
pub async fn handle_load_saved_searches(client: SharedClient, tx: Sender<Action>) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut c = client.lock().await;
        match c.list_saved_searches(None, None).await {
            Ok(searches) => {
                let _ = tx.send(Action::SavedSearchesLoaded(Ok(searches))).await;
            }
            Err(e) => {
                let _ = tx.send(Action::SavedSearchesLoaded(Err(Arc::new(e)))).await;
            }
        }
    });
}

/// Handle running a search.
pub async fn handle_run_search(
    client: SharedClient,
    tx: Sender<Action>,
    query: String,
    search_defaults: SearchDefaults,
) {
    tracing::debug!("handle_run_search called with query: {}", query);
    tracing::debug!(
        "search_defaults.earliest_time: {}",
        search_defaults.earliest_time
    );
    tracing::debug!(
        "search_defaults.latest_time: {}",
        search_defaults.latest_time
    );
    tracing::debug!(
        "search_defaults.max_results: {}",
        search_defaults.max_results
    );

    // Validate search defaults to prevent 400 errors from Splunk
    let earliest_time = if search_defaults.earliest_time.trim().is_empty() {
        tracing::warn!("search_defaults.earliest_time is empty, using default '-24h'");
        "-24h".to_string()
    } else {
        search_defaults.earliest_time.clone()
    };
    let latest_time = if search_defaults.latest_time.trim().is_empty() {
        tracing::warn!("search_defaults.latest_time is empty, using default 'now'");
        "now".to_string()
    } else {
        search_defaults.latest_time.clone()
    };
    let max_results = if search_defaults.max_results == 0 {
        tracing::warn!("search_defaults.max_results is 0, using default 1000");
        1000
    } else {
        search_defaults.max_results
    };

    let _ = tx.send(Action::Loading(true)).await;
    let _ = tx.send(Action::Progress(0.1)).await;

    // Store the query that is about to run for accurate status messages
    let _ = tx.send(Action::SearchStarted(query.clone())).await;

    let tx_clone = tx.clone();
    let query_clone = query.clone();
    tokio::spawn(async move {
        let mut c = client.lock().await;

        // Create progress callback that sends Action::Progress via channel
        let progress_tx = tx_clone.clone();
        let mut progress_callback = progress_callback_to_action_sender(progress_tx);

        // Use search_with_progress for unified timeout and progress handling
        match c
            .search_with_progress(
                &query_clone,
                true, // wait for completion
                Some(&earliest_time),
                Some(&latest_time),
                Some(max_results),
                Some(&mut progress_callback),
            )
            .await
        {
            Ok((results, sid, total)) => {
                let _ = tx_clone.send(Action::Progress(1.0)).await;
                let _ = tx_clone
                    .send(Action::SearchComplete(Ok((results, sid, total))))
                    .await;
            }
            Err(e) => {
                let details = build_search_error_details(
                    &e,
                    query_clone,
                    "search_with_progress".to_string(),
                    None, // SID not available on failure
                );
                let error_msg = search_error_message(&e);
                // Error details stored in SearchComplete handler; user can press 'e' to view
                let _ = tx_clone
                    .send(Action::SearchComplete(Err((error_msg, details))))
                    .await;
            }
        }
    });
}

/// Handle loading more search results (pagination).
pub async fn handle_load_more_search_results(
    client: SharedClient,
    tx: Sender<Action>,
    sid: String,
    offset: u64,
    count: u64,
) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut c = client.lock().await;
        match c.get_search_results(&sid, count, offset).await {
            Ok(results) => {
                let _ = tx
                    .send(Action::MoreSearchResultsLoaded(Ok((
                        results.results,
                        offset,
                        results.total,
                    ))))
                    .await;
            }
            Err(e) => {
                let _ = tx
                    .send(Action::MoreSearchResultsLoaded(Err(Arc::new(e))))
                    .await;
            }
        }
    });
}

/// Handle SPL validation request (debounced).
///
/// Validates SPL syntax without executing the search. Short queries (< 3 chars)
/// are considered valid to reduce API load. Errors are logged but don't fail
/// the UI - validation is best-effort.
pub async fn handle_validate_spl(client: SharedClient, tx: Sender<Action>, search: String) {
    // Skip validation for empty or very short queries
    if search.len() < 3 {
        let _ = tx
            .send(Action::SplValidationResult {
                valid: true,
                errors: vec![],
                warnings: vec![],
            })
            .await;
        return;
    }

    tokio::spawn(async move {
        let mut c = client.lock().await;

        match c.validate_spl(&search).await {
            Ok(result) => {
                let _ = tx
                    .send(Action::SplValidationResult {
                        valid: result.valid,
                        errors: result.errors.into_iter().map(|e| e.message).collect(),
                        warnings: result.warnings.into_iter().map(|w| w.message).collect(),
                    })
                    .await;
            }
            Err(e) => {
                // Log error but don't fail - validation is best-effort
                tracing::debug!("SPL validation failed: {}", e);
                let _ = tx
                    .send(Action::SplValidationResult {
                        valid: true, // Assume valid on error
                        errors: vec![],
                        warnings: vec!["Validation unavailable".to_string()],
                    })
                    .await;
            }
        }
    });
}
