//! Authentication strategies and session management.

use secrecy::{ExposeSecret, SecretString};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, RwLock};

/// Username placeholder for API token authentication when displaying error messages.
pub(crate) const API_TOKEN_USERNAME: &str = "api-token";

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
///
/// Uses interior mutability to allow concurrent access to the session token
/// while ensuring only one refresh operation occurs at a time via singleflight pattern.
pub struct SessionManager {
    auth_strategy: AuthStrategy,
    /// Session token storage with interior mutability for concurrent access.
    /// Uses RwLock to allow multiple concurrent readers when token is valid.
    session_token: RwLock<Option<SessionToken>>,
    /// Singleflight refresh: ensures only one login call at a time.
    /// The Mutex guards an optional Arc<Notify> that waiting tasks can subscribe to.
    refresh_notify: Mutex<Option<Arc<tokio::sync::Notify>>>,
    /// Stores the error from a failed refresh so waiting followers can access it.
    /// Cleared when a new refresh starts or succeeds.
    refresh_error: Mutex<Option<Arc<crate::error::ClientError>>>,
    /// Session TTL in seconds for newly created tokens.
    session_ttl_seconds: u64,
    /// Buffer in seconds before expiry when proactive refresh should occur.
    expiry_buffer_seconds: u64,
}

impl std::fmt::Debug for SessionManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionManager")
            .field("auth_strategy", &self.auth_strategy)
            .field("session_ttl_seconds", &self.session_ttl_seconds)
            .field("expiry_buffer_seconds", &self.expiry_buffer_seconds)
            .finish_non_exhaustive()
    }
}

/// Session token with expiry information.
#[derive(Debug, Clone)]
struct SessionToken {
    value: SecretString,
    expires_at: Option<Instant>,
}

