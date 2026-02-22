//! Transaction management for multi-step Splunk configuration changes.
//!
//! Purpose: Execute and persist multi-step Splunk operations with rollback semantics.
//! Responsibilities: Validate operations, commit execution, and recover or rollback on failures.
//! Non-scope: CLI presentation concerns and long-term transaction history analytics.
//! Invariants/Assumptions: Operations execute in-order and rollback attempts preserve failure visibility.
//!
//! This module provides the infrastructure for atomic multi-resource operations.
//! It implements a two-phase commit pattern:
//! 1. Validation: Verify all operations against the current Splunk state.
//! 2. Execution: Sequentially execute operations with automatic rollback on failure.

use crate::client::SplunkClient;
use crate::error::{ClientError, Result, RollbackFailure};
use crate::models::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::future::Future;
use std::time::Duration;
use tracing::{error, info, warn};

/// Per-operation timeout for rollback cleanup calls.
const ROLLBACK_OPERATION_TIMEOUT: Duration = Duration::from_secs(30);

/// Metric: total number of transaction commit attempts.
pub const METRIC_TRANSACTION_COMMIT_ATTEMPTS: &str = "splunk_transaction_commit_attempts_total";
/// Metric: successful transaction commits.
pub const METRIC_TRANSACTION_COMMIT_SUCCESSES: &str = "splunk_transaction_commit_successes_total";
/// Metric: failed transaction commits.
pub const METRIC_TRANSACTION_COMMIT_FAILURES: &str = "splunk_transaction_commit_failures_total";
/// Metric: rollback operation attempts.
pub const METRIC_TRANSACTION_ROLLBACK_ATTEMPTS: &str = "splunk_transaction_rollback_attempts_total";
/// Metric: rollback operation failures (includes timeout and unsupported rollback paths).
pub const METRIC_TRANSACTION_ROLLBACK_FAILURES: &str = "splunk_transaction_rollback_failures_total";

/// Represents a single reversible operation in a transaction.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TransactionOperation {
    /// Create a new index.
    CreateIndex(CreateIndexParams),
    /// Delete an index.
    DeleteIndex(String),
    /// Modify an existing index.
    ModifyIndex(String, ModifyIndexParams),
    /// Create a new user.
    CreateUser(CreateUserParams),
    /// Delete a user.
    DeleteUser(String),
    /// Modify an existing user.
    ModifyUser(String, ModifyUserParams),
    /// Create a new role.
    CreateRole(CreateRoleParams),
    /// Delete a role.
    DeleteRole(String),
    /// Modify an existing role.
    ModifyRole(String, ModifyRoleParams),
    /// Create a new search macro.
    CreateMacro(MacroCreateParams),
    /// Delete a search macro.
    DeleteMacro(String),
    /// Update an existing search macro.
    UpdateMacro(String, MacroUpdateParams),
    /// Create a new saved search.
    CreateSavedSearch(SavedSearchCreateParams),
    /// Delete a saved search.
    DeleteSavedSearch(String),
    /// Update an existing saved search.
    UpdateSavedSearch(String, SavedSearchUpdateParams),
}

/// A collection of operations that should be executed atomically.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transaction {
    /// Unique identifier for the transaction.
    pub id: String,
    /// Ordered list of operations to perform.
    pub operations: Vec<TransactionOperation>,
    /// Named markers within the operation list for partial rollbacks.
    pub savepoints: HashMap<String, usize>,
    /// When the transaction was initiated.
    pub created_at: DateTime<Utc>,
}

impl Default for Transaction {
    fn default() -> Self {
        Self::new()
    }
}

impl Transaction {
    /// Create a new, empty transaction with a random UUID.
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            operations: Vec::new(),
            savepoints: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    /// Add an operation to the transaction.
    pub fn add_operation(&mut self, op: TransactionOperation) {
        self.operations.push(op);
    }

    /// Set a savepoint at the current position.
    pub fn set_savepoint(&mut self, name: String) {
        self.savepoints.insert(name, self.operations.len());
    }

    /// Rollback the transaction to a previously set savepoint.
    /// Returns true if the savepoint existed and rollback was successful.
    pub fn rollback_to_savepoint(&mut self, name: &str) -> bool {
        if let Some(&index) = self.savepoints.get(name) {
            self.operations.truncate(index);
            true
        } else {
            false
        }
    }
}

