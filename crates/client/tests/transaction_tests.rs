//! Purpose: Validate transaction manager persistence and validation behavior.
//! Responsibilities: Ensure staged transaction files round-trip correctly and input validation enforces invariants.
//! Non-scope: Does not exercise live Splunk API side effects for commit/rollback execution.
//! Invariants/Assumptions: Tests are hermetic and use tempfile-backed directories only.

use secrecy::SecretString;
use splunk_client::transaction::{Transaction, TransactionManager, TransactionOperation};
use splunk_client::{AuthStrategy, CreateIndexParams, SplunkClient};

fn build_test_client() -> SplunkClient {
    SplunkClient::builder()
        .base_url("https://localhost:8089".to_string())
        .auth_strategy(AuthStrategy::ApiToken {
            token: SecretString::new("test-token".to_string().into()),
        })
        .build()
        .expect("Failed to build test client")
}

#[tokio::test]
async fn load_pending_returns_none_when_file_is_missing() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let manager = TransactionManager::new(temp_dir.path().to_path_buf());

    let pending = manager
        .load_pending()
        .await
        .expect("Failed to load pending transaction");

    assert!(pending.is_none(), "Expected no pending transaction");
}

#[tokio::test]
async fn save_and_load_pending_round_trip() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let manager = TransactionManager::new(temp_dir.path().to_path_buf());

    let mut transaction = Transaction::new();
    transaction.add_operation(TransactionOperation::CreateIndex(CreateIndexParams {
        name: "audit_test_index".to_string(),
        ..Default::default()
    }));

    manager
        .save_pending(&transaction)
        .await
        .expect("Failed to save pending transaction");

    let loaded = manager
        .load_pending()
        .await
        .expect("Failed to load pending transaction")
        .expect("Expected saved transaction");

    assert_eq!(loaded.id, transaction.id);
    assert_eq!(loaded.operations.len(), 1);
}

#[tokio::test]
async fn clear_pending_removes_saved_file() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let manager = TransactionManager::new(temp_dir.path().to_path_buf());

    let transaction = Transaction::new();
    manager
        .save_pending(&transaction)
        .await
        .expect("Failed to save pending transaction");

    manager
        .clear_pending()
        .await
        .expect("Failed to clear pending transaction");

    let loaded = manager
        .load_pending()
        .await
        .expect("Failed to load pending transaction after clear");

    assert!(loaded.is_none(), "Pending transaction should be removed");
}

#[tokio::test]
async fn archive_writes_transaction_history_file() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let manager = TransactionManager::new(temp_dir.path().to_path_buf());
    let transaction = Transaction::new();

    manager
        .archive(&transaction, "committed")
        .await
        .expect("Failed to archive transaction");

    let history_dir = temp_dir.path().join("history");
    assert!(history_dir.exists(), "History directory should exist");

    let entries = std::fs::read_dir(&history_dir)
        .expect("Failed to read history directory")
        .collect::<Result<Vec<_>, _>>()
        .expect("Failed to collect history directory entries");

    assert_eq!(entries.len(), 1, "Expected exactly one history entry");
}

#[tokio::test]
async fn validate_rejects_empty_create_index_name() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let manager = TransactionManager::new(temp_dir.path().to_path_buf());
    let client = build_test_client();

    let mut transaction = Transaction::new();
    transaction.add_operation(TransactionOperation::CreateIndex(CreateIndexParams {
        name: String::new(),
        ..Default::default()
    }));

    let err = manager
        .validate(&client, &transaction)
        .await
        .expect_err("Expected validation to fail for empty index name");

    let msg = err.to_string();
    assert!(
        msg.contains("Index name cannot be empty"),
        "Unexpected validation error: {msg}"
    );
}

#[tokio::test]
async fn validate_accepts_non_empty_create_index_name() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let manager = TransactionManager::new(temp_dir.path().to_path_buf());
    let client = build_test_client();

    let mut transaction = Transaction::new();
    transaction.add_operation(TransactionOperation::CreateIndex(CreateIndexParams {
        name: "valid_index".to_string(),
        ..Default::default()
    }));

    manager
        .validate(&client, &transaction)
        .await
        .expect("Validation should succeed for valid operation");
}
