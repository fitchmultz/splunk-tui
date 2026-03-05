//! Input management API methods for [`SplunkClient`].
//!
//! # What this module handles:
//! - Listing data inputs (TCP, UDP, Monitor, Script)
//! - Enabling/disabling inputs
//!
//! # What this module does NOT handle:
//! - Creating or removing inputs (not yet implemented)
//! - Low-level input endpoint HTTP calls (in [`crate::endpoints::inputs`])

use crate::client::SplunkClient;
use crate::endpoints;
use crate::error::ClientError;
use crate::error::Result;
use crate::models::Input;

impl SplunkClient {
    /// List all data inputs across all types.
    ///
    /// Fetches inputs from all types: tcp/raw, tcp/cooked, udp, monitor, script.
    /// Results are concatenated into a single list.
    ///
    /// # Arguments
    ///
    /// * `count` - Maximum number of results to return per input type (default: 30)
    /// * `offset` - Offset for pagination
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of `Input` structs on success.
    ///
    /// # Errors
    ///
    /// Returns a `ClientError` for unrecoverable failures.
    /// Input types that are not available on a given Splunk instance (404)
    /// are skipped.
    pub async fn list_inputs(
        &self,
        count: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Input>> {
        let input_types = ["tcp/raw", "tcp/cooked", "udp", "monitor", "script"];
        let mut all_inputs = Vec::new();

        for input_type in &input_types {
            match self.list_inputs_by_type(input_type, count, offset).await {
                Ok(inputs) => all_inputs.extend(inputs),
                Err(ClientError::NotFound(_)) | Err(ClientError::ApiError { status: 404, .. }) => {
                    // Some deployments do not expose every input type endpoint.
                    continue;
                }
                Err(e) => return Err(e),
            }
        }

        Ok(all_inputs)
    }

    /// List inputs of a specific type.
    ///
    /// # Arguments
    ///
    /// * `input_type` - The type of input (tcp/raw, tcp/cooked, udp, monitor, script)
    /// * `count` - Maximum number of results to return (default: 30)
    /// * `offset` - Offset for pagination
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of `Input` structs on success.
    ///
    /// # Errors
    ///
    /// Returns a `ClientError` if the request fails or the response cannot be parsed.
    pub async fn list_inputs_by_type(
        &self,
        input_type: &str,
        count: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Input>> {
        crate::retry_call!(
            self,
            __token,
            endpoints::list_inputs_by_type(
                &self.http,
                &self.base_url,
                &__token,
                input_type,
                count,
                offset,
                self.max_retries,
                self.metrics.as_ref(),
                self.circuit_breaker.as_deref(),
            )
            .await
        )
    }

    /// Enable an input.
    ///
    /// # Arguments
    ///
    /// * `input_type` - The type of input (tcp/raw, tcp/cooked, udp, monitor, script)
    /// * `name` - The name of the input to enable
    ///
    /// # Errors
    ///
    /// Returns a `ClientError` if the request fails.
    pub async fn enable_input(&self, input_type: &str, name: &str) -> Result<()> {
        crate::retry_call!(
            self,
            __token,
            endpoints::enable_input(
                &self.http,
                &self.base_url,
                &__token,
                input_type,
                name,
                self.max_retries,
                self.metrics.as_ref(),
                self.circuit_breaker.as_deref(),
            )
            .await
        )
    }

    /// Disable an input.
    ///
    /// # Arguments
    ///
    /// * `input_type` - The type of input (tcp/raw, tcp/cooked, udp, monitor, script)
    /// * `name` - The name of the input to disable
    ///
    /// # Errors
    ///
    /// Returns a `ClientError` if the request fails.
    pub async fn disable_input(&self, input_type: &str, name: &str) -> Result<()> {
        crate::retry_call!(
            self,
            __token,
            endpoints::disable_input(
                &self.http,
                &self.base_url,
                &__token,
                input_type,
                name,
                self.max_retries,
                self.metrics.as_ref(),
                self.circuit_breaker.as_deref(),
            )
            .await
        )
    }
}
