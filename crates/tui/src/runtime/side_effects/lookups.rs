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
use std::sync::Arc;

/// Handle loading lookup tables with pagination support.
///
/// Emits `LookupsLoaded` when offset == 0 (initial load/refresh).
/// Emits `MoreLookupsLoaded` when offset > 0 (pagination).
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
        Ok(lookups) => {
            if offset == 0 {
                Action::LookupsLoaded(Ok(lookups))
            } else {
                Action::MoreLookupsLoaded(Ok(lookups))
            }
        }
        Err(e) => {
            let arc_err = Arc::new(e);
            if offset == 0 {
                Action::LookupsLoaded(Err(arc_err))
            } else {
                Action::MoreLookupsLoaded(Err(arc_err))
            }
        }
    };

    let _ = tx.send(action).await;
}
