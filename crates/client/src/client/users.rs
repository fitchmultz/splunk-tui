//! User management API methods for [`SplunkClient`].
//!
//! # What this module handles:
//! - Listing users
//!
//! # What this module does NOT handle:
//! - Creating or modifying users (not yet implemented)
//! - Authentication and session management (in [`crate::client::session`])
//! - Low-level user endpoint HTTP calls (in [`crate::endpoints::users`])

use crate::client::SplunkClient;
use crate::endpoints;
use crate::error::Result;
use crate::models::User;

impl SplunkClient {
    /// List all users.
    pub async fn list_users(
        &mut self,
        count: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<User>> {
        crate::retry_call!(
            self,
            __token,
            endpoints::list_users(
                &self.http,
                &self.base_url,
                &__token,
                count,
                offset,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }
}
