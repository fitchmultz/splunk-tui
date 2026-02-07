//! Authentication strategies and session management.

use secrecy::{ExposeSecret, SecretString};
use splunk_config::constants::DEFAULT_EXPIRY_BUFFER_SECS;
use std::time::Instant;

/// Strategy for authenticating with Splunk.
#[derive(Debug, Clone)]
pub enum AuthStrategy {
    /// Username and password authentication.
    /// The client will automatically manage session tokens.
    SessionToken {
        username: String,
        password: SecretString,
    },
    /// API token (bearer token authentication).
    /// This is preferred for automation as it doesn't require session management.
    ApiToken { token: SecretString },
}

/// Manages Splunk session tokens with automatic renewal.
#[derive(Debug)]
pub struct SessionManager {
    auth_strategy: AuthStrategy,
    session_token: Option<SessionToken>,
}

/// Session token with expiry information.
#[derive(Debug, Clone)]
struct SessionToken {
    value: SecretString,
    expires_at: Option<Instant>,
    expiry_buffer_seconds: u64,
}

impl SessionToken {
    fn new(
        value: SecretString,
        ttl_seconds: Option<u64>,
        expiry_buffer_seconds: Option<u64>,
    ) -> Self {
        let expires_at =
            ttl_seconds.map(|ttl| Instant::now() + std::time::Duration::from_secs(ttl));
        Self {
            value,
            expires_at,
            expiry_buffer_seconds: expiry_buffer_seconds.unwrap_or(DEFAULT_EXPIRY_BUFFER_SECS),
        }
    }

    /// Check if the token is past its actual expiry time.
    fn is_expired(&self) -> bool {
        self.expires_at
            .map(|exp| exp.saturating_duration_since(Instant::now()).is_zero())
            .unwrap_or(false)
    }

    /// Check if the token will expire soon (within the buffer window).
    ///
    /// This is used to proactively refresh tokens before they expire,
    /// preventing race conditions where a token expires during an API call.
    fn will_expire_soon(&self) -> bool {
        self.expires_at
            .map(|exp| {
                let buffer = std::time::Duration::from_secs(self.expiry_buffer_seconds);
                // Calculate the effective expiry time (actual expiry minus buffer)
                // If buffer is larger than remaining time, treat as expiring soon
                let now = Instant::now();
                let remaining = exp.saturating_duration_since(now);
                remaining < buffer
            })
            .unwrap_or(false)
    }
}

impl SessionManager {
    /// Create a new session manager with the given auth strategy.
    pub fn new(strategy: AuthStrategy) -> Self {
        Self {
            auth_strategy: strategy,
            session_token: None,
        }
    }

    /// Get the current auth strategy.
    pub fn strategy(&self) -> &AuthStrategy {
        &self.auth_strategy
    }

    /// Check if we're using API token auth (no session management needed).
    pub fn is_api_token(&self) -> bool {
        matches!(self.auth_strategy, AuthStrategy::ApiToken { .. })
    }

    /// Get the bearer token for API requests.
    /// For API token auth, returns the token directly.
    /// For session auth, returns the session token if valid.
    pub fn get_bearer_token(&self) -> Option<&str> {
        match &self.auth_strategy {
            AuthStrategy::ApiToken { token } => Some(token.expose_secret()),
            AuthStrategy::SessionToken { .. } => {
                self.session_token.as_ref().map(|t| t.value.expose_secret())
            }
        }
    }

    /// Set the session token (received from login response).
    ///
    /// # Arguments
    /// * `token` - The session token string
    /// * `ttl_seconds` - Time-to-live in seconds (None means no expiry)
    /// * `expiry_buffer_seconds` - Buffer before expiry to trigger proactive refresh
    ///   (None uses the default of 60 seconds)
    pub fn set_session_token(
        &mut self,
        token: String,
        ttl_seconds: Option<u64>,
        expiry_buffer_seconds: Option<u64>,
    ) {
        self.session_token = Some(SessionToken::new(
            SecretString::new(token.into()),
            ttl_seconds,
            expiry_buffer_seconds,
        ));
    }

    /// Generic helper to check session token state.
    /// Returns false for API token auth, true if no session token exists.
    fn check_session<F>(&self, check: F) -> bool
    where
        F: FnOnce(&SessionToken) -> bool,
    {
        if self.is_api_token() {
            return false;
        }
        self.session_token.as_ref().map(check).unwrap_or(true)
    }

    /// Check if the current session token is expired or will expire soon.
    pub fn is_session_expired(&self) -> bool {
        self.check_session(|t| t.is_expired())
    }

    /// Check if the current session token will expire soon (within buffer).
    ///
    /// Returns false for API token auth (no expiry concerns).
    pub fn session_expires_soon(&self) -> bool {
        self.check_session(|t| t.will_expire_soon())
    }

