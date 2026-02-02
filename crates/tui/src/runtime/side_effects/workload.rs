//! Side effects for workload management operations.
//!
//! Responsibilities:
//! - Handle LoadWorkloadPools action to fetch workload pools
//! - Handle LoadWorkloadRules action to fetch workload rules
//! - Handle LoadMoreWorkloadPools and LoadMoreWorkloadRules for pagination
//!
//! Does NOT handle:
//! - UI rendering (handled by screen module)
//! - Input handling (handled by input handlers)

use tokio::sync::mpsc::Sender;

use crate::action::Action;
use crate::runtime::side_effects::SharedClient;

/// Handle loading workload pools.
///
/// Fetches the list of workload pools from the Splunk server.
pub async fn handle_load_workload_pools(
    client: SharedClient,
    tx: Sender<Action>,
    count: u64,
    offset: u64,
) {
    let result = {
        let mut guard = client.lock().await;
        guard.list_workload_pools(Some(count), Some(offset)).await
    };

    let action = match result {
        Ok(pools) => Action::WorkloadPoolsLoaded(Ok(pools)),
        Err(e) => Action::WorkloadPoolsLoaded(Err(std::sync::Arc::new(e))),
    };

    let _ = tx.send(action).await;
}

/// Handle loading more workload pools (pagination).
///
/// Fetches the next page of workload pools from the Splunk server.
pub async fn handle_load_more_workload_pools(
    client: SharedClient,
    tx: Sender<Action>,
    count: u64,
    offset: u64,
) {
    let result = {
        let mut guard = client.lock().await;
        guard.list_workload_pools(Some(count), Some(offset)).await
    };

    let action = match result {
        Ok(pools) => Action::MoreWorkloadPoolsLoaded(Ok(pools)),
        Err(e) => Action::MoreWorkloadPoolsLoaded(Err(std::sync::Arc::new(e))),
    };

    let _ = tx.send(action).await;
}

/// Handle loading workload rules.
///
/// Fetches the list of workload rules from the Splunk server.
pub async fn handle_load_workload_rules(
    client: SharedClient,
    tx: Sender<Action>,
    count: u64,
    offset: u64,
) {
    let result = {
        let mut guard = client.lock().await;
        guard.list_workload_rules(Some(count), Some(offset)).await
    };

    let action = match result {
        Ok(rules) => Action::WorkloadRulesLoaded(Ok(rules)),
        Err(e) => Action::WorkloadRulesLoaded(Err(std::sync::Arc::new(e))),
    };

    let _ = tx.send(action).await;
}

/// Handle loading more workload rules (pagination).
///
/// Fetches the next page of workload rules from the Splunk server.
pub async fn handle_load_more_workload_rules(
    client: SharedClient,
    tx: Sender<Action>,
    count: u64,
    offset: u64,
) {
    let result = {
        let mut guard = client.lock().await;
        guard.list_workload_rules(Some(count), Some(offset)).await
    };

    let action = match result {
        Ok(rules) => Action::MoreWorkloadRulesLoaded(Ok(rules)),
        Err(e) => Action::MoreWorkloadRulesLoaded(Err(std::sync::Arc::new(e))),
    };

    let _ = tx.send(action).await;
}
