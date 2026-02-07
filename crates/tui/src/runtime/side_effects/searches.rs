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
use splunk_client::{SearchMode, SearchRequest};
use splunk_config::SearchDefaults;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tokio::sync::mpsc::Sender;

use super::SharedClient;

/// Helper to redact a query string for logging, showing only length and hash.
fn redact_query_for_log(query: &str) -> String {
    let mut hasher = DefaultHasher::new();
    query.hash(&mut hasher);
    let hash = hasher.finish();
    format!("<{} chars, hash={:08x}>", query.len(), hash)
}

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
    search_mode: SearchMode,
    realtime_window: Option<u64>,
) {
    tracing::debug!(
        "handle_run_search called with query: {}",
        redact_query_for_log(&query)
    );
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

    // Search defaults are sanitized at load time (ConfigManager::load),
    // so we can use them directly without runtime validation.
    let earliest_time = search_defaults.earliest_time.clone();
    let latest_time = search_defaults.latest_time.clone();
    let max_results = search_defaults.max_results;

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

        // Build the search request
        let request = SearchRequest::new(&query_clone, true)
            .time_bounds(&earliest_time, &latest_time)
            .max_results(max_results)
            .search_mode(search_mode);
        let request = if let Some(window) = realtime_window {
            request.realtime_window(window)
        } else {
            request
        };

        // Use search_with_progress for unified timeout and progress handling
        match c
            .search_with_progress(request, Some(&mut progress_callback))
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
pub async fn handle_validate_spl(
    client: SharedClient,
    tx: Sender<Action>,
    search: String,
    request_id: u64,
) {
    // Skip validation for empty or very short queries
    if search.len() < 3 {
        let _ = tx
            .send(Action::SplValidationResult {
                valid: true,
                errors: vec![],
                warnings: vec![],
                request_id,
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
                        request_id,
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
                        request_id,
                    })
                    .await;
            }
        }
    });
}

/// Handle updating a saved search.
///
/// Updates an existing saved search with the provided fields.
/// Only provided fields are updated; omitted fields retain their current values.
pub async fn handle_update_saved_search(
    client: SharedClient,
    tx: Sender<Action>,
    name: String,
    search: Option<String>,
    description: Option<String>,
    disabled: Option<bool>,
) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut c = client.lock().await;
        match c
            .update_saved_search(&name, search.as_deref(), description.as_deref(), disabled)
            .await
        {
            Ok(()) => {
                let _ = tx.send(Action::SavedSearchUpdated(Ok(()))).await;
            }
            Err(e) => {
                let _ = tx.send(Action::SavedSearchUpdated(Err(Arc::new(e)))).await;
            }
        }
    });
}

/// Handle creating a saved search.
pub async fn handle_create_saved_search(
    client: SharedClient,
    action_tx: Sender<Action>,
    name: String,
    search: String,
    description: Option<String>,
    disabled: bool,
) {
    let _ = action_tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut guard = client.lock().await;

        // First create the saved search
        match guard.create_saved_search(&name, &search).await {
            Ok(()) => {
                // If description or disabled provided, update the saved search
                if description.is_some() || disabled {
                    match guard
                        .update_saved_search(&name, None, description.as_deref(), Some(disabled))
                        .await
                    {
                        Ok(()) => {
                            let _ = action_tx.send(Action::SavedSearchCreated(Ok(()))).await;
                            // Refresh saved searches list on success
                            let _ = action_tx.send(Action::LoadSavedSearches).await;
                        }
                        Err(e) => {
                            // Saved search was created but update failed
                            let _ = action_tx
                                .send(Action::SavedSearchCreated(Err(Arc::new(e))))
                                .await;
                            // Still refresh since the saved search was created
                            let _ = action_tx.send(Action::LoadSavedSearches).await;
                        }
                    }
                } else {
                    let _ = action_tx.send(Action::SavedSearchCreated(Ok(()))).await;
                    // Refresh saved searches list on success
                    let _ = action_tx.send(Action::LoadSavedSearches).await;
                }
            }
            Err(e) => {
                let _ = action_tx
                    .send(Action::SavedSearchCreated(Err(Arc::new(e))))
                    .await;
            }
        }
    });
}

/// Handle deleting a saved search.
pub async fn handle_delete_saved_search(
    client: SharedClient,
    action_tx: Sender<Action>,
    name: String,
) {
    let _ = action_tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut guard = client.lock().await;
        match guard.delete_saved_search(&name).await {
            Ok(()) => {
                let _ = action_tx.send(Action::SavedSearchDeleted(Ok(name))).await;
                // Refresh saved searches list on success
                let _ = action_tx.send(Action::LoadSavedSearches).await;
            }
            Err(e) => {
                let _ = action_tx
                    .send(Action::SavedSearchDeleted(Err(Arc::new(e))))
                    .await;
            }
        }
    });
}

/// Handle toggling a saved search's enabled/disabled state.
pub async fn handle_toggle_saved_search(
    client: SharedClient,
    action_tx: Sender<Action>,
    name: String,
    disabled: bool,
) {
    let _ = action_tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut guard = client.lock().await;
        match guard
            .update_saved_search(&name, None, None, Some(disabled))
            .await
        {
            Ok(()) => {
                let _ = action_tx.send(Action::SavedSearchToggled(Ok(()))).await;
                // Refresh saved searches list on success
                let _ = action_tx.send(Action::LoadSavedSearches).await;
            }
            Err(e) => {
                let _ = action_tx
                    .send(Action::SavedSearchToggled(Err(Arc::new(e))))
                    .await;
            }
        }
    });
}
