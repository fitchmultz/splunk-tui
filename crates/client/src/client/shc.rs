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
        crate::retry_call!(
            self,
            __token,
            endpoints::get_shc_members(
                &self.http,
                &self.base_url,
                &__token,
                self.max_retries,
                self.metrics.as_ref(),
                self.circuit_breaker.as_deref(),
            )
            .await
        )
    }

    /// Get SHC captain information.
    pub async fn get_shc_captain(&self) -> Result<ShcCaptain> {
        crate::retry_call!(
            self,
            __token,
            endpoints::get_shc_captain(
                &self.http,
                &self.base_url,
                &__token,
                self.max_retries,
                self.metrics.as_ref(),
                self.circuit_breaker.as_deref(),
            )
            .await
        )
    }

    /// Get SHC status.
    pub async fn get_shc_status(&self) -> Result<ShcStatus> {
        crate::retry_call!(
            self,
            __token,
            endpoints::get_shc_status(
                &self.http,
                &self.base_url,
                &__token,
                self.max_retries,
                self.metrics.as_ref(),
                self.circuit_breaker.as_deref(),
            )
            .await
        )
    }

    /// Get SHC configuration.
    pub async fn get_shc_config(&self) -> Result<ShcConfig> {
        crate::retry_call!(
            self,
            __token,
            endpoints::get_shc_config(
                &self.http,
                &self.base_url,
                &__token,
                self.max_retries,
                self.metrics.as_ref(),
                self.circuit_breaker.as_deref(),
            )
            .await
        )
    }

    /// Add a member to the SHC.
    pub async fn add_shc_member(&self, target_uri: &str) -> Result<ShcManagementResponse> {
        let params = AddShcMemberParams {
            target_uri: target_uri.to_string(),
        };
        crate::retry_call!(
            self,
            __token,
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
        )
    }

    /// Remove a member from the SHC.
    pub async fn remove_shc_member(&self, member_guid: &str) -> Result<ShcManagementResponse> {
        let params = RemoveShcMemberParams {
            member: member_guid.to_string(),
        };
        crate::retry_call!(
            self,
            __token,
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
        )
    }

    /// Trigger a rolling restart of the SHC.
    pub async fn rolling_restart_shc(&self, force: bool) -> Result<ShcManagementResponse> {
        let params = RollingRestartParams { force };
        crate::retry_call!(
            self,
            __token,
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
        )
    }

    /// Set a specific member as captain.
    pub async fn set_shc_captain(&self, target_guid: &str) -> Result<ShcManagementResponse> {
        let params = SetCaptainParams {
            target_guid: target_guid.to_string(),
        };
        crate::retry_call!(
            self,
            __token,
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
        )
    }
}
