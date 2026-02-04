//! Side effects for workload management operations.
//!
//! Responsibilities:
//! - Handle LoadWorkloadPools action to fetch workload pools
//! - Handle LoadWorkloadRules action to fetch workload rules
//!
//! Does NOT handle:
//! - UI rendering (handled by screen module)
//! - Input handling (handled by input handlers)
//! - LoadMoreWorkloadPools/LoadMoreWorkloadRules directly (handled by main loop)
//!
//! # Pagination Invariant
//!
//! When `offset == 0`, results replace the current list (`*Loaded` actions).
//! When `offset > 0`, results append to the current list (`More*Loaded` actions).

use tokio::sync::mpsc::Sender;

use crate::action::Action;
use crate::runtime::side_effects::SharedClient;

/// Build the action for workload pools based on fetch result and offset.
///
/// When `offset == 0`, returns `WorkloadPoolsLoaded` (replace mode).
/// When `offset > 0`, returns `MoreWorkloadPoolsLoaded` (append mode).
fn build_workload_pools_action(
    result: Result<Vec<splunk_client::models::WorkloadPool>, splunk_client::ClientError>,
    offset: u64,
) -> Action {
    match offset {
        0 => match result {
            Ok(pools) => Action::WorkloadPoolsLoaded(Ok(pools)),
            Err(e) => Action::WorkloadPoolsLoaded(Err(std::sync::Arc::new(e))),
        },
        _ => match result {
            Ok(pools) => Action::MoreWorkloadPoolsLoaded(Ok(pools)),
            Err(e) => Action::MoreWorkloadPoolsLoaded(Err(std::sync::Arc::new(e))),
        },
    }
}

/// Build the action for workload rules based on fetch result and offset.
///
/// When `offset == 0`, returns `WorkloadRulesLoaded` (replace mode).
/// When `offset > 0`, returns `MoreWorkloadRulesLoaded` (append mode).
fn build_workload_rules_action(
    result: Result<Vec<splunk_client::models::WorkloadRule>, splunk_client::ClientError>,
    offset: u64,
) -> Action {
    match offset {
        0 => match result {
            Ok(rules) => Action::WorkloadRulesLoaded(Ok(rules)),
            Err(e) => Action::WorkloadRulesLoaded(Err(std::sync::Arc::new(e))),
        },
        _ => match result {
            Ok(rules) => Action::MoreWorkloadRulesLoaded(Ok(rules)),
            Err(e) => Action::MoreWorkloadRulesLoaded(Err(std::sync::Arc::new(e))),
        },
    }
}

/// Handle loading workload pools.
///
/// Fetches the list of workload pools from the Splunk server.
/// Uses `offset` to determine whether to replace (offset == 0) or append (offset > 0).
pub async fn handle_load_workload_pools(
    client: SharedClient,
    tx: Sender<Action>,
    count: u64,
    offset: u64,
) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut guard = client.lock().await;
        let result = guard.list_workload_pools(Some(count), Some(offset)).await;
        let action = build_workload_pools_action(result, offset);
        let _ = tx.send(action).await;
    });
}

/// Handle loading workload rules.
///
/// Fetches the list of workload rules from the Splunk server.
/// Uses `offset` to determine whether to replace (offset == 0) or append (offset > 0).
pub async fn handle_load_workload_rules(
    client: SharedClient,
    tx: Sender<Action>,
    count: u64,
    offset: u64,
) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let mut guard = client.lock().await;
        let result = guard.list_workload_rules(Some(count), Some(offset)).await;
        let action = build_workload_rules_action(result, offset);
        let _ = tx.send(action).await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_pools() -> Vec<splunk_client::models::WorkloadPool> {
        vec![]
    }

    fn create_test_rules() -> Vec<splunk_client::models::WorkloadRule> {
        vec![]
    }

    #[test]
    fn test_build_workload_pools_action_offset_zero_ok() {
        let pools = create_test_pools();
        let result = Ok(pools.clone());
        let action = build_workload_pools_action(result, 0);

        match action {
            Action::WorkloadPoolsLoaded(Ok(_)) => {}
            _ => panic!("Expected WorkloadPoolsLoaded(Ok) for offset 0"),
        }
    }

    #[test]
    fn test_build_workload_pools_action_offset_zero_err() {
        let err = splunk_client::ClientError::ApiError {
            status: 500,
            url: "http://test".to_string(),
            message: "test error".to_string(),
            request_id: None,
        };
        let action = build_workload_pools_action(Err(err), 0);

        match action {
            Action::WorkloadPoolsLoaded(Err(_)) => {}
            _ => panic!("Expected WorkloadPoolsLoaded(Err) for offset 0"),
        }
    }

    #[test]
    fn test_build_workload_pools_action_offset_nonzero_ok() {
        let pools = create_test_pools();
        let result = Ok(pools.clone());
        let action = build_workload_pools_action(result, 10);

        match action {
            Action::MoreWorkloadPoolsLoaded(Ok(_)) => {}
            _ => panic!("Expected MoreWorkloadPoolsLoaded(Ok) for offset > 0"),
        }
    }

    #[test]
    fn test_build_workload_pools_action_offset_nonzero_err() {
        let err = splunk_client::ClientError::ApiError {
            status: 500,
            url: "http://test".to_string(),
            message: "test error".to_string(),
            request_id: None,
        };
        let action = build_workload_pools_action(Err(err), 10);

        match action {
            Action::MoreWorkloadPoolsLoaded(Err(_)) => {}
            _ => panic!("Expected MoreWorkloadPoolsLoaded(Err) for offset > 0"),
        }
    }

    #[test]
    fn test_build_workload_rules_action_offset_zero_ok() {
        let rules = create_test_rules();
        let result = Ok(rules.clone());
        let action = build_workload_rules_action(result, 0);

        match action {
            Action::WorkloadRulesLoaded(Ok(_)) => {}
            _ => panic!("Expected WorkloadRulesLoaded(Ok) for offset 0"),
        }
    }

    #[test]
    fn test_build_workload_rules_action_offset_zero_err() {
        let err = splunk_client::ClientError::ApiError {
            status: 500,
            url: "http://test".to_string(),
            message: "test error".to_string(),
            request_id: None,
        };
        let action = build_workload_rules_action(Err(err), 0);

        match action {
            Action::WorkloadRulesLoaded(Err(_)) => {}
            _ => panic!("Expected WorkloadRulesLoaded(Err) for offset 0"),
        }
    }

    #[test]
    fn test_build_workload_rules_action_offset_nonzero_ok() {
        let rules = create_test_rules();
        let result = Ok(rules.clone());
        let action = build_workload_rules_action(result, 10);

        match action {
            Action::MoreWorkloadRulesLoaded(Ok(_)) => {}
            _ => panic!("Expected MoreWorkloadRulesLoaded(Ok) for offset > 0"),
        }
    }

    #[test]
    fn test_build_workload_rules_action_offset_nonzero_err() {
        let err = splunk_client::ClientError::ApiError {
            status: 500,
            url: "http://test".to_string(),
            message: "test error".to_string(),
            request_id: None,
        };
        let action = build_workload_rules_action(Err(err), 10);

        match action {
            Action::MoreWorkloadRulesLoaded(Err(_)) => {}
            _ => panic!("Expected MoreWorkloadRulesLoaded(Err) for offset > 0"),
        }
    }
}
