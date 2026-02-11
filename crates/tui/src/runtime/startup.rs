//! Startup phase management for bootstrap mode.
//!
//! Responsibilities:
//! - Define startup phases (Bootstrap, Connecting, Main, Fatal).
//! - Classify startup errors as recoverable (bootstrap) vs fatal.
//! - Coordinate transition from bootstrap to main run mode.
//!
//! Does NOT handle:
//! - UI rendering (handled by app).
//! - Client authentication (delegates to client crate).
//! - Configuration persistence (delegates to config crate).
//!
//! Invariants:
//! - Startup phases generally advance forward: Bootstrap → Connecting → Main.
//! - Backward transition allowed: Connecting → Bootstrap on connection failure.
//! - Bootstrap mode exits through successful connection, fatal error, or app quit.

use anyhow::Error;
use splunk_config::ConfigError;

/// Current phase of application startup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupPhase {
    /// Initial bootstrap mode - no authenticated client yet.
    /// App can render but API calls are disabled.
    Bootstrap { reason: BootstrapReason },

    /// Actively attempting to connect/authenticate.
    /// UI should show loading state.
    Connecting,

    /// Normal operation with authenticated client.
    /// All features available.
    Main,

    /// Fatal error occurred, application must exit.
    Fatal,
}

/// Reason for entering bootstrap mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootstrapReason {
    /// No authentication configured (missing username/password or API token).
    MissingAuth,

    /// Authentication configuration present but invalid/expired.
    InvalidAuth,

    /// Requested profile not found in config.
    ProfileNotFound,

    /// User explicitly requested fresh start without credentials.
    ExplicitFreshStart,

    /// Base URL not configured.
    MissingBaseUrl,
}

impl std::fmt::Display for BootstrapReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BootstrapReason::MissingAuth => {
                write!(f, "Authentication required. Please create a profile.")
            }
            BootstrapReason::InvalidAuth => {
                write!(f, "Authentication failed. Please check your credentials.")
            }
            BootstrapReason::ProfileNotFound => {
                write!(f, "Profile not found. Please create a profile.")
            }
            BootstrapReason::ExplicitFreshStart => {
                write!(f, "Fresh start requested. Please create a profile.")
            }
            BootstrapReason::MissingBaseUrl => {
                write!(f, "Server URL required. Please create a profile.")
            }
        }
    }
}

/// Decision made during startup initialization.
#[derive(Debug)]
pub enum StartupDecision {
    /// Enter bootstrap mode due to recoverable auth/config issue.
    EnterBootstrap(BootstrapReason),

    /// Continue with normal startup using provided client config.
    ContinueWithConfig,

    /// Fatal error - application cannot start.
    Fatal(Error),
}

/// Classify a configuration error to determine startup behavior.
///
/// This function determines whether a config error should:
/// - Enter bootstrap mode (recoverable auth/config issues)
/// - Be treated as fatal (system errors, IO failures, etc.)
pub fn classify_config_error(err: &ConfigError) -> StartupDecision {
    match err {
        // Recoverable auth/config errors - enter bootstrap mode
        ConfigError::MissingAuth => StartupDecision::EnterBootstrap(BootstrapReason::MissingAuth),
        ConfigError::MissingBaseUrl => {
            StartupDecision::EnterBootstrap(BootstrapReason::MissingBaseUrl)
        }
        ConfigError::ProfileNotFound(_) => {
            StartupDecision::EnterBootstrap(BootstrapReason::ProfileNotFound)
        }

        // Potentially recoverable - depends on context
        ConfigError::DecryptionFailed(_) => {
            StartupDecision::EnterBootstrap(BootstrapReason::InvalidAuth)
        }
        ConfigError::Keyring(_) => StartupDecision::EnterBootstrap(BootstrapReason::InvalidAuth),

        // Fatal errors - system issues that prevent startup
        ConfigError::MissingEnvVar(_) => StartupDecision::Fatal(Error::msg(err.to_string())),
        ConfigError::InvalidValue { .. } => StartupDecision::Fatal(Error::msg(err.to_string())),
        ConfigError::ConfigDirUnavailable(_) => StartupDecision::Fatal(Error::msg(err.to_string())),
        ConfigError::ConfigFileRead { .. } => StartupDecision::Fatal(Error::msg(err.to_string())),
        ConfigError::ConfigFileParse { .. } => StartupDecision::Fatal(Error::msg(err.to_string())),
        ConfigError::InvalidTimeout { .. } => StartupDecision::Fatal(Error::msg(err.to_string())),
        ConfigError::InvalidSessionTtl { .. } => {
            StartupDecision::Fatal(Error::msg(err.to_string()))
        }
        ConfigError::InvalidHealthCheckInterval { .. } => {
            StartupDecision::Fatal(Error::msg(err.to_string()))
        }
        ConfigError::InvalidMaxRetries { .. } => {
            StartupDecision::Fatal(Error::msg(err.to_string()))
        }
        ConfigError::DotenvParse { .. } => StartupDecision::Fatal(Error::msg(err.to_string())),
        ConfigError::DotenvIo { .. } => StartupDecision::Fatal(Error::msg(err.to_string())),
        ConfigError::DotenvUnknown => StartupDecision::Fatal(Error::msg(err.to_string())),
        ConfigError::Io(e) => StartupDecision::Fatal(Error::msg(e.to_string())),
    }
}

