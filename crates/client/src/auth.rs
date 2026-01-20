//! Authentication strategies and session management.

use secrecy::{ExposeSecret, SecretString};
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
}

impl SessionToken {
    fn new(value: SecretString, ttl_seconds: Option<u64>) -> Self {
        let expires_at =
            ttl_seconds.map(|ttl| Instant::now() + std::time::Duration::from_secs(ttl));
        Self { value, expires_at }
    }

    fn is_expired(&self) -> bool {
        self.expires_at
            .map(|exp| exp.saturating_duration_since(Instant::now()).is_zero())
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
    pub fn set_session_token(&mut self, token: String, ttl_seconds: Option<u64>) {
        self.session_token = Some(SessionToken::new(
            SecretString::new(token.into()),
            ttl_seconds,
        ));
    }

    /// Check if the current session token is expired or will expire soon.
    pub fn is_session_expired(&self) -> bool {
        if self.is_api_token() {
            return false;
        }
        self.session_token
            .as_ref()
            .map(|t| t.is_expired())
            .unwrap_or(true)
    }

    /// Clear the current session token (force re-authentication).
    pub fn clear_session(&mut self) {
        self.session_token = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        manager.set_session_token("session-key".to_string(), None);
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

        manager.set_session_token("session-key".to_string(), Some(1));
        assert!(!manager.is_session_expired());

        // Note: Can't easily test actual expiry in unit test without time manipulation
    }
}
