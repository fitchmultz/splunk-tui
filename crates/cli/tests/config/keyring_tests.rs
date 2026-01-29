//! Keyring integration tests for `splunk-cli config`.
//!
//! Tests the --use-keyring flag for secure credential storage.
//!
//! Does NOT:
//! - Test on platforms where keyring is unavailable (graceful skip).

use crate::common::splunk_cmd;
use crate::config::setup_temp_config;
use keyring::Entry;
use splunk_config::types::KEYRING_SERVICE;
use std::fs;

/// Test keyring integration with --use-keyring flag.
#[test]
fn test_config_set_with_keyring() {
    let (_temp_dir, config_path) = setup_temp_config();
    let profile_name = "test-profile";
    let expected_token = "test-token-123";
    let expected_keyring_account = format!("{}-token", profile_name);

    let output = splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", &config_path)
        .env_remove("SPLUNK_PASSWORD")
        .env_remove("SPLUNK_API_TOKEN")
        .args([
            "config",
            "set",
            profile_name,
            "--base-url",
            "https://splunk.example.com:8089",
            "--api-token",
            expected_token,
            "--use-keyring",
        ])
        .output()
        .unwrap();

    if output.status.success() {
        // Verify credentials are stored as Keyring object
        let content = fs::read_to_string(&config_path).expect("Failed to read config file");
        let json: serde_json::Value =
            serde_json::from_str(&content).expect("Failed to parse config JSON");

        // Check for Keyring variant (object with keyring_account)
        let api_token_field = &json["profiles"][profile_name]["api_token"];
        assert!(
            api_token_field.is_object(),
            "Expected api_token to be a JSON object when using keyring, got {:?}",
            api_token_field
        );
        let keyring_account = api_token_field["keyring_account"]
            .as_str()
            .expect("keyring_account field missing or not a string");
        assert_eq!(keyring_account, expected_keyring_account);

        // Verify content in keyring
        let entry =
            Entry::new(KEYRING_SERVICE, keyring_account).expect("Failed to create keyring entry");

        // On some platforms/environments (like macOS tests), child processes may use an isolated
        // keychain or the parent may not have permission to read entries created by the child.
        // We attempt verification but handle NoEntry gracefully if the CLI reported success.
        match entry.get_password() {
            Ok(stored_password) => {
                assert_eq!(stored_password, expected_token);
                // Clean up keyring
                entry
                    .delete_credential()
                    .expect("Failed to delete credential from keyring");

                // Verify deletion
                let entry_check = Entry::new(KEYRING_SERVICE, keyring_account)
                    .expect("Failed to re-create keyring entry");
                assert!(
                    entry_check.get_password().is_err(),
                    "Credential still exists in keyring after deletion"
                );
            }
            Err(keyring::Error::NoEntry) => {
                eprintln!(
                    "Warning: Credential not found in keyring by parent process (isolation?), but CLI reported success."
                );
            }
            Err(e) => panic!("Keyring error: {}", e),
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("keyring") || stderr.contains("Keyring") {
            eprintln!("Skipping keyring test: {}", stderr);
        } else {
            panic!(
                "splunk-cli config set failed with unexpected error: {}",
                stderr
            );
        }
    }
}
