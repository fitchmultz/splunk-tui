//! App management API methods for [`SplunkClient`].
//!
//! # What this module handles:
//! - Listing installed apps
//! - Getting app details
//! - Enabling/disabling apps
//!
//! # What this module does NOT handle:
//! - Installing or removing apps (not yet implemented)
//! - Low-level app endpoint HTTP calls (in [`crate::endpoints`])

use crate::client::SplunkClient;
use crate::endpoints;
use crate::error::Result;
use crate::models::App;

impl SplunkClient {
    /// List all installed apps.
    pub async fn list_apps(&mut self, count: Option<u64>, offset: Option<u64>) -> Result<Vec<App>> {
        crate::retry_call!(
            self,
            __token,
            endpoints::list_apps(
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

    /// Get specific app details by name.
    pub async fn get_app(&mut self, app_name: &str) -> Result<App> {
        crate::retry_call!(
            self,
            __token,
            endpoints::get_app(
                &self.http,
                &self.base_url,
                &__token,
                app_name,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Enable an app by name.
    pub async fn enable_app(&mut self, app_name: &str) -> Result<()> {
        crate::retry_call!(
            self,
            __token,
            endpoints::enable_app(
                &self.http,
                &self.base_url,
                &__token,
                app_name,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Disable an app by name.
    pub async fn disable_app(&mut self, app_name: &str) -> Result<()> {
        crate::retry_call!(
            self,
            __token,
            endpoints::disable_app(
                &self.http,
                &self.base_url,
                &__token,
                app_name,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }
}
