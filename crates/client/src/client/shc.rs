//! Search Head Cluster (SHC) API methods for [`SplunkClient`].
//!
//! # What this module handles:
//! - Getting SHC members, captain, status, and configuration
//! - Managing SHC members (add, remove)
//! - SHC administrative operations (rolling restart, set captain)
//!
//! # What this module does NOT handle:
//! - Low-level SHC endpoint HTTP calls (in [`crate::endpoints::shc`])

use crate::client::SplunkClient;
use crate::endpoints;
use crate::error::Result;
use crate::models::{
    AddShcMemberParams, RemoveShcMemberParams, RollingRestartParams, SetCaptainParams, ShcCaptain,
    ShcConfig, ShcManagementResponse, ShcMember, ShcStatus,
};

impl SplunkClient {
    /// Get SHC members.
    pub async fn get_shc_members(&self) -> Result<Vec<ShcMember>> {
        self.execute_request(
            crate::client::request_executor::RequestPolicy::for_operation("get_shc_members"),
            |__token| async move {
                endpoints::get_shc_members(
                    &self.http,
                    &self.base_url,
                    &__token,
                    self.max_retries,
                    self.metrics.as_ref(),
                    self.circuit_breaker.as_deref(),
                )
                .await
            },
        )
        .await
    }

    /// Get SHC captain information.
    pub async fn get_shc_captain(&self) -> Result<ShcCaptain> {
        self.execute_request(
            crate::client::request_executor::RequestPolicy::for_operation("get_shc_captain"),
            |__token| async move {
                endpoints::get_shc_captain(
                    &self.http,
                    &self.base_url,
                    &__token,
                    self.max_retries,
                    self.metrics.as_ref(),
                    self.circuit_breaker.as_deref(),
                )
                .await
            },
        )
        .await
    }

    /// Get SHC status.
    pub async fn get_shc_status(&self) -> Result<ShcStatus> {
        self.execute_request(
            crate::client::request_executor::RequestPolicy::for_operation("get_shc_status"),
            |__token| async move {
                endpoints::get_shc_status(
                    &self.http,
                    &self.base_url,
                    &__token,
                    self.max_retries,
                    self.metrics.as_ref(),
                    self.circuit_breaker.as_deref(),
                )
                .await
            },
        )
        .await
    }

    /// Get SHC configuration.
    pub async fn get_shc_config(&self) -> Result<ShcConfig> {
        self.execute_request(
            crate::client::request_executor::RequestPolicy::for_operation("get_shc_config"),
            |__token| async move {
                endpoints::get_shc_config(
                    &self.http,
                    &self.base_url,
                    &__token,
                    self.max_retries,
                    self.metrics.as_ref(),
                    self.circuit_breaker.as_deref(),
                )
                .await
            },
        )
        .await
    }

    /// Add a member to the SHC.
    pub async fn add_shc_member(&self, target_uri: &str) -> Result<ShcManagementResponse> {
        let params = AddShcMemberParams {
            target_uri: target_uri.to_string(),
        };
        self.execute_request(
            crate::client::request_executor::RequestPolicy::for_operation("add_shc_member"),
            |__token| {
                let params = params.clone();
                async move {
                    endpoints::add_shc_member(
                        &self.http,
                        &self.base_url,
                        &__token,
                        &params,
                        self.max_retries,
                        self.metrics.as_ref(),
                        self.circuit_breaker.as_deref(),
                    )
                    .await
                }
            },
        )
        .await
    }

    /// Remove a member from the SHC.
    pub async fn remove_shc_member(&self, member_guid: &str) -> Result<ShcManagementResponse> {
        let params = RemoveShcMemberParams {
            member: member_guid.to_string(),
        };
        self.execute_request(
            crate::client::request_executor::RequestPolicy::for_operation("remove_shc_member"),
            |__token| {
                let params = params.clone();
                async move {
                    endpoints::remove_shc_member(
                        &self.http,
                        &self.base_url,
                        &__token,
                        &params,
                        self.max_retries,
                        self.metrics.as_ref(),
                        self.circuit_breaker.as_deref(),
                    )
                    .await
                }
            },
        )
        .await
    }

    /// Trigger a rolling restart of the SHC.
    pub async fn rolling_restart_shc(&self, force: bool) -> Result<ShcManagementResponse> {
        let params = RollingRestartParams { force };
        self.execute_request(
            crate::client::request_executor::RequestPolicy::for_operation("rolling_restart_shc"),
            |__token| {
                let params = params.clone();
                async move {
                    endpoints::rolling_restart_shc(
                        &self.http,
                        &self.base_url,
                        &__token,
                        &params,
                        self.max_retries,
                        self.metrics.as_ref(),
                        self.circuit_breaker.as_deref(),
                    )
                    .await
                }
            },
        )
        .await
    }

    /// Set a specific member as captain.
    pub async fn set_shc_captain(&self, target_guid: &str) -> Result<ShcManagementResponse> {
        let params = SetCaptainParams {
            target_guid: target_guid.to_string(),
        };
        self.execute_request(
            crate::client::request_executor::RequestPolicy::for_operation("set_shc_captain"),
            |__token| {
                let params = params.clone();
                async move {
                    endpoints::set_shc_captain(
                        &self.http,
                        &self.base_url,
                        &__token,
                        &params,
                        self.max_retries,
                        self.metrics.as_ref(),
                        self.circuit_breaker.as_deref(),
                    )
                    .await
                }
            },
        )
        .await
    }
}
