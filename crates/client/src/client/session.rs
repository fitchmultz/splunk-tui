//! Client-level session management helpers.
//!
//! This module contains methods on [`SplunkClient`] that interact with the
//! [`SessionManager`] to handle authentication token retrieval and validation.
//!
//! # What this module does NOT handle:
//! - Low-level session token storage and expiry tracking (handled by [`SessionManager`] in `auth.rs`)
//! - Authentication strategy selection (handled during client construction in `builder.rs`)
//! - Direct HTTP authentication calls (handled by endpoint functions in `endpoints/`)
//!
//! # Invariants
//! - [`get_auth_token()`] requires `&mut self` because it may trigger a login call
//! - API token authentication never triggers login; the token is returned directly
//! - Session authentication proactively refreshes tokens before they expire

use crate::auth::AuthStrategy;
use crate::client::SplunkClient;
use crate::endpoints;
use crate::error::{ClientError, Result};
use secrecy::ExposeSecret;

impl SplunkClient {
    /// Get the current authentication token, logging in if necessary.
    ///
    /// This method handles the authentication token lifecycle:
    /// - For API token auth: returns the configured token directly
    /// - For session auth: checks if the session is expired or will expire soon,
    ///   and triggers a login if needed
    ///
    /// Proactively refreshes the session if it will expire soon (within the buffer
    /// window) to prevent race conditions where the token expires during an API call.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::AuthFailed`] if login fails.
    /// Returns [`ClientError::SessionExpired`] if no valid token is available.
    pub(crate) async fn get_auth_token(&mut self) -> Result<String> {
        // For API token auth, just return the token
        if self.session_manager.is_api_token()
            && let Some(token) = self.session_manager.get_bearer_token()
        {
            return Ok(token.to_string());
        }

        // For session auth, check if we need to login (expired OR will expire soon)
        if self.session_manager.is_session_expired() || self.session_manager.session_expires_soon()
        {
            self.login().await?;
        }

        self.session_manager
            .get_bearer_token()
            .map(|s| s.to_string())
            .ok_or_else(|| {
                let username = match self.session_manager.strategy() {
                    AuthStrategy::SessionToken { username, .. } => username.clone(),
                    AuthStrategy::ApiToken { .. } => "api-token".to_string(),
                };
                ClientError::SessionExpired { username }
            })
    }

    /// Check if the client is using API token authentication.
    ///
    /// API tokens do not expire and do not require session management.
    pub fn is_api_token_auth(&self) -> bool {
        self.session_manager.is_api_token()
    }

    /// Login with username/password to get a session token.
    ///
    /// This method is only valid for [`AuthStrategy::SessionToken`] authentication.
    /// The obtained session token is stored in the `SessionManager` for subsequent
    /// API calls.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::AuthFailed`] if the auth strategy is not session-based.
    /// Returns [`ClientError::ApiError`] if the login request fails.
    pub async fn login(&mut self) -> Result<String> {
        if let AuthStrategy::SessionToken { username, password } = self.session_manager.strategy() {
            let token = endpoints::login(
                &self.http,
                &self.base_url,
                username,
                password.expose_secret(),
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await?;

            // Use configured session TTL for proactive refresh
            let token_clone = token.clone();
            self.session_manager
                .set_session_token(token_clone, Some(self.session_ttl_seconds));

            Ok(token)
        } else {
            Err(ClientError::AuthFailed(
                "Cannot login with API token auth strategy".to_string(),
            ))
        }
    }
}
