//! Undo/Redo system for destructive TUI operations.
//!
//! This module provides a "trash" style undo system where destructive operations
//! are queued with a grace period (30 seconds by default) before execution.
//! Users can undo operations within the grace period using Ctrl+Z.
//!
//! # Example
//!
//! ```
//! use splunk_tui::undo::{UndoBuffer, UndoableOperation};
//! use std::time::Duration;
//!
//! let mut buffer = UndoBuffer::new();
//!
//! // Queue a destructive operation
//! let op = UndoableOperation::DeleteIndex {
//!     name: "test_idx".to_string(),
//!     original_settings: None,
//! };
//! let id = buffer.push(op, "Delete index 'test_idx'".to_string());
//!
//! // Check if we can undo
//! assert!(buffer.can_undo());
//!
//! // Undo the operation before grace period expires
//! if let Some(entry) = buffer.undo() {
//!     println!("Undone: {}", entry.description);
//! }
//! ```

use std::collections::VecDeque;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Default grace period before operation is executed (30 seconds).
pub const DEFAULT_UNDO_GRACE_PERIOD: Duration = Duration::from_secs(30);

/// Maximum number of operations to keep in history.
pub const MAX_UNDO_HISTORY: usize = 50;

/// Types of operations that can be undone.
#[derive(Debug, Clone)]
pub enum UndoableOperation {
    /// Delete index (can be undone by recreating with same settings).
    DeleteIndex {
        /// Name of the index to delete.
        name: String,
        /// Original index settings for recovery.
        original_settings: Option<IndexSettings>,
    },
    /// Delete saved search (can be undone by recreating).
    DeleteSavedSearch {
        /// Name of the saved search.
        name: String,
        /// Original saved search data for recovery.
        original: Option<SavedSearchRecoveryData>,
    },
    /// Delete job (cannot truly undo - just cancels the deletion action).
    DeleteJob {
        /// Search ID of the job.
        sid: String,
    },
    /// Delete lookup table.
    DeleteLookup {
        /// Name of the lookup table.
        name: String,
        /// App context.
        app: Option<String>,
        /// Owner context.
        owner: Option<String>,
    },
    /// Modify index settings (undo restores original settings).
    ModifyIndex {
        /// Name of the index.
        name: String,
        /// Original parameters.
        original_params: ModifyIndexParams,
        /// New parameters.
        new_params: ModifyIndexParams,
    },
    /// Delete user.
    DeleteUser {
        /// Username.
        name: String,
        /// Original user data for recovery.
        original: Option<UserRecoveryData>,
    },
    /// Delete role.
    DeleteRole {
        /// Role name.
        name: String,
    },
    /// Remove app.
    RemoveApp {
        /// App name.
        app_name: String,
    },
    /// Delete profile (local config only).
    DeleteProfile {
        /// Profile name.
        name: String,
        /// Original profile data for recovery.
        original: Option<ProfileRecoveryData>,
    },
    /// Cancel job (can be undone by recreating the search).
    CancelJob {
        /// Search ID of the job.
        sid: String,
        /// Original search query for recovery.
        search_query: Option<String>,
    },
    /// Batch delete jobs.
    DeleteJobsBatch {
        /// List of search IDs.
        sids: Vec<String>,
    },
    /// Batch cancel jobs.
    CancelJobsBatch {
        /// List of search IDs.
        sids: Vec<String>,
    },
}

/// Recovery data for saved searches.
#[derive(Debug, Clone)]
pub struct SavedSearchRecoveryData {
    /// The search query.
    pub search: String,
    /// Description of the saved search.
    pub description: Option<String>,
    /// Whether the saved search is disabled.
    pub disabled: bool,
}

/// Recovery data for users.
#[derive(Debug, Clone)]
pub struct UserRecoveryData {
    /// Roles assigned to the user.
    pub roles: Vec<String>,
    /// Real name of the user.
    pub realname: Option<String>,
    /// Email address.
    pub email: Option<String>,
    /// Default app.
    pub default_app: Option<String>,
}