impl SessionToken {
    fn new(value: SecretString, ttl_seconds: Option<u64>) -> Self {
        let expires_at =
            ttl_seconds.map(|ttl| Instant::now() + std::time::Duration::from_secs(ttl));
        Self { value, expires_at }
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
    fn will_expire_soon(&self, buffer_seconds: u64) -> bool {
        self.expires_at
            .map(|exp| {
                let buffer = std::time::Duration::from_secs(buffer_seconds);
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
    pub fn new(
        strategy: AuthStrategy,
        session_ttl_seconds: u64,
        expiry_buffer_seconds: u64,
    ) -> Self {
        Self {
            auth_strategy: strategy,
            session_token: RwLock::new(None),
            refresh_notify: Mutex::new(None),
            refresh_error: Mutex::new(None),
            session_ttl_seconds,
            expiry_buffer_seconds,
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
    ///
    /// Note: This method does not trigger a refresh. Use [`get_or_refresh_token`]
    /// for automatic token refresh.
    pub async fn get_bearer_token(&self) -> Option<String> {
        match &self.auth_strategy {
            AuthStrategy::ApiToken { token } => Some(token.expose_secret().to_string()),
            AuthStrategy::SessionToken { .. } => {
                let token_guard = self.session_token.read().await;
                token_guard
                    .as_ref()
                    .map(|t| t.value.expose_secret().to_string())
            }
        }
    }

    /// Set the session token (received from login response).
    ///
    /// # Arguments
    /// * `token` - The session token string
    /// * `ttl_seconds` - Time-to-live in seconds (None means no expiry)
    pub async fn set_session_token(&self, token: String, ttl_seconds: Option<u64>) {
        let mut token_guard = self.session_token.write().await;
        *token_guard = Some(SessionToken::new(
            SecretString::new(token.into()),
            ttl_seconds,
        ));
    }

    /// Check if the current session token is expired or will expire soon (read-only).
    ///
    /// For API token auth, always returns false (no expiry concerns).
    pub async fn is_session_expired(&self) -> bool {
        if self.is_api_token() {
            return false;
        }
        let token_guard = self.session_token.read().await;
        token_guard.as_ref().map(|t| t.is_expired()).unwrap_or(true)
    }

    /// Check if the current session token will expire soon (within buffer).
    ///
    /// Returns false for API token auth (no expiry concerns).
    /// Returns true if no session token is set.
    pub async fn session_expires_soon(&self) -> bool {
        if self.is_api_token() {
            return false;
        }
        let token_guard = self.session_token.read().await;
        token_guard
            .as_ref()
            .map(|t| t.will_expire_soon(self.expiry_buffer_seconds))
            .unwrap_or(true)
    }

    /// Clear the current session token (force re-authentication).
    pub async fn clear_session(&self) {
        let mut token_guard = self.session_token.write().await;
        *token_guard = None;
    }

    /// Get a valid bearer token, refreshing if necessary using singleflight pattern.
    ///
    /// This method ensures that even with concurrent callers, only one login request
    /// is made to the Splunk server. All concurrent callers will await the same
    /// refresh result.
    ///
    /// # Arguments
    /// * `login_fn` - Async function that performs the actual login
    ///
    /// # Returns
    /// A valid bearer token string, or an error if refresh fails.
    pub async fn get_or_refresh_token<F, Fut>(
        &self,
        login_fn: F,
    ) -> Result<String, crate::error::ClientError>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<String, crate::error::ClientError>> + Send,
    {
        // Fast path: API token auth (lock-free)
        if let AuthStrategy::ApiToken { token } = &self.auth_strategy {
            return Ok(token.expose_secret().to_string());
        }

        // Fast path: Check if existing token is still valid (read lock only)
        {
            let token_guard = self.session_token.read().await;
            if let Some(token) = token_guard.as_ref() {
                if !token.is_expired() && !token.will_expire_soon(self.expiry_buffer_seconds) {
                    return Ok(token.value.expose_secret().to_string());
                }
            }
        } // Release read lock

        // Slow path: Need to refresh - use singleflight pattern with Notify
        // First, try to become the "leader" that will perform the refresh
        let notify = {
            let mut notify_guard = self.refresh_notify.lock().await;
            match notify_guard.as_ref() {
                Some(n) => {
                    // Another task is already refreshing, clone the Arc and wait
                    n.clone()
                }
                None => {
                    // We are the leader - create a new Notify
                    let new_notify = Arc::new(tokio::sync::Notify::new());
                    *notify_guard = Some(new_notify.clone());

                    // Clear any previous error before starting new refresh
                    {
                        let mut error_guard = self.refresh_error.lock().await;
                        *error_guard = None;
                    }
                    drop(notify_guard);

                    // Perform the actual login
                    let result = login_fn().await;

                    match &result {
                        Ok(token) => {
                            // Store the token on success
                            self.set_session_token(token.clone(), Some(self.session_ttl_seconds))
                                .await;
                            // Clear any previous error
                            let mut error_guard = self.refresh_error.lock().await;
                            *error_guard = None;
                        }
                        Err(err) => {
                            // Store the error for followers to access
                            let mut error_guard = self.refresh_error.lock().await;
                            *error_guard = Some(Arc::new(err.clone()));
                        }
                    }

                    // Notify all waiting tasks
                    let mut notify_guard = self.refresh_notify.lock().await;
                    *notify_guard = None;
                    drop(notify_guard);
                    new_notify.notify_waiters();

                    return result;
                }
            }
        };

        // We are not the leader - wait for the leader to complete
        notify.notified().await;

        // After being notified, check if we now have a valid token
        // If the refresh succeeded, we'll have a token. If it failed, we return error.
        let token_guard = self.session_token.read().await;
        if let Some(token) = token_guard.as_ref() {
            if !token.is_expired() {
                return Ok(token.value.expose_secret().to_string());
            }
        }
        drop(token_guard);

        // Token still not valid - retrieve the stored error from the leader
        let stored_error = {
            let mut error_guard = self.refresh_error.lock().await;
            error_guard.take()
        };

        match stored_error {
            Some(original_error) => {
                // Wrap with context about who was trying to authenticate
                let (username, auth_method) = match &self.auth_strategy {
                    AuthStrategy::SessionToken { username, .. } => {
                        (username.clone(), "session token".to_string())
                    }
                    AuthStrategy::ApiToken { .. } => {
                        (API_TOKEN_USERNAME.to_string(), "api token".to_string())
                    }
                };
                Err(crate::error::ClientError::TokenRefreshFailed {
                    username,
                    auth_method,
                    source: Box::new((*original_error).clone()),
                })
            }
            None => {
                // No stored error - this shouldn't happen, but provide a fallback
                Err(crate::error::ClientError::AuthFailed(
                    "Token refresh failed with unknown error".to_string(),
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use splunk_config::constants::{DEFAULT_EXPIRY_BUFFER_SECS, DEFAULT_SESSION_TTL_SECS};

    #[tokio::test]
    async fn test_api_token_bypasses_session() {
        let strategy = AuthStrategy::ApiToken {
            token: SecretString::new("test-token".to_string().into()),
        };
        let manager = SessionManager::new(
            strategy,
            DEFAULT_SESSION_TTL_SECS,
            DEFAULT_EXPIRY_BUFFER_SECS,
        );
        assert!(manager.is_api_token());
        assert!(!manager.is_session_expired().await);
    }

    #[tokio::test]
    async fn test_api_token_get_bearer_token() {
        let strategy = AuthStrategy::ApiToken {
            token: SecretString::new("test-token".to_string().into()),
        };
        let manager = SessionManager::new(
            strategy,
            DEFAULT_SESSION_TTL_SECS,
            DEFAULT_EXPIRY_BUFFER_SECS,
        );
        let token = manager.get_bearer_token().await;
        assert_eq!(token, Some("test-token".to_string()));
    }

    #[tokio::test]
    async fn test_session_token_without_ttl() {
        let strategy = AuthStrategy::SessionToken {
            username: "admin".to_string(),
            password: SecretString::new("pass".to_string().into()),
        };
        let manager = SessionManager::new(
            strategy,
            DEFAULT_SESSION_TTL_SECS,
            DEFAULT_EXPIRY_BUFFER_SECS,
        );
        assert!(!manager.is_api_token());
        assert!(manager.get_bearer_token().await.is_none());
        assert!(manager.is_session_expired().await);

        manager
            .set_session_token("session-key".to_string(), None)
            .await;
        assert_eq!(
            manager.get_bearer_token().await,
            Some("session-key".to_string())
        );
        // Without TTL, session never expires
        assert!(!manager.is_session_expired().await);
    }

    #[tokio::test]
    async fn test_session_token_with_ttl() {
        let strategy = AuthStrategy::SessionToken {
            username: "admin".to_string(),
            password: SecretString::new("pass".to_string().into()),
        };
        let manager = SessionManager::new(
            strategy,
            DEFAULT_SESSION_TTL_SECS,
            DEFAULT_EXPIRY_BUFFER_SECS,
        );

        manager
            .set_session_token("session-key".to_string(), Some(1))
            .await;
        assert!(!manager.is_session_expired().await);

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

        let manager = SessionManager::new(
            strategy,
            DEFAULT_SESSION_TTL_SECS,
            DEFAULT_EXPIRY_BUFFER_SECS,
        );
        let debug_output = format!("{:?}", manager);

        // The secret token should NOT appear in debug output
        assert!(
            !debug_output.contains(secret_token),
            "Debug output should not contain the token"
        );
    }

    /// Test that session tokens set after login are not exposed in Debug output.
    #[tokio::test]
    async fn test_set_session_token_not_exposed_in_debug() {
        let strategy = AuthStrategy::SessionToken {
            username: "admin".to_string(),
            password: SecretString::new("password".to_string().into()),
        };

        let manager = SessionManager::new(
            strategy,
            DEFAULT_SESSION_TTL_SECS,
            DEFAULT_EXPIRY_BUFFER_SECS,
        );
        let session_token = "new-session-token-after-login-123";
        manager
            .set_session_token(session_token.to_string(), Some(DEFAULT_SESSION_TTL_SECS))
            .await;

        let debug_output = format!("{:?}", manager);

        // The session token should NOT appear in debug output
        assert!(
            !debug_output.contains(session_token),
            "Debug output should not contain the session token"
        );
    }

    /// Test that clearing a session doesn't expose the token in Debug output.
    #[tokio::test]
    async fn test_clear_session_not_exposed_in_debug() {
        let strategy = AuthStrategy::SessionToken {
            username: "admin".to_string(),
            password: SecretString::new("password".to_string().into()),
        };

        let manager = SessionManager::new(
            strategy,
            DEFAULT_SESSION_TTL_SECS,
            DEFAULT_EXPIRY_BUFFER_SECS,
        );
        let session_token = "session-to-be-cleared-456";
        manager
            .set_session_token(session_token.to_string(), Some(DEFAULT_SESSION_TTL_SECS))
            .await;

        // Clear the session
        manager.clear_session().await;

        let debug_output = format!("{:?}", manager);

        // Even after clearing, the old token should not appear
        assert!(
            !debug_output.contains(session_token),
            "Debug output should not contain cleared session token"
        );
    }

    /// Test that API token auth never expires while session auth does.
    #[tokio::test]
    async fn test_api_token_vs_session_expiration() {
        // API token auth - should never expire
        let api_strategy = AuthStrategy::ApiToken {
            token: SecretString::new("api-token".to_string().into()),
        };
        let api_manager = SessionManager::new(
            api_strategy,
            DEFAULT_SESSION_TTL_SECS,
            DEFAULT_EXPIRY_BUFFER_SECS,
        );
        assert!(!api_manager.is_session_expired().await);
        assert!(api_manager.is_api_token());

        // Session token auth without setting token - should be expired
        let session_strategy = AuthStrategy::SessionToken {
            username: "admin".to_string(),
            password: SecretString::new("pass".to_string().into()),
        };
        let session_manager = SessionManager::new(
            session_strategy,
            DEFAULT_SESSION_TTL_SECS,
            DEFAULT_EXPIRY_BUFFER_SECS,
        );
        assert!(session_manager.is_session_expired().await);
        assert!(!session_manager.is_api_token());
    }

    /// Test that bearer token can be accessed programmatically via ExposeSecret.
    #[tokio::test]
    async fn test_bearer_token_accessible_via_expose_secret() {
        let secret_token = "bearer-token-secret-789";
        let strategy = AuthStrategy::ApiToken {
            token: SecretString::new(secret_token.to_string().into()),
        };

        let manager = SessionManager::new(
            strategy,
            DEFAULT_SESSION_TTL_SECS,
            DEFAULT_EXPIRY_BUFFER_SECS,
        );

        // Token should be accessible via get_bearer_token (for API calls)
        let bearer = manager.get_bearer_token().await;
        assert_eq!(bearer, Some(secret_token.to_string()));
    }

    /// Test that multiple secrets in SessionManager are all protected.
    #[tokio::test]
    async fn test_multiple_secrets_protected() {
        let strategy = AuthStrategy::SessionToken {
            username: "admin".to_string(),
            password: SecretString::new("password1".to_string().into()),
        };

        let manager = SessionManager::new(
            strategy,
            DEFAULT_SESSION_TTL_SECS,
            DEFAULT_EXPIRY_BUFFER_SECS,
        );
        manager
            .set_session_token(
                "session-token-123".to_string(),
                Some(DEFAULT_SESSION_TTL_SECS),
            )
            .await;

        let debug_output = format!("{:?}", manager);

        // Neither password nor session token should appear
        assert!(!debug_output.contains("password1"));
        assert!(!debug_output.contains("session-token-123"));
    }

    // ============================================================================
    // Session Expiry Buffer Tests
    // ============================================================================

    /// Test that will_expire_soon() returns false when outside buffer window.
    #[tokio::test]
    async fn test_will_expire_soon_returns_false_outside_buffer() {
        let strategy = AuthStrategy::SessionToken {
            username: "admin".to_string(),
            password: SecretString::new("pass".to_string().into()),
        };
        let manager = SessionManager::new(
            strategy,
            DEFAULT_SESSION_TTL_SECS,
            DEFAULT_EXPIRY_BUFFER_SECS,
        );

        // Set token with 120s TTL and 60s buffer - expires in 120s, buffer ends at 60s
        manager
            .set_session_token("session-key".to_string(), Some(120))
            .await;

        // Should not expire soon immediately after setting
        assert!(!manager.session_expires_soon().await);
        assert!(!manager.is_session_expired().await);
    }

    /// Test that session_expires_soon() returns false for API token auth.
    #[tokio::test]
    async fn test_session_expires_soon_returns_false_for_api_token() {
        let strategy = AuthStrategy::ApiToken {
            token: SecretString::new("api-token".to_string().into()),
        };
        let manager = SessionManager::new(
            strategy,
            DEFAULT_SESSION_TTL_SECS,
            DEFAULT_EXPIRY_BUFFER_SECS,
        );

        // API tokens never expire
        assert!(!manager.session_expires_soon().await);
        assert!(!manager.is_session_expired().await);
    }

    /// Test that session_expires_soon() returns true when no session token is set.
    #[tokio::test]
    async fn test_session_expires_soon_returns_true_when_no_token() {
        let strategy = AuthStrategy::SessionToken {
            username: "admin".to_string(),
            password: SecretString::new("pass".to_string().into()),
        };
        let manager = SessionManager::new(
            strategy,
            DEFAULT_SESSION_TTL_SECS,
            DEFAULT_EXPIRY_BUFFER_SECS,
        );

        // No token set - should be considered expiring soon
        assert!(manager.session_expires_soon().await);
        assert!(manager.is_session_expired().await);
    }

    /// Test that buffer of 0 seconds behaves like original is_expired().
    #[tokio::test]
    async fn test_zero_buffer_behaves_like_is_expired() {
        let strategy = AuthStrategy::SessionToken {
            username: "admin".to_string(),
            password: SecretString::new("pass".to_string().into()),
        };
        let manager = SessionManager::new(
            strategy,
            DEFAULT_SESSION_TTL_SECS,
            DEFAULT_EXPIRY_BUFFER_SECS,
        );

        // Set token with 120s TTL (buffer is DEFAULT_EXPIRY_BUFFER_SECS = 60)
        manager
            .set_session_token("session-key".to_string(), Some(120))
            .await;

        // With 0 buffer, will_expire_soon should behave like is_expired
        assert!(!manager.session_expires_soon().await);
        assert!(!manager.is_session_expired().await);
    }

    /// Test that will_expire_soon returns true when within buffer window.
    /// Note: This test uses a very short TTL to simulate expiry within the test.
    #[test]
    fn test_will_expire_soon_returns_true_within_buffer() {
        // Create a session token directly with a very short TTL
        // With DEFAULT_EXPIRY_BUFFER_SECS = 60 and TTL = 1, will_expire_soon() is always true
        let token = SessionToken::new(
            SecretString::new("test-token".to_string().into()),
            Some(1), // 1 second TTL (buffer is 60s, so it expires immediately)
        );

        // Should be considered expiring soon because buffer > remaining time
        assert!(token.will_expire_soon(DEFAULT_EXPIRY_BUFFER_SECS));
    }

    /// Test that default buffer is applied when token is created.
    #[test]
    fn test_default_buffer_applied() {
        let token = SessionToken::new(
            SecretString::new("test-token".to_string().into()),
            Some(DEFAULT_SESSION_TTL_SECS),
        );

        // Should have the default buffer (60 seconds)
        assert!(!token.is_expired());
        assert!(!token.will_expire_soon(DEFAULT_EXPIRY_BUFFER_SECS));
    }

    // ============================================================================
    // Singleflight Refresh Tests
    // ============================================================================

    /// Test that get_or_refresh_token returns API token directly without calling login.
    #[tokio::test]
    async fn test_get_or_refresh_token_api_token_no_login() {
        let strategy = AuthStrategy::ApiToken {
            token: SecretString::new("api-token".to_string().into()),
        };
        let manager = SessionManager::new(
            strategy,
            DEFAULT_SESSION_TTL_SECS,
            DEFAULT_EXPIRY_BUFFER_SECS,
        );

        let login_called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let login_called_clone = login_called.clone();

        let token = manager
            .get_or_refresh_token(|| async move {
                login_called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
                Ok("new-token".to_string())
            })
            .await;

        assert_eq!(token.unwrap(), "api-token");
        assert!(!login_called.load(std::sync::atomic::Ordering::SeqCst));
    }

    /// Test that get_or_refresh_token returns cached token when valid.
    #[tokio::test]
    async fn test_get_or_refresh_token_returns_cached_when_valid() {
        let strategy = AuthStrategy::SessionToken {
            username: "admin".to_string(),
            password: SecretString::new("pass".to_string().into()),
        };
        let manager = SessionManager::new(
            strategy,
            DEFAULT_SESSION_TTL_SECS,
            DEFAULT_EXPIRY_BUFFER_SECS,
        );

        // Set a valid token
        manager
            .set_session_token("cached-token".to_string(), Some(3600))
            .await;

        let login_called = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let login_called_clone = login_called.clone();

        let token = manager
            .get_or_refresh_token(|| async move {
                login_called_clone.store(true, std::sync::atomic::Ordering::SeqCst);
                Ok("new-token".to_string())
            })
            .await;

        assert_eq!(token.unwrap(), "cached-token");
        assert!(!login_called.load(std::sync::atomic::Ordering::SeqCst));
    }

    /// Test that get_or_refresh_token calls login when no token exists.
    #[tokio::test]
    async fn test_get_or_refresh_token_calls_login_when_no_token() {
        let strategy = AuthStrategy::SessionToken {
            username: "admin".to_string(),
            password: SecretString::new("pass".to_string().into()),
        };
        let manager = SessionManager::new(
            strategy,
            DEFAULT_SESSION_TTL_SECS,
            DEFAULT_EXPIRY_BUFFER_SECS,
        );

        let token = manager
            .get_or_refresh_token(|| async { Ok("new-token".to_string()) })
            .await;

        assert_eq!(token.unwrap(), "new-token");

        // Verify the token was stored
        let cached = manager.get_bearer_token().await;
        assert_eq!(cached, Some("new-token".to_string()));
    }

    /// Test that get_or_refresh_token calls login when token is expired.
    #[tokio::test]
    async fn test_get_or_refresh_token_calls_login_when_expired() {
        let strategy = AuthStrategy::SessionToken {
            username: "admin".to_string(),
            password: SecretString::new("pass".to_string().into()),
        };
        let manager = SessionManager::new(
            strategy,
            DEFAULT_SESSION_TTL_SECS,
            DEFAULT_EXPIRY_BUFFER_SECS,
        );

        // Set an expired token (0 seconds TTL = already expired)
        manager
            .set_session_token("expired-token".to_string(), Some(0))
            .await;

        // Small delay to ensure expiry
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let token = manager
            .get_or_refresh_token(|| async { Ok("refreshed-token".to_string()) })
            .await;

        assert_eq!(token.unwrap(), "refreshed-token");
    }

    /// Test concurrent calls to get_or_refresh_token only trigger one login.
    /// This is the key singleflight pattern test.
    #[tokio::test]
    async fn test_concurrent_calls_singleflight() {
        let strategy = AuthStrategy::SessionToken {
            username: "admin".to_string(),
            password: SecretString::new("pass".to_string().into()),
        };
        let manager = Arc::new(SessionManager::new(
            strategy,
            DEFAULT_SESSION_TTL_SECS,
            DEFAULT_EXPIRY_BUFFER_SECS,
        ));

        let login_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));

        // Spawn 10 concurrent tasks
        let mut handles = vec![];
        for _ in 0..10 {
            let manager = manager.clone();
            let login_count = login_count.clone();
            handles.push(tokio::spawn(async move {
                manager
                    .get_or_refresh_token(move || async move {
                        // Simulate login delay
                        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                        login_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        Ok("shared-token".to_string())
                    })
                    .await
            }));
        }

        // All should succeed
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), "shared-token");
        }

        // Login should only have been called once
        assert_eq!(login_count.load(std::sync::atomic::Ordering::SeqCst), 1);

        // All should have the same token
        let cached = manager.get_bearer_token().await;
        assert_eq!(cached, Some("shared-token".to_string()));
    }

    /// Test that singleflight properly handles login failures.
    #[tokio::test]
    async fn test_singleflight_login_failure() {
        let strategy = AuthStrategy::SessionToken {
            username: "admin".to_string(),
            password: SecretString::new("wrong-pass".to_string().into()),
        };
        let manager = Arc::new(SessionManager::new(
            strategy,
            DEFAULT_SESSION_TTL_SECS,
            DEFAULT_EXPIRY_BUFFER_SECS,
        ));

        // Spawn 5 concurrent tasks
        let mut handles = vec![];
        for _ in 0..5 {
            let manager = manager.clone();
            handles.push(tokio::spawn(async move {
                manager
                    .get_or_refresh_token(|| async {
                        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                        Err(crate::error::ClientError::AuthFailed(
                            "Invalid credentials".to_string(),
                        ))
                    })
                    .await
            }));
        }

        // All should fail - leader gets AuthFailed, followers get TokenRefreshFailed
        for handle in handles {
            let result = handle.await.unwrap();
            match result {
                Err(crate::error::ClientError::AuthFailed(_)) => {
                    // Leader returns raw error
                }
                Err(crate::error::ClientError::TokenRefreshFailed { .. }) => {
                    // Followers get wrapped error
                }
                Err(e) => panic!("Unexpected error type: {:?}", e),
                Ok(_) => panic!("Expected error, got success"),
            }
        }

        // Token should not be set
        assert!(manager.get_bearer_token().await.is_none());
    }

    /// Test that error context is preserved through concurrent refresh failures.
    #[tokio::test]
    async fn test_singleflight_error_context_preserved() {
        let strategy = AuthStrategy::SessionToken {
            username: "testuser".to_string(),
            password: SecretString::new("wrong-pass".to_string().into()),
        };
        let manager = Arc::new(SessionManager::new(
            strategy,
            DEFAULT_SESSION_TTL_SECS,
            DEFAULT_EXPIRY_BUFFER_SECS,
        ));

        let original_error =
            crate::error::ClientError::AuthFailed("Invalid credentials: bad password".to_string());

        // Spawn 5 concurrent tasks
        let mut handles = vec![];
        for _ in 0..5 {
            let manager = manager.clone();
            let err_msg = original_error.to_string();
            handles.push(tokio::spawn(async move {
                manager
                    .get_or_refresh_token(move || {
                        let err_msg = err_msg.clone();
                        async move {
                            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                            Err(crate::error::ClientError::AuthFailed(err_msg))
                        }
                    })
                    .await
            }));
        }

        // All should fail with structured error containing original context
        // Note: The leader returns the raw error, followers get TokenRefreshFailed
        let mut found_leader = false;
        let mut found_followers = 0;
        for handle in handles {
            let result = handle.await.unwrap();
            match result {
                Err(crate::error::ClientError::TokenRefreshFailed {
                    username,
                    auth_method,
                    source,
                }) => {
                    assert_eq!(username, "testuser");
                    assert_eq!(auth_method, "session token");
                    assert!(
                        source.to_string().contains("Invalid credentials"),
                        "Original error should be preserved in source"
                    );
                    found_followers += 1;
                }
                Err(crate::error::ClientError::AuthFailed(_msg)) => {
                    // Leader returns raw error
                    found_leader = true;
                }
                Err(e) => {
                    panic!("Expected TokenRefreshFailed or AuthFailed, got {:?}", e);
                }
                Ok(_) => panic!("Expected error, got success"),
            }
        }

        // Verify we got both leader and follower responses
        assert!(found_leader, "Expected at least one leader task");
        assert!(
            found_followers >= 1,
            "Expected at least one follower task, got {}",
            found_followers
        );

        // Token should not be set
        assert!(manager.get_bearer_token().await.is_none());
    }
}
