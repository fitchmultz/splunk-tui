//! Bootstrap mode startup integration tests
//!
//! Tests for verifying the pre-auth bootstrap mode behavior (RQ-0454):
//! - Missing-auth startup enters bootstrap mode instead of exiting
//! - In bootstrap mode with first-run conditions, tutorial popup opens
//! - Profile creation from tutorial triggers connection attempt path
//! - Successful connect transitions bootstrap -> main run mode
//! - Failed connect stays in bootstrap and reports failure state

use splunk_tui::runtime::startup::{
    BootstrapReason, StartupDecision, StartupPhase, classify_config_error, classify_startup_error,
    should_launch_tutorial,
};

// ============================================================================
// Startup phase and bootstrap reason tests
// ============================================================================

#[test]
fn test_startup_phase_bootstrap_with_missing_auth() {
    let phase = StartupPhase::Bootstrap {
        reason: BootstrapReason::MissingAuth,
    };
    assert!(matches!(
        phase,
        StartupPhase::Bootstrap {
            reason: BootstrapReason::MissingAuth
        }
    ));
}

#[test]
fn test_startup_phase_bootstrap_with_invalid_auth() {
    let phase = StartupPhase::Bootstrap {
        reason: BootstrapReason::InvalidAuth,
    };
    assert!(matches!(
        phase,
        StartupPhase::Bootstrap {
            reason: BootstrapReason::InvalidAuth
        }
    ));
}

#[test]
fn test_startup_phase_bootstrap_with_profile_not_found() {
    let phase = StartupPhase::Bootstrap {
        reason: BootstrapReason::ProfileNotFound,
    };
    assert!(matches!(
        phase,
        StartupPhase::Bootstrap {
            reason: BootstrapReason::ProfileNotFound
        }
    ));
}

#[test]
fn test_startup_phase_main() {
    let phase = StartupPhase::Main;
    assert_eq!(phase, StartupPhase::Main);
}

#[test]
fn test_startup_phase_connecting() {
    let phase = StartupPhase::Connecting;
    assert_eq!(phase, StartupPhase::Connecting);
}

// ============================================================================
// Bootstrap reason display messages
// ============================================================================

#[test]
fn test_bootstrap_reason_missing_auth_message() {
    let msg = BootstrapReason::MissingAuth.to_string();
    assert!(msg.contains("Authentication"));
    assert!(msg.contains("create a profile"));
}

#[test]
fn test_bootstrap_reason_invalid_auth_message() {
    let msg = BootstrapReason::InvalidAuth.to_string();
    assert!(msg.contains("Authentication"));
    assert!(msg.contains("failed"));
}

#[test]
fn test_bootstrap_reason_profile_not_found_message() {
    let msg = BootstrapReason::ProfileNotFound.to_string();
    assert!(msg.contains("Profile"));
    assert!(msg.contains("not found"));
}

#[test]
fn test_bootstrap_reason_missing_base_url_message() {
    let msg = BootstrapReason::MissingBaseUrl.to_string();
    assert!(msg.contains("URL"));
    assert!(msg.contains("required"));
}

// ============================================================================
// Config error classification tests
// ============================================================================

#[test]
fn test_classify_missing_auth_error() {
    let err = splunk_config::ConfigError::MissingAuth;
    let decision = classify_config_error(&err);

    match decision {
        StartupDecision::EnterBootstrap(BootstrapReason::MissingAuth) => {}
        _ => panic!("Expected EnterBootstrap(MissingAuth), got {:?}", decision),
    }
}

#[test]
fn test_classify_missing_base_url_error() {
    let err = splunk_config::ConfigError::MissingBaseUrl;
    let decision = classify_config_error(&err);

    match decision {
        StartupDecision::EnterBootstrap(BootstrapReason::MissingBaseUrl) => {}
        _ => panic!(
            "Expected EnterBootstrap(MissingBaseUrl), got {:?}",
            decision
        ),
    }
}

#[test]
fn test_classify_profile_not_found_error() {
    let err = splunk_config::ConfigError::ProfileNotFound("test".to_string());
    let decision = classify_config_error(&err);

    match decision {
        StartupDecision::EnterBootstrap(BootstrapReason::ProfileNotFound) => {}
        _ => panic!(
            "Expected EnterBootstrap(ProfileNotFound), got {:?}",
            decision
        ),
    }
}

#[test]
fn test_classify_decryption_failed_error() {
    let err = splunk_config::ConfigError::DecryptionFailed("test".to_string());
    let decision = classify_config_error(&err);

    match decision {
        StartupDecision::EnterBootstrap(BootstrapReason::InvalidAuth) => {}
        _ => panic!("Expected EnterBootstrap(InvalidAuth), got {:?}", decision),
    }
}

// ============================================================================
// Startup error classification tests
// ============================================================================

#[test]
fn test_classify_startup_error_with_auth_keyword() {
    let err = anyhow::anyhow!("Authentication failed: invalid credentials");
    let decision = classify_startup_error(&err);

    match decision {
        StartupDecision::EnterBootstrap(BootstrapReason::MissingAuth) => {}
        _ => panic!("Expected EnterBootstrap(MissingAuth), got {:?}", decision),
    }
}