/// Classify an anyhow error that may wrap a ConfigError.
pub fn classify_startup_error(err: &Error) -> StartupDecision {
    // Check error message for common patterns
    let msg = err.to_string().to_lowercase();
    if msg.contains("auth")
        || msg.contains("credential")
        || msg.contains("profile")
        || msg.contains("login")
    {
        return StartupDecision::EnterBootstrap(BootstrapReason::MissingAuth);
    }

    // Default to fatal for unknown errors
    StartupDecision::Fatal(Error::msg(err.to_string()))
}

/// Result of bootstrap initialization.
///
/// Contains all state needed to start the app in bootstrap or main mode.
#[derive(Debug)]
pub struct BootstrapResult {
    /// Current startup phase
    pub phase: StartupPhase,

    /// Whether the tutorial should be shown on startup
    pub should_launch_tutorial: bool,

    /// Optional bootstrap reason message for UI display
    pub bootstrap_message: Option<String>,
}

/// Determine if the tutorial should be launched.
///
/// This is a shared helper used by both startup code and tests.
///
/// # Arguments
///
/// * `profiles_empty` - Whether no profiles exist yet
/// * `skip_tutorial` - Whether --skip-tutorial flag was passed
/// * `tutorial_completed` - Whether the tutorial was previously completed
pub fn should_launch_tutorial(
    profiles_empty: bool,
    skip_tutorial: bool,
    tutorial_completed: bool,
) -> bool {
    profiles_empty && !skip_tutorial && !tutorial_completed
}

/// Check if an action requires an authenticated client.
///
/// Returns false for actions that can be handled in bootstrap mode.
pub fn action_requires_client(action: &crate::action::Action) -> bool {
    use crate::action::Action;

    match action {
        // System actions - always allowed
        Action::Quit => false,
        Action::Tick => false,
        Action::Input(_) => false,
        Action::Mouse(_) => false,
        Action::Resize(_, _) => false,
        Action::Loading(_) => false,
        Action::Notify(_, _) => false,

        // Tutorial actions - allowed in bootstrap
        Action::StartTutorial { .. } => false,
        Action::TutorialProfileCreated { .. } => false,
        Action::TutorialConnectionResult { .. } => false,
        Action::TutorialCompleted => false,
        Action::TutorialSkipped => false,
        Action::LoadSearchScreenForTutorial => false,

        // Profile management - allowed in bootstrap
        Action::OpenCreateProfileDialog { .. } => false,
        Action::OpenEditProfileDialog { .. } => false,
        Action::OpenEditProfileDialogWithData { .. } => false,
        Action::OpenDeleteProfileConfirm { .. } => false,
        Action::SaveProfile { .. } => false,
        Action::DeleteProfile { .. } => false,
        Action::ProfileSaved(_) => false,
        Action::ProfileDeleted(_) => false,

        // Settings - allowed in bootstrap
        Action::SwitchToSettingsScreen => false,
        Action::SwitchToSettings => false,
        Action::SettingsLoaded(_) => false,
        Action::CycleTheme => false,

        // Navigation - allowed in bootstrap (though screens will be empty)
        Action::NextScreen => false,
        Action::PreviousScreen => false,
        Action::SwitchToSearch => false,
        Action::OpenCommandPalette => false,
        Action::OpenHelpPopup => false,
        Action::SetFocus(_) => false,
        Action::NextFocus => false,
        Action::PreviousFocus => false,
        Action::ToggleFocusMode => false,

        // UI state - allowed in bootstrap
        Action::PersistState => false,
        Action::ShowErrorDetails(_) => false,
        Action::ShowErrorDetailsFromCurrent => false,
        Action::ClearErrorDetails => false,
        Action::Progress(_) => false,
        Action::CopyToClipboard(_) => false,

        // All other actions require a client
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_launch_tutorial_all_conditions() {
        // First run: empty profiles, not skipped, not completed
        assert!(should_launch_tutorial(true, false, false));

        // Not first run: profiles exist
        assert!(!should_launch_tutorial(false, false, false));

        // Not first run: skipped
        assert!(!should_launch_tutorial(true, true, false));

        // Not first run: already completed
        assert!(!should_launch_tutorial(true, false, true));

        // Not first run: all conditions
        assert!(!should_launch_tutorial(false, true, true));
    }

    #[test]
    fn test_classify_missing_auth_error() {
        let err = ConfigError::MissingAuth;
        let decision = classify_config_error(&err);

        match decision {
            StartupDecision::EnterBootstrap(BootstrapReason::MissingAuth) => {}
            _ => panic!("Expected EnterBootstrap(MissingAuth), got {:?}", decision),
        }
    }

    #[test]
    fn test_classify_missing_base_url_error() {
        let err = ConfigError::MissingBaseUrl;
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
        let err = ConfigError::ProfileNotFound("test".to_string());
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
    fn test_bootstrap_reason_display_messages() {
        assert!(
            BootstrapReason::MissingAuth
                .to_string()
                .contains("Authentication")
        );
        assert!(BootstrapReason::InvalidAuth.to_string().contains("failed"));
        assert!(
            BootstrapReason::ProfileNotFound
                .to_string()
                .contains("not found")
        );
        assert!(
            BootstrapReason::ExplicitFreshStart
                .to_string()
                .contains("Fresh start")
        );
        assert!(BootstrapReason::MissingBaseUrl.to_string().contains("URL"));
    }
}
