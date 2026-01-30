//! User management API methods for [`SplunkClient`].
//!
//! # What this module handles:
//! - Listing users
//! - Creating new users
//! - Modifying existing users
//! - Deleting users
//!
//! # What this module does NOT handle:
//! - Authentication and session management (in [`crate::client::session`])
//! - Low-level user endpoint HTTP calls (in [`crate::endpoints::users`])

use crate::client::SplunkClient;
use crate::endpoints;
use crate::error::Result;
use crate::models::{CreateUserParams, ModifyUserParams, User};

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

    /// Create a new user with the specified parameters.
    pub async fn create_user(&mut self, params: &CreateUserParams) -> Result<User> {
        crate::retry_call!(
            self,
            __token,
            endpoints::create_user(
                &self.http,
                &self.base_url,
                &__token,
                params,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Modify an existing user.
    pub async fn modify_user(&mut self, name: &str, params: &ModifyUserParams) -> Result<User> {
        crate::retry_call!(
            self,
            __token,
            endpoints::modify_user(
                &self.http,
                &self.base_url,
                &__token,
                name,
                params,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Delete a user by name.
    pub async fn delete_user(&mut self, name: &str) -> Result<()> {
        crate::retry_call!(
            self,
            __token,
            endpoints::delete_user(
                &self.http,
                &self.base_url,
                &__token,
                name,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }
}