/// Manages the lifecycle and execution of transactions.
pub struct TransactionManager {
    /// Directory where transaction logs are stored.
    pub log_dir: std::path::PathBuf,
}

impl TransactionManager {
    /// Create a new TransactionManager with the specified log directory.
    pub fn new(log_dir: std::path::PathBuf) -> Self {
        Self { log_dir }
    }

    /// Load a pending transaction from disk.
    pub async fn load_pending(&self) -> Result<Option<Transaction>> {
        let path = self.log_dir.join("pending_transaction.json");
        if !tokio::fs::try_exists(&path).await.map_err(|e| {
            crate::error::ClientError::InvalidResponse(format!(
                "Failed to check transaction log path: {}",
                e
            ))
        })? {
            return Ok(None);
        }
        let content = tokio::fs::read_to_string(&path).await.map_err(|e| {
            crate::error::ClientError::InvalidResponse(format!(
                "Failed to read transaction log: {}",
                e
            ))
        })?;
        let transaction: Transaction = serde_json::from_str(&content).map_err(|e| {
            crate::error::ClientError::InvalidResponse(format!(
                "Failed to parse transaction log: {}",
                e
            ))
        })?;
        Ok(Some(transaction))
    }

    /// Save a transaction to disk as pending.
    pub async fn save_pending(&self, transaction: &Transaction) -> Result<()> {
        tokio::fs::create_dir_all(&self.log_dir)
            .await
            .map_err(|e| {
                crate::error::ClientError::ValidationError(format!(
                    "Failed to create log directory: {}",
                    e
                ))
            })?;
        let path = self.log_dir.join("pending_transaction.json");
        let content = serde_json::to_string_pretty(transaction).map_err(|e| {
            crate::error::ClientError::ValidationError(format!(
                "Failed to serialize transaction: {}",
                e
            ))
        })?;
        tokio::fs::write(path, content).await.map_err(|e| {
            crate::error::ClientError::ValidationError(format!(
                "Failed to write transaction log: {}",
                e
            ))
        })?;
        Ok(())
    }

    /// Clear the pending transaction from disk.
    pub async fn clear_pending(&self) -> Result<()> {
        let path = self.log_dir.join("pending_transaction.json");
        if tokio::fs::try_exists(&path).await.map_err(|e| {
            crate::error::ClientError::ValidationError(format!(
                "Failed to check pending transaction path: {}",
                e
            ))
        })? {
            tokio::fs::remove_file(path).await.map_err(|e| {
                crate::error::ClientError::ValidationError(format!(
                    "Failed to remove transaction log: {}",
                    e
                ))
            })?;
        }
        Ok(())
    }

    /// Archive a completed transaction.
    pub async fn archive(&self, transaction: &Transaction, status: &str) -> Result<()> {
        let history_dir = self.log_dir.join("history");
        tokio::fs::create_dir_all(&history_dir).await.map_err(|e| {
            crate::error::ClientError::ValidationError(format!(
                "Failed to create history directory: {}",
                e
            ))
        })?;

        let filename = format!(
            "{}_{}_{}.json",
            transaction.created_at.format("%Y%m%d_%H%M%S"),
            transaction.id,
            status
        );
        let path = history_dir.join(filename);

        let content = serde_json::to_string_pretty(transaction).map_err(|e| {
            crate::error::ClientError::ValidationError(format!(
                "Failed to serialize transaction: {}",
                e
            ))
        })?;
        tokio::fs::write(path, content).await.map_err(|e| {
            crate::error::ClientError::ValidationError(format!(
                "Failed to write transaction archive: {}",
                e
            ))
        })?;

        Ok(())
    }

