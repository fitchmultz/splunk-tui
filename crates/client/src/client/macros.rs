//! Search macro client methods.
//!
//! Responsibilities:
//! - High-level API for macro operations with auth retry.
//! - Wrap endpoint functions with retry_call! macro.

use crate::client::SplunkClient;
use crate::endpoints;
use crate::endpoints::{CreateMacroRequest, UpdateMacroRequest};
use crate::error::Result;
use crate::models::{Macro, MacroCreateParams, MacroUpdateParams};

impl SplunkClient {
    /// List all search macros.
    pub async fn list_macros(&self) -> Result<Vec<Macro>> {
        crate::retry_call!(
            self,
            __token,
            endpoints::list_macros(
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

    /// Get a single macro by name.
    ///
    /// # Arguments
    /// * `name` - The name of the macro
    ///
    /// # Returns
    /// The `Macro` if found, or `ClientError::NotFound` if it doesn't exist.
    pub async fn get_macro(&self, name: &str) -> Result<Macro> {
        crate::retry_call!(
            self,
            __token,
            endpoints::get_macro(
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

    /// Create a new macro.
    ///
    /// # Arguments
    /// * `params` - Parameters for creating the macro
    pub async fn create_macro(&self, params: MacroCreateParams) -> Result<()> {
        let request = CreateMacroRequest {
            name: &params.name,
            definition: &params.definition,
            args: params.args.as_deref(),
            description: params.description.as_deref(),
            disabled: params.disabled,
            iseval: params.iseval,
            validation: params.validation.as_deref(),
            errormsg: params.errormsg.as_deref(),
        };

        crate::retry_call!(
            self,
            __token,
            endpoints::create_macro(
                &self.http,
                &self.base_url,
                &__token,
                &request,
                self.max_retries,
                self.metrics.as_ref(),
                self.circuit_breaker.as_deref(),
            )
            .await
        )
    }

    /// Update an existing macro.
    ///
    /// Only provided fields are updated; omitted fields retain their current values.
    ///
    /// # Arguments
    /// * `name` - The name of the macro to update
    /// * `params` - Parameters for updating the macro
    ///
    /// # Returns
    /// Ok(()) on success, or `ClientError::NotFound` if the macro doesn't exist.
    pub async fn update_macro(&self, name: &str, params: MacroUpdateParams) -> Result<()> {
        let request = UpdateMacroRequest {
            name,
            definition: params.definition.as_deref(),
            args: params.args.as_deref(),
            description: params.description.as_deref(),
            disabled: params.disabled,
            iseval: params.iseval,
            validation: params.validation.as_deref(),
            errormsg: params.errormsg.as_deref(),
        };

        crate::retry_call!(
            self,
            __token,
            endpoints::update_macro(
                &self.http,
                &self.base_url,
                &__token,
                &request,
                self.max_retries,
                self.metrics.as_ref(),
                self.circuit_breaker.as_deref(),
            )
            .await
        )
    }

    /// Delete a macro.
    ///
    /// # Arguments
    /// * `name` - The name of the macro to delete
    ///
    /// # Returns
    /// Ok(()) on success, or `ClientError::NotFound` if the macro doesn't exist.
    pub async fn delete_macro(&self, name: &str) -> Result<()> {
        crate::retry_call!(
            self,
            __token,
            endpoints::delete_macro(
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
