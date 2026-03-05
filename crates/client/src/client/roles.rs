//! Role management API methods for [`SplunkClient`].
//!
//! # What this module handles:
//! - Listing roles
//! - Creating new roles
//! - Modifying existing roles
//! - Deleting roles
//!
//! # What this module does NOT handle:
//! - Authentication and session management (in [`crate::client::session`])
//! - Low-level role endpoint HTTP calls (in [`crate::endpoints::roles`])

use crate::client::SplunkClient;
use crate::endpoints;
use crate::error::Result;
use crate::models::{CreateRoleParams, ModifyRoleParams, Role};

impl SplunkClient {
    /// List all roles.
    pub async fn list_roles(
        &self,
        count: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Role>> {
        crate::retry_call!(
            self,
            __token,
            endpoints::list_roles(
                &self.http,
                &self.base_url,
                &__token,
                count,
                offset,
                self.max_retries,
                self.metrics.as_ref(),
                self.circuit_breaker.as_deref(),
            )
            .await
        )
    }

    /// Create a new role with the specified parameters.
    pub async fn create_role(&self, params: &CreateRoleParams) -> Result<Role> {
        crate::retry_call!(
            self,
            __token,
            endpoints::create_role(
                &self.http,
                &self.base_url,
                &__token,
                params,
                self.max_retries,
                self.metrics.as_ref(),
                self.circuit_breaker.as_deref(),
            )
            .await
        )
    }

    /// Modify an existing role.
    pub async fn modify_role(&self, name: &str, params: &ModifyRoleParams) -> Result<Role> {
        crate::retry_call!(
            self,
            __token,
            endpoints::modify_role(
                &self.http,
                &self.base_url,
                &__token,
                name,
                params,
                self.max_retries,
                self.metrics.as_ref(),
                self.circuit_breaker.as_deref(),
            )
            .await
        )
    }

    /// Delete a role by name.
    pub async fn delete_role(&self, name: &str) -> Result<()> {
        crate::retry_call!(
            self,
            __token,
            endpoints::delete_role(
                &self.http,
                &self.base_url,
                &__token,
                name,
                self.max_retries,
                self.metrics.as_ref(),
                self.circuit_breaker.as_deref(),
            )
            .await
        )
    }
}
