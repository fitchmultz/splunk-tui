//! App management API methods for [`SplunkClient`].
//!
//! # What this module handles:
//! - Listing installed apps
//! - Getting app details
//! - Enabling/disabling apps
//! - Installing apps from .spl packages
//! - Removing (uninstalling) apps
//!
//! # What this module does NOT handle:
//! - Low-level app endpoint HTTP calls (in [`crate::endpoints`])

use crate::client::SplunkClient;
use crate::endpoints;
use crate::error::Result;
use crate::models::App;

impl SplunkClient {
    /// List all installed apps.
    pub async fn list_apps(&self, count: Option<usize>, offset: Option<usize>) -> Result<Vec<App>> {
        self.execute_request(
            crate::client::request_executor::RequestPolicy::for_operation("list_apps"),
            |__token| async move {
                endpoints::list_apps(
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

    /// Get specific app details by name.
    pub async fn get_app(&self, app_name: &str) -> Result<App> {
        self.execute_request(
            crate::client::request_executor::RequestPolicy::for_operation("get_app"),
            |__token| async move {
                endpoints::get_app(
                    &self.http,
                    &self.base_url,
                    &__token,
                    app_name,
                    self.max_retries,
                    self.metrics.as_ref(),
                    self.circuit_breaker.as_deref(),
                )
                .await
            },
        )
        .await
    }

    /// Enable an app by name.
    pub async fn enable_app(&self, app_name: &str) -> Result<()> {
        self.execute_request(
            crate::client::request_executor::RequestPolicy::for_operation("enable_app"),
            |__token| async move {
                endpoints::enable_app(
                    &self.http,
                    &self.base_url,
                    &__token,
                    app_name,
                    self.max_retries,
                    self.metrics.as_ref(),
                    self.circuit_breaker.as_deref(),
                )
                .await
            },
        )
        .await
    }

    /// Disable an app by name.
    pub async fn disable_app(&self, app_name: &str) -> Result<()> {
        self.execute_request(
            crate::client::request_executor::RequestPolicy::for_operation("disable_app"),
            |__token| async move {
                endpoints::disable_app(
                    &self.http,
                    &self.base_url,
                    &__token,
                    app_name,
                    self.max_retries,
                    self.metrics.as_ref(),
                    self.circuit_breaker.as_deref(),
                )
                .await
            },
        )
        .await
    }

    /// Install an app from a .spl file package.
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to the .spl package file
    ///
    /// # Returns
    ///
    /// The installed `App` on success.
    pub async fn install_app(&self, file_path: &std::path::Path) -> Result<App> {
        self.execute_request(
            crate::client::request_executor::RequestPolicy::for_operation("install_app"),
            |__token| async move {
                endpoints::install_app(
                    &self.http,
                    &self.base_url,
                    &__token,
                    file_path,
                    self.max_retries,
                    self.metrics.as_ref(),
                    self.circuit_breaker.as_deref(),
                )
                .await
            },
        )
        .await
    }

    /// Remove (uninstall) an app by name.
    ///
    /// # Arguments
    ///
    /// * `app_name` - Name of the app to remove
    pub async fn remove_app(&self, app_name: &str) -> Result<()> {
        self.execute_request(
            crate::client::request_executor::RequestPolicy::for_operation("remove_app"),
            |__token| async move {
                endpoints::remove_app(
                    &self.http,
                    &self.base_url,
                    &__token,
                    app_name,
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
