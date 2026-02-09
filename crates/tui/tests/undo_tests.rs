//! Integration tests for the undo/redo system.
//!
//! These tests verify the undo buffer functionality, action handling,
//! and integration with the TUI app.

use splunk_tui::undo::{ModifyIndexParams, UndoBuffer, UndoEntryStatus, UndoableOperation};
use std::time::Duration;

/// Test basic undo buffer push and undo operations
#[test]
fn test_undo_buffer_push_and_undo() {
    let mut buffer = UndoBuffer::new();

    let op = UndoableOperation::DeleteIndex {
        name: "test_idx".to_string(),
        original_settings: None,
    };

    let id = buffer.push(op, "Delete index 'test_idx'".to_string());

    // Verify entry was created
    assert!(buffer.can_undo());
    assert_eq!(buffer.pending_count(), 1);
    assert!(!id.is_nil());

    // Undo the operation
    let entry = buffer.undo().unwrap();
    assert_eq!(entry.description, "Delete index 'test_idx'");
    assert!(entry.undone);
    assert!(!buffer.can_undo()); // No more undoable operations
}

/// Test undo/redo roundtrip
#[test]
fn test_undo_redo_roundtrip() {
    let mut buffer = UndoBuffer::new();

    let op = UndoableOperation::DeleteJob {
        sid: "123".to_string(),
    };

    buffer.push(op.clone(), "Delete job 123".to_string());

    // Undo
    let undone_entry = buffer.undo().unwrap();
    assert_eq!(undone_entry.description, "Delete job 123");
    assert!(buffer.can_redo());
    assert!(!buffer.can_undo());

    // Redo
    let redo_entry = buffer.redo().unwrap();
    assert_eq!(redo_entry.description, "Delete job 123");
    assert!(!buffer.can_redo());
}

/// Test that new operations clear the redo stack
#[test]
fn test_new_operation_clears_redo_stack() {
    let mut buffer = UndoBuffer::new();

    // Push and undo an operation
    buffer.push(
        UndoableOperation::DeleteJob {
            sid: "1".to_string(),
        },
        "Delete 1".to_string(),
    );
    buffer.undo();
    assert!(buffer.can_redo());

    // Push a new operation - should clear redo stack
    buffer.push(
        UndoableOperation::DeleteJob {
            sid: "2".to_string(),
        },
        "Delete 2".to_string(),
    );
    assert!(!buffer.can_redo());
}

/// Test undo buffer maximum history limit
#[test]
fn test_undo_buffer_max_history() {
    let mut buffer = UndoBuffer::new();

    // Fill beyond max (50)
    for i in 0..60 {
        let op = UndoableOperation::DeleteIndex {
            name: format!("idx{}", i),
            original_settings: None,
        };
        buffer.push(op, format!("Delete {}", i));
    }

    // Should only keep MAX_UNDO_HISTORY (50)
    assert_eq!(buffer.history().len(), 50);
}

/// Test entry status transitions
#[test]
fn test_entry_status() {
    let op = UndoableOperation::DeleteIndex {
        name: "test".to_string(),
        original_settings: None,
    };

    let mut entry = splunk_tui::undo::UndoEntry::new(op, "Test".to_string());

    // Fresh entry should be pending
    assert_eq!(entry.status(), UndoEntryStatus::Pending);
    assert!(!entry.is_expired()); // Should not be expired yet

    // Mark as executed
    entry.executed = true;
    assert_eq!(entry.status(), UndoEntryStatus::Executed);

    // Mark as undone (overrides executed for display)
    entry.undone = true;
    assert_eq!(entry.status(), UndoEntryStatus::Undone);

    // Reset and expire
    entry.executed = false;
    entry.undone = false;
    entry.created_at = std::time::Instant::now() - Duration::from_secs(60);
    assert_eq!(entry.status(), UndoEntryStatus::Expired);
}

/// Test remaining time calculation
#[test]
fn test_remaining_time() {
    let op = UndoableOperation::DeleteIndex {
        name: "test".to_string(),
        original_settings: None,
    };

    let entry = splunk_tui::undo::UndoEntry::new(op, "Test".to_string());

    // Should have approximately 30 seconds remaining
    let remaining = entry.remaining();
    assert!(remaining.as_secs() <= 30);
    assert!(remaining.as_secs() >= 29); // Allow for test execution time

    // remaining_secs should match
    assert_eq!(entry.remaining_secs(), remaining.as_secs());
}

