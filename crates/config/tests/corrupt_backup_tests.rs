//! Regression tests for corrupt config backup behavior.
//!
//! These tests verify the backup-and-default fallback mechanism cannot regress.
//! See RQ-0490 for requirements.

use secrecy::SecretString;
use std::io::Write;

/// Test that ConfigManager returns defaults when backup creation fails.
///
/// Scenario: Config file is corrupt AND parent directory is read-only,
/// preventing backup creation via rename.
///
/// Expected behavior:
/// - ConfigManager::new_with_path returns Ok (not Err)
/// - Manager has empty profiles (defaults)
/// - Manager.load() returns default PersistedState
/// - Original corrupt file remains on disk (not deleted)
#[cfg(unix)]
#[test]
fn test_corrupt_config_backup_failure_continues_with_defaults() {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.json");

    // Create a corrupt config file
    let mut file = std::fs::File::create(&config_path).unwrap();
    file.write_all(b"{ invalid json }").unwrap();
    drop(file);

    // Make parent directory read-only to cause backup rename to fail
    // Note: We need to make the DIRECTORY read-only, not the file
    // The rename operation requires write permission on the directory
    std::fs::set_permissions(temp_dir.path(), std::fs::Permissions::from_mode(0o555)).unwrap();

    // ConfigManager should still succeed with defaults
    let manager = splunk_config::ConfigManager::new_with_path(config_path.clone()).unwrap();

    // Verify defaults are used
    assert!(
        manager.list_profiles().is_empty(),
        "Should have no profiles (defaults)"
    );
    let state = manager.load();
    assert!(!state.auto_refresh, "Should use default auto_refresh=false");

    // Cleanup: restore write permission before temp_dir drops
    std::fs::set_permissions(temp_dir.path(), std::fs::Permissions::from_mode(0o755)).unwrap();

    // Verify original corrupt file still exists (wasn't deleted)
    assert!(
        config_path.exists(),
        "Corrupt file should still exist when backup fails"
    );
    let content = std::fs::read_to_string(&config_path).unwrap();
    assert_eq!(
        content, "{ invalid json }",
        "Corrupt content should be preserved"
    );
}

/// Test that the original corrupt file is NOT deleted when backup fails.
///
/// This is a critical safety property: even if backup fails, we must not
/// lose the original corrupt file (which may contain partially recoverable data).
#[cfg(unix)]
#[test]
fn test_corrupt_config_backup_failure_preserves_original() {
    use std::os::unix::fs::PermissionsExt;

    let temp_dir = tempfile::tempdir().unwrap();
    let subdir = temp_dir.path().join("readonly");
    std::fs::create_dir(&subdir).unwrap();

    let config_path = subdir.join("config.json");

    // Create corrupt config
    std::fs::write(&config_path, b"{ corrupt }").unwrap();

    // Make subdirectory read-only
    std::fs::set_permissions(&subdir, std::fs::Permissions::from_mode(0o555)).unwrap();

    // Attempt to create manager (backup will fail)
    let _manager = splunk_config::ConfigManager::new_with_path(config_path.clone()).unwrap();

    // Cleanup permission
    std::fs::set_permissions(&subdir, std::fs::Permissions::from_mode(0o755)).unwrap();

    // Original file MUST still exist
    assert!(
        config_path.exists(),
        "Original corrupt file must be preserved on backup failure"
    );
    assert_eq!(
        std::fs::read_to_string(&config_path).unwrap(),
        "{ corrupt }",
        "Original content must be preserved"
    );
}

/// Test that successful backup uses rename (atomic), not copy.
///
/// This verifies the backup is created by renaming the original file,
/// meaning the original path no longer exists after successful backup.
#[test]
fn test_corrupt_config_backup_success_removes_original() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.json");

    // Create corrupt config
    std::fs::write(&config_path, b"{ bad }").unwrap();

    // Create manager - backup should succeed
    let _manager = splunk_config::ConfigManager::new_with_path(config_path.clone()).unwrap();

    // Original path should NOT exist (was renamed to backup)
    assert!(
        !config_path.exists(),
        "Original path should not exist after successful backup"
    );

    // Backup file should exist
    let backup_files: Vec<_> = std::fs::read_dir(temp_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_string_lossy()
                .starts_with("config.corrupt.")
        })
        .collect();

    assert_eq!(backup_files.len(), 1, "Should have exactly one backup file");
}

/// Comprehensive regression test for all backup semantics.
///
/// This test asserts all critical behaviors in one place to catch regressions:
/// 1. Corrupt config triggers backup
/// 2. Backup filename format is correct
/// 3. Fallback to defaults occurs
/// 4. New config can be saved after recovery
#[test]
fn test_corrupt_config_comprehensive_regression() {
    use splunk_config::encryption::MasterKeySource;

    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.json");
    let password = SecretString::new("test-master-password".to_string().into());
    let master_key_source = MasterKeySource::Password(password.clone());

    // Phase 1: Create corrupt config
    std::fs::write(&config_path, b"{ not valid json }").unwrap();

    // Phase 2: Initialize manager with password-based master key - should trigger backup
    let mut manager = splunk_config::ConfigManager::new_with_path_and_source(
        config_path.clone(),
        master_key_source.clone(),
    )
    .unwrap();

    // Assert: defaults are used
    assert!(manager.list_profiles().is_empty());

    // Assert: backup was created with correct format
    let backup_files: Vec<_> = std::fs::read_dir(temp_dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            name.starts_with("config.corrupt.") && name.matches('.').count() == 2
        })
        .collect();

    assert_eq!(
        backup_files.len(),
        1,
        "Expected exactly one backup file with format config.corrupt.TIMESTAMP"
    );

    // Assert: backup contains original corrupt content
    let backup_content = std::fs::read_to_string(backup_files[0].path()).unwrap();
    assert_eq!(backup_content, "{ not valid json }");

    // Phase 3: Save new config
    let profile = splunk_config::ProfileConfig {
        base_url: Some("https://recovered.splunk.com:8089".to_string()),
        ..Default::default()
    };
    manager.save_profile("recovered", profile).unwrap();

    // Phase 4: Reload with same master key source and verify persistence
    let reloaded = splunk_config::ConfigManager::new_with_path_and_source(
        config_path.clone(),
        master_key_source,
    )
    .unwrap();
    assert_eq!(reloaded.list_profiles().len(), 1);
    assert!(reloaded.list_profiles().contains_key("recovered"));

    // Assert: original backup still exists
    assert_eq!(
        std::fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e
                .file_name()
                .to_string_lossy()
                .starts_with("config.corrupt."))
            .count(),
        1,
        "Backup should still exist after recovery"
    );
}
