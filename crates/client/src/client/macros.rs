//! Search macro client methods.
//!
//! Responsibilities:
//! - High-level API for macro operations with auth retry.
//! - Wrap endpoint functions with retry_call! macro.

use crate::client::SplunkClient;
use crate::endpoints;
use crate::error::Result;
use crate::models::Macro;

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
    /// * `name` - The name of the macro
    /// * `definition` - The SPL snippet or eval expression
    /// * `args` - Optional comma-separated argument names
    /// * `description` - Optional description
    /// * `disabled` - Whether the macro is disabled
    /// * `iseval` - Whether the macro is an eval expression
    /// * `validation` - Optional validation expression
    /// * `errormsg` - Optional error message for validation failure
    pub async fn create_macro(
        &mut self,
        name: &str,
        definition: &str,
        args: Option<&str>,
        description: Option<&str>,
        disabled: bool,
        iseval: bool,
        validation: Option<&str>,
        errormsg: Option<&str>,
    ) -> Result<()> {
        crate::retry_call!(
            self,
            __token,
            endpoints::create_macro(
                &self.http,
                &self.base_url,
                &__token,
                name,
                definition,
                args,
                description,
                disabled,
                iseval,
                validation,
                errormsg,
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
    /// * `name` - The name of the macro to update
    /// * `definition` - Optional new definition
    /// * `args` - Optional new arguments
    /// * `description` - Optional new description
    /// * `disabled` - Optional enable/disable flag
    /// * `iseval` - Optional eval expression flag
    /// * `validation` - Optional new validation expression
    /// * `errormsg` - Optional new error message
    ///
    /// # Returns
    /// Ok(()) on success, or `ClientError::NotFound` if the macro doesn't exist.
    pub async fn update_macro(
        &mut self,
        name: &str,
        definition: Option<&str>,
        args: Option<&str>,
        description: Option<&str>,
        disabled: Option<bool>,
        iseval: Option<bool>,
        validation: Option<&str>,
        errormsg: Option<&str>,
    ) -> Result<()> {
        crate::retry_call!(
            self,
            __token,
            endpoints::update_macro(
                &self.http,
                &self.base_url,
                &__token,
                name,
                definition,
                args,
                description,
                disabled,
                iseval,
                validation,
                errormsg,
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