#[test]
fn test_classify_startup_error_with_credential_keyword() {
    let err = anyhow::anyhow!("Invalid credentials provided");
    let decision = classify_startup_error(&err);

    match decision {
        StartupDecision::EnterBootstrap(BootstrapReason::MissingAuth) => {}
        _ => panic!("Expected EnterBootstrap(MissingAuth), got {:?}", decision),
    }
}

#[test]
fn test_classify_startup_error_with_profile_keyword() {
    let err = anyhow::anyhow!("Profile configuration error");
    let decision = classify_startup_error(&err);

    match decision {
        StartupDecision::EnterBootstrap(BootstrapReason::MissingAuth) => {}
        _ => panic!("Expected EnterBootstrap(MissingAuth), got {:?}", decision),
    }
}

#[test]
fn test_classify_startup_error_unknown() {
    let err = anyhow::anyhow!("Some random error");
    let decision = classify_startup_error(&err);

    match decision {
        StartupDecision::Fatal(_) => {}
        _ => panic!("Expected Fatal, got {:?}", decision),
    }
}

// ============================================================================
// First-run detection with bootstrap context
// ============================================================================

#[test]
fn test_should_launch_tutorial_in_bootstrap_context() {
    // In bootstrap mode with no profiles, first run should be true
    assert!(should_launch_tutorial(true, false, false));
}

#[test]
fn test_should_not_launch_tutorial_when_tutorial_completed() {
    // Even in bootstrap mode, don't show tutorial if already completed
    assert!(!should_launch_tutorial(true, false, true));
}

#[test]
fn test_should_not_launch_tutorial_when_profiles_exist() {
    // Not bootstrap mode - profiles exist
    assert!(!should_launch_tutorial(false, false, false));
}

#[test]
fn test_should_not_launch_tutorial_when_skipped() {
    // User explicitly skipped tutorial
    assert!(!should_launch_tutorial(true, true, false));
}

// ============================================================================
// Bootstrap mode transition tests
// ============================================================================

#[test]
fn test_bootstrap_reason_clone_equality() {
    let reason1 = BootstrapReason::MissingAuth;
    let reason2 = BootstrapReason::MissingAuth;
    assert_eq!(reason1, reason2);

    let reason3 = BootstrapReason::InvalidAuth;
    assert_ne!(reason1, reason3);
}

#[test]
fn test_startup_phase_clone_equality() {
    let phase1 = StartupPhase::Main;
    let phase2 = StartupPhase::Main;
    assert_eq!(phase1, phase2);

    let phase3 = StartupPhase::Bootstrap {
        reason: BootstrapReason::MissingAuth,
    };
    assert_ne!(phase1, phase3);
}

// ============================================================================
// Action classification tests (which actions require client)
// ============================================================================

use splunk_tui::action::Action;
use splunk_tui::runtime::startup::action_requires_client;

#[test]
fn test_action_requires_client_for_api_calls() {
    // API calls require client
    assert!(action_requires_client(&Action::LoadIndexes {
        count: 10,
        offset: 0
    }));
    assert!(action_requires_client(&Action::LoadJobs {
        count: 10,
        offset: 0
    }));
    assert!(action_requires_client(&Action::LoadClusterInfo));
}

#[test]
fn test_action_does_not_require_client_for_system() {
    // System actions don't require client
    assert!(!action_requires_client(&Action::Quit));
    assert!(!action_requires_client(&Action::Tick));
}

#[test]
fn test_action_does_not_require_client_for_tutorial() {
    // Tutorial actions don't require client
    assert!(!action_requires_client(&Action::StartTutorial {
        is_replay: false
    }));
    assert!(!action_requires_client(&Action::TutorialCompleted));
    assert!(!action_requires_client(&Action::TutorialSkipped));
}

#[test]
fn test_action_does_not_require_client_for_profile_management() {
    // Profile management doesn't require client
    assert!(!action_requires_client(&Action::OpenCreateProfileDialog {
        from_tutorial: false
    }));
    assert!(!action_requires_client(&Action::SaveProfile {
        name: "test".to_string(),
        profile: splunk_config::types::ProfileConfig::default(),
        use_keyring: false,
        original_name: None,
        from_tutorial: false,
    }));
}

#[test]
fn test_action_does_not_require_client_for_navigation() {
    // Navigation actions don't require client
    assert!(!action_requires_client(&Action::NextScreen));
    assert!(!action_requires_client(&Action::PreviousScreen));
    assert!(!action_requires_client(&Action::SwitchToSearch));
}

#[test]
fn test_bootstrap_connect_action_variants_exist() {
    // Verify bootstrap action variants exist and can be created
    let _action = Action::BootstrapConnectRequested;
    let _action = Action::BootstrapConnectFinished {
        ok: true,
        error: None,
    };
    let _action = Action::BootstrapConnectFinished {
        ok: false,
        error: Some("Test error".to_string()),
    };
}

// Note: Full integration tests for the bootstrap -> main transition
// require the actual client to be built, which needs valid Splunk
// credentials. These are covered by the tutorial_app_tests which
// test the tutorial flow that triggers bootstrap connection.
