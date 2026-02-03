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
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut guard = client.lock().await;
        match guard
            .list_lookup_tables(Some(count as u32), Some(offset as u32))
            .await
        {
            Ok(lookups) => {
                if offset == 0 {
                    let _ = tx.send(Action::LookupsLoaded(Ok(lookups))).await;
                } else {
                    let _ = tx.send(Action::MoreLookupsLoaded(Ok(lookups))).await;
                }
            }
            Err(e) => {
                let arc_err = Arc::new(e);
                if offset == 0 {
                    let _ = tx.send(Action::LookupsLoaded(Err(arc_err))).await;
                } else {
                    let _ = tx.send(Action::MoreLookupsLoaded(Err(arc_err))).await;
                }
            }
        }
    });
}