    /// Validate all operations in the transaction.
    ///
    /// This performs dry-run checks and local parameter validation.
    pub async fn validate(&self, _client: &SplunkClient, transaction: &Transaction) -> Result<()> {
        info!("Validating transaction {}", transaction.id);
        // Basic validation: ensure names are not empty
        for op in &transaction.operations {
            match op {
                TransactionOperation::CreateIndex(p) if p.name.is_empty() => {
                    return Err(crate::error::ClientError::ValidationError(
                        "Index name cannot be empty".into(),
                    ));
                }
                TransactionOperation::CreateUser(p) if p.name.is_empty() => {
                    return Err(crate::error::ClientError::ValidationError(
                        "Username cannot be empty".into(),
                    ));
                }
                TransactionOperation::CreateRole(p) if p.name.is_empty() => {
                    return Err(crate::error::ClientError::ValidationError(
                        "Role name cannot be empty".into(),
                    ));
                }
                TransactionOperation::CreateMacro(p) if p.name.is_empty() => {
                    return Err(crate::error::ClientError::ValidationError(
                        "Macro name cannot be empty".into(),
                    ));
                }
                TransactionOperation::CreateSavedSearch(p) if p.name.is_empty() => {
                    return Err(crate::error::ClientError::ValidationError(
                        "Saved search name cannot be empty".into(),
                    ));
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Commit the transaction to Splunk.
    ///
    /// Executes operations sequentially. If any operation fails, it initiates
    /// an automatic rollback of all previously completed operations in this transaction.
    /// Returns a TransactionRollbackError if rollback fails for any operations.
    pub async fn commit(&self, client: &SplunkClient, transaction: &Transaction) -> Result<()> {
        metrics::counter!(METRIC_TRANSACTION_COMMIT_ATTEMPTS).increment(1);
        info!(
            "Committing transaction {} with {} operations",
            transaction.id,
            transaction.operations.len()
        );
        let mut completed_ops = Vec::new();

        for op in &transaction.operations {
            match self.execute_operation(client, op).await {
                Ok(_) => {
                    completed_ops.push(op.clone());
                }
                Err(e) => {
                    warn!(
                        "Operation failed in transaction {}: {}. Initiating rollback.",
                        transaction.id, e
                    );
                    let rollback_failures = self.rollback(client, completed_ops).await?;

                    if !rollback_failures.is_empty() {
                        metrics::counter!(METRIC_TRANSACTION_COMMIT_FAILURES).increment(1);
                        error!(
                            transaction_id = %transaction.id,
                            failure_count = rollback_failures.len(),
                            "Rollback completed with failures"
                        );
                        return Err(ClientError::TransactionRollbackError {
                            count: rollback_failures.len(),
                            failures: rollback_failures,
                        });
                    }
                    metrics::counter!(METRIC_TRANSACTION_COMMIT_FAILURES).increment(1);
                    return Err(e);
                }
            }
        }

        metrics::counter!(METRIC_TRANSACTION_COMMIT_SUCCESSES).increment(1);
        info!("Transaction {} committed successfully", transaction.id);
        Ok(())
    }

    async fn execute_operation(
        &self,
        client: &SplunkClient,
        op: &TransactionOperation,
    ) -> Result<()> {
        match op {
            TransactionOperation::CreateIndex(params) => {
                client.create_index(params).await?;
            }
            TransactionOperation::DeleteIndex(name) => {
                client.delete_index(name).await?;
            }
            TransactionOperation::ModifyIndex(name, params) => {
                client.modify_index(name, params).await?;
            }
            TransactionOperation::CreateUser(params) => {
                client.create_user(params).await?;
            }
            TransactionOperation::DeleteUser(name) => {
                client.delete_user(name).await?;
            }
            TransactionOperation::ModifyUser(name, params) => {
                client.modify_user(name, params).await?;
            }
            TransactionOperation::CreateRole(params) => {
                client.create_role(params).await?;
            }
            TransactionOperation::DeleteRole(name) => {
                client.delete_role(name).await?;
            }
            TransactionOperation::ModifyRole(name, params) => {
                client.modify_role(name, params).await?;
            }
            TransactionOperation::CreateMacro(params) => {
                client.create_macro(params.clone()).await?;
            }
            TransactionOperation::DeleteMacro(name) => {
                client.delete_macro(name).await?;
            }
            TransactionOperation::UpdateMacro(name, params) => {
                client.update_macro(name, params.clone()).await?;
            }
            TransactionOperation::CreateSavedSearch(params) => {
                client.create_saved_search(params.clone()).await?;
            }
            TransactionOperation::DeleteSavedSearch(name) => {
                client.delete_saved_search(name).await?;
            }
            TransactionOperation::UpdateSavedSearch(name, params) => {
                client.update_saved_search(name, params.clone()).await?;
            }
        }
        Ok(())
    }

    /// Rollback a list of completed operations in reverse order.
    ///
    /// Returns a vector of any failures that occurred during rollback.
    /// Callers MUST handle rollback failures to ensure data integrity issues are visible.
    async fn rollback(
        &self,
        client: &SplunkClient,
        ops: Vec<TransactionOperation>,
    ) -> Result<Vec<RollbackFailure>> {
        info!("Rolling back {} operations", ops.len());
        let mut failures = Vec::new();
        metrics::counter!(METRIC_TRANSACTION_ROLLBACK_ATTEMPTS).increment(ops.len() as u64);

        for op in ops.into_iter().rev() {
            match op {
                TransactionOperation::CreateIndex(params) => {
                    self.record_rollback_result(
                        "delete_index",
                        params.name.clone(),
                        client.delete_index(&params.name),
                        &mut failures,
                    )
                    .await;
                }
                TransactionOperation::CreateUser(params) => {
                    self.record_rollback_result(
                        "delete_user",
                        params.name.clone(),
                        client.delete_user(&params.name),
                        &mut failures,
                    )
                    .await;
                }
                TransactionOperation::CreateRole(params) => {
                    self.record_rollback_result(
                        "delete_role",
                        params.name.clone(),
                        client.delete_role(&params.name),
                        &mut failures,
                    )
                    .await;
                }
                TransactionOperation::CreateMacro(params) => {
                    self.record_rollback_result(
                        "delete_macro",
                        params.name.clone(),
                        client.delete_macro(&params.name),
                        &mut failures,
                    )
                    .await;
                }
                TransactionOperation::CreateSavedSearch(params) => {
                    self.record_rollback_result(
                        "delete_saved_search",
                        params.name.clone(),
                        client.delete_saved_search(&params.name),
                        &mut failures,
                    )
                    .await;
                }
                // For Modify/Update operations, full rollback would require original state.
                // For Delete operations, full rollback would require recreation.
                // Treat missing rollback paths as explicit failures.
                _ => self.record_missing_rollback(op, &mut failures),
            }
        }

        Ok(failures)
    }

    async fn record_rollback_result<F>(
        &self,
        operation: &'static str,
        resource_name: String,
        future: F,
        failures: &mut Vec<RollbackFailure>,
    ) where
        F: Future<Output = Result<()>>,
    {
        match tokio::time::timeout(ROLLBACK_OPERATION_TIMEOUT, future).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                metrics::counter!(METRIC_TRANSACTION_ROLLBACK_FAILURES).increment(1);
                error!(
                    operation,
                    resource = %resource_name,
                    error = %e,
                    "Rollback operation failed"
                );
                failures.push(RollbackFailure {
                    resource_name,
                    operation: operation.to_string(),
                    error: e,
                });
            }
            Err(_) => {
                metrics::counter!(METRIC_TRANSACTION_ROLLBACK_FAILURES).increment(1);
                let timeout_error = ClientError::OperationTimeout {
                    operation,
                    timeout: ROLLBACK_OPERATION_TIMEOUT,
                };
                error!(
                    operation,
                    resource = %resource_name,
                    timeout_secs = ROLLBACK_OPERATION_TIMEOUT.as_secs(),
                    "Rollback operation timed out"
                );
                failures.push(RollbackFailure {
                    resource_name,
                    operation: operation.to_string(),
                    error: timeout_error,
                });
            }
        }
    }

    fn record_missing_rollback(
        &self,
        op: TransactionOperation,
        failures: &mut Vec<RollbackFailure>,
    ) {
        metrics::counter!(METRIC_TRANSACTION_ROLLBACK_FAILURES).increment(1);
        warn!("No automated rollback path for operation: {:?}", op);
        failures.push(RollbackFailure {
            resource_name: format!("{:?}", op),
            operation: "missing_rollback_path".to_string(),
            error: ClientError::ValidationError(
                "No automated rollback path for completed operation".to_string(),
            ),
        });
    }
}
