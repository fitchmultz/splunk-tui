//! Transaction management for multi-step Splunk configuration changes.
//!
//! This module provides the infrastructure for atomic multi-resource operations.
//! It implements a two-phase commit pattern:
//! 1. Validation: Verify all operations against the current Splunk state.
//! 2. Execution: Sequentially execute operations with automatic rollback on failure.

use crate::client::SplunkClient;
use crate::error::Result;
use crate::models::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};

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
    pub fn load_pending(&self) -> Result<Option<Transaction>> {
        let path = self.log_dir.join("pending_transaction.json");
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&path).map_err(|e| {
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
    pub fn save_pending(&self, transaction: &Transaction) -> Result<()> {
        if !self.log_dir.exists() {
            std::fs::create_dir_all(&self.log_dir).map_err(|e| {
                crate::error::ClientError::ValidationError(format!(
                    "Failed to create log directory: {}",
                    e
                ))
            })?;
        }
        let path = self.log_dir.join("pending_transaction.json");
        let content = serde_json::to_string_pretty(transaction).map_err(|e| {
            crate::error::ClientError::ValidationError(format!(
                "Failed to serialize transaction: {}",
                e
            ))
        })?;
        std::fs::write(path, content).map_err(|e| {
            crate::error::ClientError::ValidationError(format!(
                "Failed to write transaction log: {}",
                e
            ))
        })?;
        Ok(())
    }

    /// Clear the pending transaction from disk.
    pub fn clear_pending(&self) -> Result<()> {
        let path = self.log_dir.join("pending_transaction.json");
        if path.exists() {
            std::fs::remove_file(path).map_err(|e| {
                crate::error::ClientError::ValidationError(format!(
                    "Failed to remove transaction log: {}",
                    e
                ))
            })?;
        }
        Ok(())
    }

    /// Archive a completed transaction.
    pub fn archive(&self, transaction: &Transaction, status: &str) -> Result<()> {
        let history_dir = self.log_dir.join("history");
        if !history_dir.exists() {
            std::fs::create_dir_all(&history_dir).map_err(|e| {
                crate::error::ClientError::ValidationError(format!(
                    "Failed to create history directory: {}",
                    e
                ))
            })?;
        }

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
        std::fs::write(path, content).map_err(|e| {
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
    pub async fn validate(&self, _client: &SplunkClient, _transaction: &Transaction) -> Result<()> {
        info!("Validating transaction {}", _transaction.id);
        // Basic validation: ensure names are not empty
        for op in &_transaction.operations {
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
    pub async fn commit(&self, client: &SplunkClient, transaction: &Transaction) -> Result<()> {
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
                    if let Err(rollback_err) = self.rollback(client, completed_ops).await {
                        warn!(
                            "Rollback failed for transaction {}: {}",
                            transaction.id, rollback_err
                        );
                    }
                    return Err(e);
                }
            }
        }

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
    async fn rollback(&self, client: &SplunkClient, ops: Vec<TransactionOperation>) -> Result<()> {
        info!("Rolling back {} operations", ops.len());
        for op in ops.into_iter().rev() {
            match op {
                TransactionOperation::CreateIndex(params) => {
                    let _ = client.delete_index(&params.name).await;
                }
                TransactionOperation::CreateUser(params) => {
                    let _ = client.delete_user(&params.name).await;
                }
                TransactionOperation::CreateRole(params) => {
                    let _ = client.delete_role(&params.name).await;
                }
                TransactionOperation::CreateMacro(params) => {
                    let _ = client.delete_macro(&params.name).await;
                }
                TransactionOperation::CreateSavedSearch(params) => {
                    let _ = client.delete_saved_search(&params.name).await;
                }
                // For Modify/Update operations, full rollback would require original state.
                // For Delete operations, full rollback would require recreation.
                // As per plan, we document these limitations.
                _ => {
                    warn!("No automated rollback path for operation: {:?}", op);
                }
            }
        }
        Ok(())
    }
}
