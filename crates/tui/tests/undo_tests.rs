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

/// Regression test: Expired queued operations must execute, not get dropped.
/// This test verifies the fix for RQ-0482 where expired entries were silently
/// dropped instead of being executed because peek_pending() excludes expired items.
#[test]
fn test_expired_operation_executes_not_drops() {
    use std::time::{Duration, Instant};

    let mut buffer = UndoBuffer::new();

    // Queue an operation
    let id = buffer.push(
        UndoableOperation::DeleteJob {
            sid: "test-sid".to_string(),
        },
        "Delete job test-sid".to_string(),
    );

    // Verify it's pending
    assert_eq!(buffer.pending_count(), 1);
    assert!(buffer.peek_pending().is_some());
    assert_eq!(buffer.peek_pending().unwrap().id, id);

    // Expire the entry by manipulating created_at
    if let Some(entry) = buffer.peek_queued_for_execution_mut() {
        entry.created_at = Instant::now() - Duration::from_secs(60);
    }

    // peek_pending should NOT find it (expired) - this is for UI countdown
    assert!(buffer.peek_pending().is_none());
    assert_eq!(buffer.pending_count(), 0);

    // peek_queued_for_execution SHOULD find it (includes expired)
    // This is what the executor uses to find operations to execute
    let queued = buffer.peek_queued_for_execution();
    assert!(
        queued.is_some(),
        "peek_queued_for_execution should find expired entries"
    );
    let queued_entry = queued.unwrap();
    assert_eq!(queued_entry.id, id);
    assert!(queued_entry.is_expired());
    assert!(!queued_entry.executed);
    assert!(!queued_entry.undone);

    // Mark as executed (simulating what process_undo_buffer does)
    buffer.mark_executed(id);

    // After execution, peek_queued_for_execution should not find it
    assert!(buffer.peek_queued_for_execution().is_none());

    // clear_expired should not remove executed entries immediately
    // (clear_expired only removes entries that are old AND executed/undone)
    buffer.clear_expired();
    assert_eq!(buffer.history().len(), 1); // Still there
}

/// Test that non-expired entries are NOT returned by execution accessor
/// when an expired entry exists before them in history.
#[test]
fn test_execution_order_respects_fifo() {
    use std::time::{Duration, Instant};

    let mut buffer = UndoBuffer::new();

    // Add two operations (most recent first due to push_front)
    let id1 = buffer.push(
        UndoableOperation::DeleteJob {
            sid: "1".to_string(),
        },
        "First".to_string(),
    );
    let id2 = buffer.push(
        UndoableOperation::DeleteJob {
            sid: "2".to_string(),
        },
        "Second".to_string(),
    );

    // Expire only id2 (it's at front of history due to push_front being most recent)
    // peek_queued_for_execution_mut returns the first (most recent) entry which is id2
    if let Some(entry) = buffer.peek_queued_for_execution_mut() {
        assert_eq!(entry.id, id2, "First entry should be id2 (most recent)");
        entry.created_at = Instant::now() - Duration::from_secs(60);
    }

    // Should find the expired one (id2) for execution first
    let queued = buffer.peek_queued_for_execution().unwrap();
    assert!(queued.is_expired());
    assert_eq!(queued.id, id2);

    // After executing id2, should find id1 (non-expired)
    buffer.mark_executed(id2);
    let next = buffer.peek_queued_for_execution().unwrap();
    assert!(!next.is_expired());
    assert_eq!(next.id, id1);
}

/// Test that peek_queued_for_execution_mut allows modifying expired entries
#[test]
fn test_peek_queued_for_execution_mut() {
    use std::time::{Duration, Instant};

    let mut buffer = UndoBuffer::new();

    buffer.push(
        UndoableOperation::DeleteJob {
            sid: "1".to_string(),
        },
        "Test".to_string(),
    );

    // Mutate to expire
    if let Some(entry) = buffer.peek_queued_for_execution_mut() {
        entry.created_at = Instant::now() - Duration::from_secs(60);
    }

    // Verify it's expired
    assert!(buffer.peek_queued_for_execution().unwrap().is_expired());
    assert!(buffer.peek_pending().is_none());
}

/// Test that the UI countdown method (peek_pending) still works correctly
/// after the fix for expired execution.
#[test]
fn test_peek_pending_still_excludes_expired_for_ui() {
    use std::time::{Duration, Instant};

    let mut buffer = UndoBuffer::new();

    // Add a non-expired operation
    let id = buffer.push(
        UndoableOperation::DeleteJob {
            sid: "1".to_string(),
        },
        "Test".to_string(),
    );

    // Should be visible in both methods
    assert!(buffer.peek_pending().is_some());
    assert!(buffer.peek_queued_for_execution().is_some());
    assert_eq!(buffer.peek_pending().unwrap().id, id);

    // Expire it
    if let Some(entry) = buffer.peek_queued_for_execution_mut() {
        entry.created_at = Instant::now() - Duration::from_secs(60);
    }

    // Now peek_pending should exclude it (for UI countdown)
    assert!(buffer.peek_pending().is_none());
    // But peek_queued_for_execution should still include it (for execution)
    assert!(buffer.peek_queued_for_execution().is_some());
}