/// Test various operation types
#[test]
fn test_undoable_operation_variants() {
    // DeleteIndex
    let op1 = UndoableOperation::DeleteIndex {
        name: "idx".to_string(),
        original_settings: Some(splunk_tui::undo::IndexSettings {
            max_data_size_mb: Some(100),
            max_hot_buckets: Some(10),
            max_warm_db_count: Some(100),
            frozen_time_period_secs: Some(86400),
            home_path: Some("/path".to_string()),
            cold_db_path: None,
            thawed_path: None,
            cold_to_frozen_dir: None,
        }),
    };
    assert!(matches!(op1, UndoableOperation::DeleteIndex { .. }));

    // DeleteJob
    let op2 = UndoableOperation::DeleteJob {
        sid: "123".to_string(),
    };
    assert!(matches!(op2, UndoableOperation::DeleteJob { sid } if sid == "123"));

    // DeleteSavedSearch
    let op3 = UndoableOperation::DeleteSavedSearch {
        name: "search".to_string(),
        original: Some(splunk_tui::undo::SavedSearchRecoveryData {
            search: "index=main".to_string(),
            description: Some("desc".to_string()),
            disabled: false,
        }),
    };
    assert!(matches!(op3, UndoableOperation::DeleteSavedSearch { .. }));

    // DeleteUser
    let op4 = UndoableOperation::DeleteUser {
        name: "user".to_string(),
        original: Some(splunk_tui::undo::UserRecoveryData {
            roles: vec!["admin".to_string()],
            realname: Some("Real Name".to_string()),
            email: Some("user@example.com".to_string()),
            default_app: Some("search".to_string()),
        }),
    };
    assert!(matches!(op4, UndoableOperation::DeleteUser { .. }));

    // ModifyIndex
    let op5 = UndoableOperation::ModifyIndex {
        name: "idx".to_string(),
        original_params: ModifyIndexParams::default(),
        new_params: ModifyIndexParams {
            max_data_size_mb: Some(200),
            ..Default::default()
        },
    };
    assert!(matches!(op5, UndoableOperation::ModifyIndex { .. }));

    // DeleteJobsBatch
    let op6 = UndoableOperation::DeleteJobsBatch {
        sids: vec!["1".to_string(), "2".to_string()],
    };
    assert!(matches!(op6, UndoableOperation::DeleteJobsBatch { sids } if sids.len() == 2));
}

/// Test ModifyIndexParams default
#[test]
fn test_modify_index_params_default() {
    let params = ModifyIndexParams::default();
    assert!(params.max_data_size_mb.is_none());
    assert!(params.max_hot_buckets.is_none());
    assert!(params.max_warm_db_count.is_none());
    assert!(params.frozen_time_period_secs.is_none());
    assert!(params.home_path.is_none());
    assert!(params.cold_db_path.is_none());
    assert!(params.thawed_path.is_none());
    assert!(params.cold_to_frozen_dir.is_none());
}

/// Test that pending count updates correctly
#[test]
fn test_pending_count_updates() {
    let mut buffer = UndoBuffer::new();

    assert_eq!(buffer.pending_count(), 0);

    // Add pending operation
    buffer.push(
        UndoableOperation::DeleteJob {
            sid: "1".to_string(),
        },
        "Delete 1".to_string(),
    );
    assert_eq!(buffer.pending_count(), 1);

    // Add another
    buffer.push(
        UndoableOperation::DeleteJob {
            sid: "2".to_string(),
        },
        "Delete 2".to_string(),
    );
    assert_eq!(buffer.pending_count(), 2);

    // Undo one - pending count should decrease
    buffer.undo();
    assert_eq!(buffer.pending_count(), 1);

    // Mark remaining as executed
    if let Some(entry) = buffer.peek_pending() {
        let id = entry.id;
        buffer.mark_executed(id);
    }
    assert_eq!(buffer.pending_count(), 0);
}

/// Test undo buffer behavior with empty buffer
#[test]
fn test_undo_buffer_behavior() {
    let mut buffer = UndoBuffer::new();

    // Test empty buffer
    assert!(!buffer.can_undo());
    assert!(!buffer.can_redo());
    assert_eq!(buffer.pending_count(), 0);
    assert!(buffer.undo().is_none());
    assert!(buffer.redo().is_none());

    // Add an operation
    let id = buffer.push(
        UndoableOperation::DeleteJob {
            sid: "test".to_string(),
        },
        "Delete test".to_string(),
    );

    // Should be undoable
    assert!(buffer.can_undo());
    assert_eq!(buffer.pending_count(), 1);

    // Peek pending should return the entry
    let pending = buffer.peek_pending().unwrap();
    assert_eq!(pending.id, id);
    assert_eq!(pending.description, "Delete test");

    // Undo it
    let undone = buffer.undo().unwrap();
    assert_eq!(undone.id, id);
    assert!(undone.undone);

    // Now should be redoable
    assert!(buffer.can_redo());
    assert!(!buffer.can_undo());

    // Redo
    let redone = buffer.redo().unwrap();
    assert_eq!(redone.id, id);

    // Back to undoable
    assert!(buffer.can_undo());
    assert!(!buffer.can_redo());
}

