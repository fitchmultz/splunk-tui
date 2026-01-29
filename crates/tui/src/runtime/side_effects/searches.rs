//! Search-related side effect handlers.
//!
//! Responsibilities:
//! - Handle async API calls for search operations.
//! - Execute searches with progress callbacks.
//! - Load saved searches and pagination results.
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
                Some(&search_defaults.earliest_time),
                Some(&search_defaults.latest_time),
                Some(search_defaults.max_results),
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