/// Recovery data for profiles.
#[derive(Debug, Clone)]
pub struct ProfileRecoveryData {
    /// Base URL of the Splunk server.
    pub base_url: String,
    /// Username.
    pub username: String,
    /// Whether to skip TLS verification.
    pub skip_verify: bool,
    /// Timeout in seconds.
    pub timeout_seconds: u64,
    /// Maximum retries.
    pub max_retries: usize,
    /// Whether to use keyring.
    pub use_keyring: bool,
}

/// Index settings for recovery.
#[derive(Debug, Clone)]
pub struct IndexSettings {
    /// Maximum data size in MB.
    pub max_data_size_mb: Option<usize>,
    /// Maximum hot buckets.
    pub max_hot_buckets: Option<usize>,
    /// Maximum warm DB count.
    pub max_warm_db_count: Option<usize>,
    /// Frozen time period in seconds.
    pub frozen_time_period_secs: Option<usize>,
    /// Home path.
    pub home_path: Option<String>,
    /// Cold DB path.
    pub cold_db_path: Option<String>,
    /// Thawed path.
    pub thawed_path: Option<String>,
    /// Cold to frozen directory.
    pub cold_to_frozen_dir: Option<String>,
}

/// Parameters for modifying an index.
#[derive(Debug, Clone, Default)]
pub struct ModifyIndexParams {
    /// Maximum data size in MB.
    pub max_data_size_mb: Option<usize>,
    /// Maximum hot buckets.
    pub max_hot_buckets: Option<usize>,
    /// Maximum warm DB count.
    pub max_warm_db_count: Option<usize>,
    /// Frozen time period in seconds.
    pub frozen_time_period_secs: Option<usize>,
    /// Home path.
    pub home_path: Option<String>,
    /// Cold DB path.
    pub cold_db_path: Option<String>,
    /// Thawed path.
    pub thawed_path: Option<String>,
    /// Cold to frozen directory.
    pub cold_to_frozen_dir: Option<String>,
}

/// Entry in the undo buffer.
#[derive(Debug, Clone)]
pub struct UndoEntry {
    /// Unique ID for this entry.
    pub id: Uuid,
    /// The operation that can be undone.
    pub operation: UndoableOperation,
    /// When this entry was created.
    pub created_at: Instant,
    /// Grace period before execution.
    pub grace_period: Duration,
    /// Whether the operation has been executed.
    pub executed: bool,
    /// Whether this entry has been undone.
    pub undone: bool,
    /// Human-readable description of the operation.
    pub description: String,
}

impl UndoEntry {
    /// Create a new undo entry.
    pub fn new(operation: UndoableOperation, description: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            operation,
            created_at: Instant::now(),
            grace_period: DEFAULT_UNDO_GRACE_PERIOD,
            executed: false,
            undone: false,
            description,
        }
    }

    /// Check if the grace period has expired.
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() >= self.grace_period
    }

    /// Get remaining time before expiration.
    pub fn remaining(&self) -> Duration {
        self.grace_period.saturating_sub(self.created_at.elapsed())
    }

    /// Get remaining seconds as integer.
    pub fn remaining_secs(&self) -> u64 {
        self.remaining().as_secs()
    }
}

/// Status of an undo entry for display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UndoEntryStatus {
    /// Operation is pending (not yet executed, within grace period).
    Pending,
    /// Operation has been executed.
    Executed,
    /// Operation was undone.
    Undone,
    /// Operation expired without being executed or undone.
    Expired,
}

impl UndoEntry {
    /// Get the status of this entry.
    pub fn status(&self) -> UndoEntryStatus {
        if self.undone {
            UndoEntryStatus::Undone
        } else if self.executed {
            UndoEntryStatus::Executed
        } else if self.is_expired() {
            UndoEntryStatus::Expired
        } else {
            UndoEntryStatus::Pending
        }
    }
}

