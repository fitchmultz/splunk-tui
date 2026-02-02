//! Search macro client methods.
//!
//! Responsibilities:
//! - High-level API for macro operations with auth retry.
//! - Wrap endpoint functions with retry_call! macro.

use crate::client::SplunkClient;
use crate::endpoints;
use crate::error::Result;
use crate::models::Macro;

/// Parameters for creating a new macro.
#[derive(Debug, Clone, Default)]
pub struct MacroCreateParams<'a> {
    /// The name of the macro
    pub name: &'a str,
    /// The SPL snippet or eval expression
    pub definition: &'a str,
    /// Optional comma-separated argument names
    pub args: Option<&'a str>,
    /// Optional description
    pub description: Option<&'a str>,
    /// Whether the macro is disabled
    pub disabled: bool,
    /// Whether the macro is an eval expression
    pub iseval: bool,
    /// Optional validation expression
    pub validation: Option<&'a str>,
    /// Optional error message for validation failure
    pub errormsg: Option<&'a str>,
}

/// Parameters for updating an existing macro.
#[derive(Debug, Clone, Default)]
pub struct MacroUpdateParams<'a> {
    /// The name of the macro to update
    pub name: &'a str,
    /// Optional new definition
    pub definition: Option<&'a str>,
    /// Optional new arguments
    pub args: Option<&'a str>,
    /// Optional new description
    pub description: Option<&'a str>,
    /// Optional enable/disable flag
    pub disabled: Option<bool>,
    /// Optional eval expression flag
    pub iseval: Option<bool>,
    /// Optional new validation expression
    pub validation: Option<&'a str>,
    /// Optional new error message
    pub errormsg: Option<&'a str>,
}

impl SplunkClient {
    /// List all search macros.
    pub async fn list_macros(&mut self) -> Result<Vec<Macro>> {
        crate::retry_call!(
            self,
            __token,
            endpoints::list_macros(
                &self.http,
                &self.base_url,
                &__token,
                self.max_retries,
                self.metrics.as_ref(),
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
    pub async fn get_macro(&mut self, name: &str) -> Result<Macro> {
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
            )
            .await
        )
    }

    /// Create a new macro.
    ///
    /// # Arguments
    /// * `params` - Parameters for creating the macro
    pub async fn create_macro(&mut self, params: MacroCreateParams<'_>) -> Result<()> {
        crate::retry_call!(
            self,
            __token,
            endpoints::create_macro(
                &self.http,
                &self.base_url,
                &__token,
                params.name,
                params.definition,
                params.args,
                params.description,
                params.disabled,
                params.iseval,
                params.validation,
                params.errormsg,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Update an existing macro.
    ///
    /// Only provided fields are updated; omitted fields retain their current values.
    ///
    /// # Arguments
    /// * `params` - Parameters for updating the macro
    ///
    /// # Returns
    /// Ok(()) on success, or `ClientError::NotFound` if the macro doesn't exist.
    pub async fn update_macro(&mut self, params: MacroUpdateParams<'_>) -> Result<()> {
        crate::retry_call!(
            self,
            __token,
            endpoints::update_macro(
                &self.http,
                &self.base_url,
                &__token,
                params.name,
                params.definition,
                params.args,
                params.description,
                params.disabled,
                params.iseval,
                params.validation,
                params.errormsg,
                self.max_retries,
                self.metrics.as_ref(),
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
    pub async fn delete_macro(&mut self, name: &str) -> Result<()> {
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
            )
            .await
        )
    }
}