/// Test that process_undo_buffer executes expired operations.
/// This test satisfies the acceptance criteria:
/// "a regression test queues an undoable delete, forces expiry, runs process_undo_buffer,
/// and observes exactly one corresponding delete action/side effect"
#[test]
fn test_process_undo_buffer_executes_expired() {
    use splunk_tui::{App, ConnectionContext};
    use std::time::{Duration, Instant};

    let mut app = App::new(None, ConnectionContext::default());

    // Queue an undoable delete operation
    let id = app.undo_buffer.push(
        UndoableOperation::DeleteJob {
            sid: "test-sid-expired".to_string(),
        },
        "Delete job test-sid-expired".to_string(),
    );

    // Verify it's pending and matches the ID
    assert_eq!(app.undo_buffer.pending_count(), 1);
    assert_eq!(app.undo_buffer.peek_pending().unwrap().id, id);

    // Force expiry by manipulating created_at
    if let Some(entry) = app.undo_buffer.peek_queued_for_execution_mut() {
        entry.created_at = Instant::now() - Duration::from_secs(60);
    }

    // Verify it's now expired and not visible to peek_pending (UI)
    assert!(app.undo_buffer.peek_pending().is_none());

    // Process the undo buffer - this should execute the expired operation
    app.process_undo_buffer();

    // Verify the operation was executed (marked as executed, removed from pending)
    assert!(app.undo_buffer.peek_queued_for_execution().is_none());

    // Verify toast was shown (success toast for execution)
    let toast_messages: Vec<&str> = app.toasts.iter().map(|t| t.message.as_str()).collect();
    assert!(
        toast_messages
            .iter()
            .any(|m| m.contains("Operation executed")),
        "Expected execution toast, got: {:?}",
        toast_messages
    );
}

/// Test that process_undo_buffer does NOT execute non-expired operations.
#[test]
fn test_process_undo_buffer_skips_non_expired() {
    use splunk_tui::{App, ConnectionContext};

    let mut app = App::new(None, ConnectionContext::default());

    // Queue a non-expired operation
    let id = app.undo_buffer.push(
        UndoableOperation::DeleteJob {
            sid: "test-sid-active".to_string(),
        },
        "Delete job test-sid-active".to_string(),
    );

    // Verify it's pending
    assert_eq!(app.undo_buffer.pending_count(), 1);
    let entry = app.undo_buffer.peek_pending().unwrap();
    assert!(!entry.is_expired());

    // Process the undo buffer - should NOT execute non-expired operation
    app.process_undo_buffer();

    // Verify the operation is still pending (not executed)
    assert_eq!(app.undo_buffer.pending_count(), 1);
    let entry = app.undo_buffer.peek_pending().unwrap();
    assert_eq!(entry.id, id);
    assert!(!entry.executed);

    // Verify NO execution toast was shown
    let toast_messages: Vec<&str> = app.toasts.iter().map(|t| t.message.as_str()).collect();
    assert!(
        !toast_messages
            .iter()
            .any(|m| m.contains("Operation executed")),
        "Expected NO execution toast for non-expired, got: {:?}",
        toast_messages
    );
}

/// Test that older expired entries are executed even when newer non-expired entries exist.
/// This verifies the fix for the edge case where:
/// - Entry B (newer, at front) is not expired
/// - Entry A (older, at back) IS expired
///
/// Expected: A should still be executed.
#[test]
fn test_process_undo_buffer_handles_mixed_expiry() {
    use splunk_tui::{App, ConnectionContext};
    use std::time::{Duration, Instant};

    let mut app = App::new(None, ConnectionContext::default());

    // Queue operation A (will be at back after B is added)
    let id_a = app.undo_buffer.push(
        UndoableOperation::DeleteJob {
            sid: "job-A-older".to_string(),
        },
        "Delete job A (older)".to_string(),
    );

    // Queue operation B (will be at front - most recent)
    let id_b = app.undo_buffer.push(
        UndoableOperation::DeleteJob {
            sid: "job-B-newer".to_string(),
        },
        "Delete job B (newer)".to_string(),
    );

    // Force expire only A (the older one at the back)
    // We need to find it by ID since peek_queued_for_execution returns the first (B)
    for entry in app.undo_buffer.history_mut().iter_mut() {
        if entry.id == id_a {
            entry.created_at = Instant::now() - Duration::from_secs(60);
        }
    }

    // Verify: B is at front and not expired, A is expired
    let front_entry = app.undo_buffer.peek_queued_for_execution().unwrap();
    assert_eq!(front_entry.id, id_b, "B should be at front");
    assert!(!front_entry.is_expired(), "B should not be expired");

    // Find A and verify it's expired
    let entry_a = app
        .undo_buffer
        .history()
        .iter()
        .find(|e| e.id == id_a)
        .unwrap();
    assert!(entry_a.is_expired(), "A should be expired");

    // Process undo buffer - should execute A (expired) but not B (not expired)
    app.process_undo_buffer();

    // Verify A was executed
    let entry_a_after = app
        .undo_buffer
        .history()
        .iter()
        .find(|e| e.id == id_a)
        .unwrap();
    assert!(entry_a_after.executed, "A should be executed");

    // Verify B is still pending (not executed)
    let entry_b_after = app
        .undo_buffer
        .history()
        .iter()
        .find(|e| e.id == id_b)
        .unwrap();
    assert!(!entry_b_after.executed, "B should NOT be executed");
    assert!(!entry_b_after.undone, "B should not be undone");

    // Verify exactly one execution toast
    let execution_toasts: Vec<&str> = app
        .toasts
        .iter()
        .filter(|t| t.message.contains("Operation executed"))
        .map(|t| t.message.as_str())
        .collect();
    assert_eq!(
        execution_toasts.len(),
        1,
        "Expected exactly one execution toast, got: {:?}",
        execution_toasts
    );
}
