//! Side effects for forwarders operations.
//!
//! Responsibilities:
//! - Handle LoadForwarders action to fetch deployment clients (forwarders)
//! - Handle LoadMoreForwarders action for pagination
//!
//! Does NOT handle:
//! - UI rendering (handled by screen module)
//! - Input handling (handled by input handlers)

use tokio::sync::mpsc::Sender;

use crate::action::Action;
use crate::runtime::side_effects::SharedClient;

/// Handle loading forwarders.
///
/// Fetches the list of deployment clients (forwarders) from the Splunk server.
pub async fn handle_load_forwarders(
    client: SharedClient,
    tx: Sender<Action>,
    count: u64,
    offset: u64,
) {
    let result = {
        let mut guard = client.lock().await;
        guard.list_forwarders(Some(count), Some(offset)).await
    };

    let action = match result {
        Ok(forwarders) => Action::ForwardersLoaded(Ok(forwarders)),
        Err(e) => Action::ForwardersLoaded(Err(std::sync::Arc::new(e))),
    };

    let _ = tx.send(action).await;
}

/// Handle loading more forwarders (pagination).
///
/// Fetches the next page of forwarders from the Splunk server.
#[allow(dead_code)]
pub async fn handle_load_more_forwarders(
    client: SharedClient,
    tx: Sender<Action>,
    count: u64,
    offset: u64,
) {
    let result = {
        let mut guard = client.lock().await;
        guard.list_forwarders(Some(count), Some(offset)).await
    };

    let action = match result {
        Ok(forwarders) => Action::MoreForwardersLoaded(Ok(forwarders)),
        Err(e) => Action::MoreForwardersLoaded(Err(std::sync::Arc::new(e))),
    };

    let _ = tx.send(action).await;
}
