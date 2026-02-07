//! License management API methods for [`SplunkClient`].
//!
//! # What this module handles:
//! - Getting license usage information
//! - Listing license pools, stacks, and installed licenses
//! - Installing license files
//! - Managing license pools (create, modify, delete)
//! - Activating/deactivating licenses
//!
//! # What this module does NOT handle:
//! - Low-level license endpoint HTTP calls (in [`crate::endpoints::license`])
//! - License file validation (handled by Splunk server)

use std::path::Path;

use crate::client::SplunkClient;
use crate::endpoints;
use crate::error::{ClientError, Result};
use crate::models::{
    CreatePoolParams, InstalledLicense, LicenseActivationResult, LicenseInstallResult, LicensePool,
    LicenseStack, LicenseUsage, ModifyPoolParams,
};

impl SplunkClient {
    /// Get license usage information.
    pub async fn get_license_usage(&self) -> Result<Vec<LicenseUsage>> {
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
    pub async fn list_license_pools(&self) -> Result<Vec<LicensePool>> {
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
    pub async fn list_license_stacks(&self) -> Result<Vec<LicenseStack>> {
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

    /// List all installed licenses.
    pub async fn list_installed_licenses(&self) -> Result<Vec<InstalledLicense>> {
        crate::retry_call!(
            self,
            __token,
            endpoints::list_installed_licenses(
                &self.http,
                &self.base_url,
                &__token,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Install a license file on the Splunk server.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the .sla license file
    ///
    /// # Errors
    ///
    /// Returns `ClientError::InvalidRequest` if the file cannot be read.
    /// Returns other errors if the upload fails.
    pub async fn install_license(&self, file_path: &Path) -> Result<LicenseInstallResult> {
        // Read the file content
        let file_content = tokio::fs::read(file_path).await.map_err(|e| {
            ClientError::InvalidRequest(format!(
                "Failed to read license file '{}': {}",
                file_path.display(),
                e
            ))
        })?;

        let filename = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("license.sla");

        // Clone file_content for retry attempts
        let file_content_clone = file_content.clone();
        crate::retry_call!(
            self,
            __token,
            endpoints::install_license(
                &self.http,
                &self.base_url,
                &__token,
                file_content_clone.clone(),
                filename,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Create a new license pool.
    ///
    /// # Arguments
    ///
    /// * `params` - Parameters for the new pool including name, stack_id, and optional quota/description
    pub async fn create_license_pool(&self, params: &CreatePoolParams) -> Result<LicensePool> {
        crate::retry_call!(
            self,
            __token,
            endpoints::create_license_pool(
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

    /// Delete a license pool by name.
    ///
    /// # Arguments
    ///
    /// * `pool_name` - Name of the pool to delete
    pub async fn delete_license_pool(&self, pool_name: &str) -> Result<()> {
        crate::retry_call!(
            self,
            __token,
            endpoints::delete_license_pool(
                &self.http,
                &self.base_url,
                &__token,
                pool_name,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Modify an existing license pool.
    ///
    /// # Arguments
    ///
    /// * `pool_name` - Name of the pool to modify
    /// * `params` - Parameters to update (quota and/or description)
    pub async fn modify_license_pool(
        &self,
        pool_name: &str,
        params: &ModifyPoolParams,
    ) -> Result<LicensePool> {
        crate::retry_call!(
            self,
            __token,
            endpoints::modify_license_pool(
                &self.http,
                &self.base_url,
                &__token,
                pool_name,
                params,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Activate a license.
    ///
    /// # Arguments
    ///
    /// * `license_name` - Name of the license to activate
    pub async fn activate_license(&self, license_name: &str) -> Result<LicenseActivationResult> {
        crate::retry_call!(
            self,
            __token,
            endpoints::activate_license(
                &self.http,
                &self.base_url,
                &__token,
                license_name,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Deactivate a license.
    ///
    /// # Arguments
    ///
    /// * `license_name` - Name of the license to deactivate
    pub async fn deactivate_license(&self, license_name: &str) -> Result<LicenseActivationResult> {
        crate::retry_call!(
            self,
            __token,
            endpoints::deactivate_license(
                &self.http,
                &self.base_url,
                &__token,
                license_name,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }
}