/// Buffer for managing undoable operations.
#[derive(Debug)]
pub struct UndoBuffer {
    /// The undo stack (most recent first).
    history: VecDeque<UndoEntry>,
    /// Redo stack for undone operations.
    redo_stack: Vec<UndoEntry>,
    /// Maximum history size.
    max_history: usize,
}

impl UndoBuffer {
    /// Create a new empty undo buffer.
    pub fn new() -> Self {
        Self {
            history: VecDeque::with_capacity(MAX_UNDO_HISTORY),
            redo_stack: Vec::new(),
            max_history: MAX_UNDO_HISTORY,
        }
    }

    /// Add an operation to the buffer.
    ///
    /// Returns the unique ID of the created entry.
    pub fn push(&mut self, operation: UndoableOperation, description: String) -> Uuid {
        let entry = UndoEntry::new(operation, description);
        let id = entry.id;

        // Add to front of history
        self.history.push_front(entry);

        // Trim old entries
        while self.history.len() > self.max_history {
            self.history.pop_back();
        }

        // Clear redo stack when new operation is added
        self.redo_stack.clear();

        id
    }

    /// Get the first pending (non-expired, non-executed) operation.
    /// Used for UI countdown display - excludes expired entries.
    pub fn peek_pending(&self) -> Option<&UndoEntry> {
        self.history
            .iter()
            .find(|e| !e.executed && !e.undone && !e.is_expired())
    }

    /// Get mutable reference to first pending operation.
    /// Used for UI countdown display - excludes expired entries.
    pub fn peek_pending_mut(&mut self) -> Option<&mut UndoEntry> {
        self.history
            .iter_mut()
            .find(|e| !e.executed && !e.undone && !e.is_expired())
    }

    /// Get the first queued operation that needs execution (not executed, not undone).
    /// Unlike peek_pending, this includes expired entries so they can be executed.
    /// Used by the executor to find operations ready for execution.
    pub fn peek_queued_for_execution(&self) -> Option<&UndoEntry> {
        self.history.iter().find(|e| !e.executed && !e.undone)
    }

    /// Get mutable reference to the first queued operation for execution.
    /// Unlike peek_pending_mut, this includes expired entries so they can be executed.
    /// Used by the executor to find operations ready for execution.
    pub fn peek_queued_for_execution_mut(&mut self) -> Option<&mut UndoEntry> {
        self.history.iter_mut().find(|e| !e.executed && !e.undone)
    }

    /// Mark an operation as executed.
    pub fn mark_executed(&mut self, id: Uuid) {
        if let Some(entry) = self.history.iter_mut().find(|e| e.id == id) {
            entry.executed = true;
        }
    }

    /// Undo the most recent pending or executed operation.
    ///
    /// Returns the operation that was undone.
    pub fn undo(&mut self) -> Option<UndoEntry> {
        // Find first entry that can be undone (pending or executed, not already undone, not expired)
        if let Some(index) = self
            .history
            .iter()
            .position(|e| !e.undone && (!e.is_expired() || e.executed))
        {
            let mut entry = self.history.remove(index).unwrap();
            entry.undone = true;

            // Clone for redo stack before consuming
            let redo_entry = entry.clone();
            self.redo_stack.push(redo_entry);

            Some(entry)
        } else {
            None
        }
    }

    /// Redo the most recently undone operation.
    ///
    /// Returns the operation to redo.
    pub fn redo(&mut self) -> Option<UndoEntry> {
        if let Some(mut entry) = self.redo_stack.pop() {
            // Clear the undone flag so it can be undone again
            entry.undone = false;
            // Push back to history so it can be undone
            self.history.push_front(entry.clone());
            Some(entry)
        } else {
            None
        }
    }

    /// Get all history entries for display.
    pub fn history(&self) -> &VecDeque<UndoEntry> {
        &self.history
    }

