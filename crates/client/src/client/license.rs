//! License management API methods for [`SplunkClient`].
//!
//! # What this module handles:
//! - Getting license usage information
//! - Listing license pools
//! - Listing license stacks
//!
//! # What this module does NOT handle:
//! - License installation or configuration (not yet implemented)
//! - Low-level license endpoint HTTP calls (in [`crate::endpoints::license`])

use crate::client::SplunkClient;
use crate::endpoints;
use crate::error::Result;
use crate::models::{LicensePool, LicenseStack, LicenseUsage};

impl SplunkClient {
    /// Get license usage information.
    pub async fn get_license_usage(&mut self) -> Result<Vec<LicenseUsage>> {
        crate::retry_call!(
            self,
            __token,
            endpoints::get_license_usage(
                &self.http,
                &self.base_url,
                &__token,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// List all license pools.
    pub async fn list_license_pools(&mut self) -> Result<Vec<LicensePool>> {
        crate::retry_call!(
            self,
            __token,
            endpoints::list_license_pools(
                &self.http,
                &self.base_url,
                &__token,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// List all license stacks.
    pub async fn list_license_stacks(&mut self) -> Result<Vec<LicenseStack>> {
        crate::retry_call!(
            self,
            __token,
            endpoints::list_license_stacks(
                &self.http,
                &self.base_url,
                &__token,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }
}
