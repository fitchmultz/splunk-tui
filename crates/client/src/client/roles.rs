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
        self.execute_request(
            crate::client::request_executor::RequestPolicy::for_operation("list_roles"),
            |__token| async move {
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
            },
        )
        .await
    }

    /// Create a new role with the specified parameters.
    pub async fn create_role(&self, params: &CreateRoleParams) -> Result<Role> {
        self.execute_request(
            crate::client::request_executor::RequestPolicy::for_operation("create_role"),
            |__token| async move {
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
            },
        )
        .await
    }

    /// Modify an existing role.
    pub async fn modify_role(&self, name: &str, params: &ModifyRoleParams) -> Result<Role> {
        self.execute_request(
            crate::client::request_executor::RequestPolicy::for_operation("modify_role"),
            |__token| async move {
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
            },
        )
        .await
    }

    /// Delete a role by name.
    pub async fn delete_role(&self, name: &str) -> Result<()> {
        self.execute_request(
            crate::client::request_executor::RequestPolicy::for_operation("delete_role"),
            |__token| async move {
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
            },
        )
        .await
    }
}