    /// Clear the current session token (force re-authentication).
    pub fn clear_session(&mut self) {
        self.session_token = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use splunk_config::constants::DEFAULT_SESSION_TTL_SECS;

    #[test]
    fn test_api_token_bypasses_session() {
        let strategy = AuthStrategy::ApiToken {
            token: SecretString::new("test-token".to_string().into()),
        };
        let manager = SessionManager::new(strategy);
        assert!(manager.is_api_token());
        assert_eq!(manager.get_bearer_token(), Some("test-token"));
        assert!(!manager.is_session_expired());
    }

    #[test]
    fn test_session_token_without_ttl() {
        let strategy = AuthStrategy::SessionToken {
            username: "admin".to_string(),
            password: SecretString::new("pass".to_string().into()),
        };
        let mut manager = SessionManager::new(strategy);
        assert!(!manager.is_api_token());
        assert!(manager.get_bearer_token().is_none());
        assert!(manager.is_session_expired());

        manager.set_session_token("session-key".to_string(), None, None);
        assert_eq!(manager.get_bearer_token(), Some("session-key"));
        // Without TTL, session never expires
        assert!(!manager.is_session_expired());
    }

    #[test]
    fn test_session_token_with_ttl() {
        let strategy = AuthStrategy::SessionToken {
            username: "admin".to_string(),
            password: SecretString::new("pass".to_string().into()),
        };
        let mut manager = SessionManager::new(strategy);

        manager.set_session_token("session-key".to_string(), Some(1), None);
        assert!(!manager.is_session_expired());

        // Note: Can't easily test actual expiry in unit test without time manipulation
    }

    // ============================================================================
    // Security-focused tests for secret handling
    // ============================================================================

    /// Test that API token is not exposed in AuthStrategy Debug output.
    #[test]
    fn test_api_token_not_exposed_in_debug() {
        let secret_token = "secret-api-token-12345";
        let strategy = AuthStrategy::ApiToken {
            token: SecretString::new(secret_token.to_string().into()),
        };

        let debug_output = format!("{:?}", strategy);

        // The secret token should NOT appear in debug output
        assert!(
            !debug_output.contains(secret_token),
            "Debug output should not contain the API token secret"
        );

        // But the variant name should be visible
        assert!(debug_output.contains("ApiToken"));
    }

    /// Test that session password is not exposed in AuthStrategy Debug output.
    #[test]
    fn test_session_password_not_exposed_in_debug() {
        let username = "admin";
        let password = "secret-password-45678";

        let strategy = AuthStrategy::SessionToken {
            username: username.to_string(),
            password: SecretString::new(password.to_string().into()),
        };

        let debug_output = format!("{:?}", strategy);

        // The password should NOT appear in debug output
        assert!(
            !debug_output.contains(password),
            "Debug output should not contain the password"
        );

        // But the username SHOULD be visible (it's not a secret)
        assert!(debug_output.contains(username));
    }

    /// Test that SessionManager does not expose tokens in Debug output.
    #[test]
    fn test_session_manager_not_exposed_in_debug() {
        let secret_token = "session-manager-secret-token";
        let strategy = AuthStrategy::ApiToken {
            token: SecretString::new(secret_token.to_string().into()),
        };

        let manager = SessionManager::new(strategy);
        let debug_output = format!("{:?}", manager);

        // The secret token should NOT appear in debug output
        assert!(
            !debug_output.contains(secret_token),
            "Debug output should not contain the token"
        );
    }

    /// Test that session tokens set after login are not exposed in Debug output.
    #[test]
    fn test_set_session_token_not_exposed_in_debug() {
        let strategy = AuthStrategy::SessionToken {
            username: "admin".to_string(),
            password: SecretString::new("password".to_string().into()),
        };

        let mut manager = SessionManager::new(strategy);
        let session_token = "new-session-token-after-login-123";
        manager.set_session_token(
            session_token.to_string(),
            Some(DEFAULT_SESSION_TTL_SECS),
            None,
        );

        let debug_output = format!("{:?}", manager);

        // The session token should NOT appear in debug output
        assert!(
            !debug_output.contains(session_token),
            "Debug output should not contain the session token"
        );
    }

    /// Test that clearing a session doesn't expose the token in Debug output.
    #[test]
    fn test_clear_session_not_exposed_in_debug() {
        let strategy = AuthStrategy::SessionToken {
            username: "admin".to_string(),
            password: SecretString::new("password".to_string().into()),
        };

        let mut manager = SessionManager::new(strategy);
        let session_token = "session-to-be-cleared-456";
        manager.set_session_token(
            session_token.to_string(),
            Some(DEFAULT_SESSION_TTL_SECS),
            None,
        );

        // Clear the session
        manager.clear_session();

        let debug_output = format!("{:?}", manager);

        // Even after clearing, the old token should not appear
        assert!(
            !debug_output.contains(session_token),
            "Debug output should not contain cleared session token"
        );
    }

    /// Test that API token auth never expires while session auth does.
    #[test]
    fn test_api_token_vs_session_expiration() {
        // API token auth - should never expire
        let api_strategy = AuthStrategy::ApiToken {
            token: SecretString::new("api-token".to_string().into()),
        };
        let api_manager = SessionManager::new(api_strategy);
        assert!(!api_manager.is_session_expired());
        assert!(api_manager.is_api_token());

        // Session token auth without setting token - should be expired
        let session_strategy = AuthStrategy::SessionToken {
            username: "admin".to_string(),
            password: SecretString::new("pass".to_string().into()),
        };
        let session_manager = SessionManager::new(session_strategy);
        assert!(session_manager.is_session_expired());
        assert!(!session_manager.is_api_token());
    }

    /// Test that bearer token can be accessed programmatically via ExposeSecret.
    #[test]
    fn test_bearer_token_accessible_via_expose_secret() {
        let secret_token = "bearer-token-secret-789";
        let strategy = AuthStrategy::ApiToken {
            token: SecretString::new(secret_token.to_string().into()),
        };

        let manager = SessionManager::new(strategy);

        // Token should be accessible via get_bearer_token (for API calls)
        let bearer = manager.get_bearer_token();
        assert_eq!(bearer, Some(secret_token));
    }

    /// Test that multiple secrets in SessionManager are all protected.
    #[test]
    fn test_multiple_secrets_protected() {
        let strategy = AuthStrategy::SessionToken {
            username: "admin".to_string(),
            password: SecretString::new("password1".to_string().into()),
        };

        let mut manager = SessionManager::new(strategy);
        manager.set_session_token(
            "session-token-123".to_string(),
            Some(DEFAULT_SESSION_TTL_SECS),
            None,
        );

        let debug_output = format!("{:?}", manager);

        // Neither password nor session token should appear
        assert!(!debug_output.contains("password1"));
        assert!(!debug_output.contains("session-token-123"));
    }

    // ============================================================================
    // Session Expiry Buffer Tests
    // ============================================================================

    /// Test that will_expire_soon() returns false when outside buffer window.
    #[test]
    fn test_will_expire_soon_returns_false_outside_buffer() {
        let strategy = AuthStrategy::SessionToken {
            username: "admin".to_string(),
            password: SecretString::new("pass".to_string().into()),
        };
        let mut manager = SessionManager::new(strategy);

        // Set token with 120s TTL and 60s buffer - expires in 120s, buffer ends at 60s
        manager.set_session_token(
            "session-key".to_string(),
            Some(120),
            Some(DEFAULT_EXPIRY_BUFFER_SECS),
        );

        // Should not expire soon immediately after setting
        assert!(!manager.session_expires_soon());
        assert!(!manager.is_session_expired());
    }

    /// Test that session_expires_soon() returns false for API token auth.
    #[test]
    fn test_session_expires_soon_returns_false_for_api_token() {
        let strategy = AuthStrategy::ApiToken {
            token: SecretString::new("api-token".to_string().into()),
        };
        let manager = SessionManager::new(strategy);

        // API tokens never expire
        assert!(!manager.session_expires_soon());
        assert!(!manager.is_session_expired());
    }

    /// Test that session_expires_soon() returns true when no session token is set.
    #[test]
    fn test_session_expires_soon_returns_true_when_no_token() {
        let strategy = AuthStrategy::SessionToken {
            username: "admin".to_string(),
            password: SecretString::new("pass".to_string().into()),
        };
        let manager = SessionManager::new(strategy);

        // No token set - should be considered expiring soon
        assert!(manager.session_expires_soon());
        assert!(manager.is_session_expired());
    }

    /// Test that buffer of 0 seconds behaves like original is_expired().
    #[test]
    fn test_zero_buffer_behaves_like_is_expired() {
        let strategy = AuthStrategy::SessionToken {
            username: "admin".to_string(),
            password: SecretString::new("pass".to_string().into()),
        };
        let mut manager = SessionManager::new(strategy);

        // Set token with 120s TTL and 0s buffer
        manager.set_session_token("session-key".to_string(), Some(120), Some(0));

        // With 0 buffer, will_expire_soon should behave like is_expired
        assert!(!manager.session_expires_soon());
        assert!(!manager.is_session_expired());
    }

    /// Test that will_expire_soon returns true when within buffer window.
    /// Note: This test uses a very short TTL to simulate expiry within the test.
    #[test]
    fn test_will_expire_soon_returns_true_within_buffer() {
        // Create a session token directly with a very short TTL
        let token = SessionToken::new(
            SecretString::new("test-token".to_string().into()),
            Some(1), // 1 second TTL
            Some(2), // 2 second buffer (larger than TTL, so it expires immediately)
        );

        // Should be considered expiring soon because buffer > remaining time
        assert!(token.will_expire_soon());
    }

    /// Test that default buffer is applied when not specified.
    #[test]
    fn test_default_buffer_applied() {
        let token = SessionToken::new(
            SecretString::new("test-token".to_string().into()),
            Some(DEFAULT_SESSION_TTL_SECS),
            None, // Use default
        );

        // Should have the default buffer (60 seconds)
        assert!(!token.is_expired());
        assert!(!token.will_expire_soon());
    }
}