/// Test remaining time decreases
#[test]
fn test_remaining_time_decreases() {
    let op = UndoableOperation::DeleteIndex {
        name: "test".to_string(),
        original_settings: None,
    };

    let entry = splunk_tui::undo::UndoEntry::new(op, "Test".to_string());

    // Initial remaining should be ~30 seconds
    let initial = entry.remaining();
    assert!(initial.as_secs() >= 29);

    // Wait a tiny bit
    std::thread::sleep(Duration::from_millis(50));

    // Remaining should have decreased
    let later = entry.remaining();
    assert!(later <= initial);
}

/// Test that clear_expired removes only expired pending entries
#[test]
fn test_clear_expired_behavior() {
    let mut buffer = UndoBuffer::new();

    // Add an operation
    buffer.push(
        UndoableOperation::DeleteJob {
            sid: "test".to_string(),
        },
        "Test".to_string(),
    );

    // Buffer should have 1 entry
    assert_eq!(buffer.history().len(), 1);

    // clear_expired shouldn't remove non-expired pending entries
    buffer.clear_expired();
    assert_eq!(buffer.history().len(), 1);

    // Manually expire by manipulating the entry through the public API
    // This is tricky - we'll use peek_pending_mut to modify the entry
    if let Some(entry) = buffer.peek_pending_mut() {
        // Set created_at to the past to simulate expiration
        entry.created_at = std::time::Instant::now() - Duration::from_secs(60);
    }

    // Now clear_expired should remove it
    buffer.clear_expired();
    assert_eq!(buffer.history().len(), 0);
}

/// Test all UndoableOperation variants are constructible
#[test]
fn test_all_operation_variants() {
    // Test all variants to ensure they're properly defined
    let _ = UndoableOperation::DeleteIndex {
        name: "idx".to_string(),
        original_settings: None,
    };

    let _ = UndoableOperation::DeleteSavedSearch {
        name: "search".to_string(),
        original: None,
    };

    let _ = UndoableOperation::DeleteJob {
        sid: "sid".to_string(),
    };

    let _ = UndoableOperation::DeleteLookup {
        name: "lookup".to_string(),
        app: Some("app".to_string()),
        owner: Some("admin".to_string()),
    };

    let _ = UndoableOperation::ModifyIndex {
        name: "idx".to_string(),
        original_params: ModifyIndexParams::default(),
        new_params: ModifyIndexParams::default(),
    };

    let _ = UndoableOperation::DeleteUser {
        name: "user".to_string(),
        original: None,
    };

    let _ = UndoableOperation::DeleteRole {
        name: "role".to_string(),
    };

    let _ = UndoableOperation::RemoveApp {
        app_name: "app".to_string(),
    };

    let _ = UndoableOperation::DeleteProfile {
        name: "profile".to_string(),
        original: None,
    };

    let _ = UndoableOperation::CancelJob {
        sid: "sid".to_string(),
        search_query: Some("search".to_string()),
    };

    let _ = UndoableOperation::DeleteJobsBatch {
        sids: vec!["1".to_string(), "2".to_string()],
    };

    let _ = UndoableOperation::CancelJobsBatch {
        sids: vec!["1".to_string()],
    };
}

/// Test recovery data structures
#[test]
fn test_recovery_data_structures() {
    // SavedSearchRecoveryData
    let saved_search = splunk_tui::undo::SavedSearchRecoveryData {
        search: "index=main | stats count".to_string(),
        description: Some("Count events".to_string()),
        disabled: false,
    };
    assert!(!saved_search.disabled);

    // UserRecoveryData
    let user = splunk_tui::undo::UserRecoveryData {
        roles: vec!["admin".to_string(), "power".to_string()],
        realname: Some("Admin User".to_string()),
        email: Some("admin@example.com".to_string()),
        default_app: Some("search".to_string()),
    };
    assert_eq!(user.roles.len(), 2);

    // ProfileRecoveryData
    let profile = splunk_tui::undo::ProfileRecoveryData {
        base_url: "https://localhost:8089".to_string(),
        username: "admin".to_string(),
        skip_verify: true,
        timeout_seconds: 30,
        max_retries: 3,
        use_keyring: false,
    };
    assert!(profile.skip_verify);

    // IndexSettings
    let settings = splunk_tui::undo::IndexSettings {
        max_data_size_mb: Some(1000),
        max_hot_buckets: Some(10),
        max_warm_db_count: Some(300),
        frozen_time_period_secs: Some(86400),
        home_path: Some("/opt/splunk/var/lib/splunk/defaultdb/db".to_string()),
        cold_db_path: None,
        thawed_path: None,
        cold_to_frozen_dir: None,
    };
    assert_eq!(settings.max_data_size_mb, Some(1000));
}