    /// Get mutable access to history entries (for testing).
    pub fn history_mut(&mut self) -> &mut VecDeque<UndoEntry> {
        &mut self.history
    }

    /// Clear expired pending operations.
    pub fn clear_expired(&mut self) {
        // Keep all executed or undone entries, plus non-expired pending ones
        self.history
            .retain(|e| e.executed || e.undone || !e.is_expired());
    }

    /// Check if there's an operation that can be undone.
    pub fn can_undo(&self) -> bool {
        self.history
            .iter()
            .any(|e| !e.undone && (!e.is_expired() || e.executed))
    }

    /// Check if there's an operation that can be redone.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Get count of pending operations.
    pub fn pending_count(&self) -> usize {
        self.history
            .iter()
            .filter(|e| !e.executed && !e.undone && !e.is_expired())
            .count()
    }

    /// Get the redo stack (for display purposes).
    pub fn redo_stack(&self) -> &[UndoEntry] {
        &self.redo_stack
    }
}

impl Default for UndoBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_undo_buffer_push_and_undo() {
        let mut buffer = UndoBuffer::new();

        let op = UndoableOperation::DeleteIndex {
            name: "test_idx".to_string(),
            original_settings: None,
        };

        buffer.push(op, "Delete index 'test_idx'".to_string());

        assert!(buffer.can_undo());
        assert_eq!(buffer.pending_count(), 1);

        let entry = buffer.undo().unwrap();
        assert_eq!(entry.description, "Delete index 'test_idx'");
        assert!(entry.undone);
    }

    #[test]
    fn test_undo_redo_roundtrip() {
        let mut buffer = UndoBuffer::new();

        let op = UndoableOperation::DeleteJob {
            sid: "123".to_string(),
        };

        buffer.push(op.clone(), "Delete job 123".to_string());

        // Undo
        let _ = buffer.undo();
        assert!(buffer.can_redo());

        // Redo
        let entry = buffer.redo().unwrap();
        assert_eq!(entry.description, "Delete job 123");
        assert!(!buffer.can_redo());
    }

    #[test]
    fn test_undo_buffer_max_history() {
        let mut buffer = UndoBuffer::new();

        // Fill beyond max
        for i in 0..60 {
            let op = UndoableOperation::DeleteIndex {
                name: format!("idx{}", i),
                original_settings: None,
            };
            buffer.push(op, format!("Delete {}", i));
        }

        // Should only keep MAX_UNDO_HISTORY
        assert_eq!(buffer.history().len(), MAX_UNDO_HISTORY);
    }

    #[test]
    fn test_clear_expired() {
        let mut buffer = UndoBuffer::new();

        let op = UndoableOperation::DeleteIndex {
            name: "test".to_string(),
            original_settings: None,
        };

        buffer.push(op, "Delete test".to_string());

        // Manually expire the entry
        if let Some(entry) = buffer.peek_pending_mut() {
            entry.created_at = Instant::now() - Duration::from_secs(60);
        }

        // Should have 1 entry before clearing
        assert_eq!(buffer.history().len(), 1);

        // Clear expired
        buffer.clear_expired();

        // Expired pending entries should be removed
        assert_eq!(buffer.history().len(), 0);
    }

    #[test]
    fn test_entry_status() {
        let op = UndoableOperation::DeleteIndex {
            name: "test".to_string(),
            original_settings: None,
        };

        let mut entry = UndoEntry::new(op, "Test".to_string());

        // Fresh entry should be pending
        assert_eq!(entry.status(), UndoEntryStatus::Pending);

        // Mark as executed
        entry.executed = true;
        assert_eq!(entry.status(), UndoEntryStatus::Executed);

        // Mark as undone
        entry.undone = true;
        assert_eq!(entry.status(), UndoEntryStatus::Undone);

        // Reset and expire
        entry.executed = false;
        entry.undone = false;
        entry.created_at = Instant::now() - Duration::from_secs(60);
        assert_eq!(entry.status(), UndoEntryStatus::Expired);
    }
}
