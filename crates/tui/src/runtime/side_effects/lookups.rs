//! Side effects for lookup table operations.
//!
//! Responsibilities:
//! - Handle LoadLookups action to fetch lookup tables
//! - Handle LoadMoreLookups action for pagination
//!
//! Does NOT handle:
//! - UI rendering (handled by screen module)
//! - Input handling (handled by input handlers)

use tokio::sync::mpsc::Sender;

use crate::action::Action;
use crate::runtime::side_effects::SharedClient;

/// Handle loading lookup tables.
///
/// Fetches the list of lookup tables from the Splunk server.
pub async fn handle_load_lookups(
    client: SharedClient,
    tx: Sender<Action>,
    count: u64,
    offset: u64,
) {
    let result = {
        let mut guard = client.lock().await;
        guard
            .list_lookup_tables(Some(count as u32), Some(offset as u32))
            .await
    };

    let action = match result {
        Ok(lookups) => Action::LookupsLoaded(Ok(lookups)),
        Err(e) => Action::LookupsLoaded(Err(std::sync::Arc::new(e))),
    };

    let _ = tx.send(action).await;
}

/// Handle loading more lookup tables (pagination).
///
/// Fetches the next page of lookup tables from the Splunk server.
#[allow(dead_code)]
pub async fn handle_load_more_lookups(
    client: SharedClient,
    tx: Sender<Action>,
    count: u64,
    offset: u64,
) {
    let result = {
        let mut guard = client.lock().await;
        guard
            .list_lookup_tables(Some(count as u32), Some(offset as u32))
            .await
    };

    let action = match result {
        Ok(lookups) => Action::MoreLookupsLoaded(Ok(lookups)),
        Err(e) => Action::MoreLookupsLoaded(Err(std::sync::Arc::new(e))),
    };

    let _ = tx.send(action).await;
}
