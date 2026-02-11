//! Search macro REST API endpoints.
//!
//! Responsibilities:
//! - Low-level HTTP calls to /services/admin/macros endpoints.
//! - Handle request serialization and response parsing.
//!
//! Does NOT handle:
//! - Does not handle auth retry (see client module).
//! - Does not contain business logic.

use reqwest::Client;
use tracing::debug;

use crate::client::circuit_breaker::CircuitBreaker;
use crate::endpoints::encode_path_segment;
use crate::endpoints::{form_params_str, send_request_with_retry};
use crate::error::{ClientError, Result};
use crate::metrics::MetricsCollector;
use crate::models::MacroListResponse;
use crate::name_merge::attach_entry_name;

/// Request parameters for creating a new macro.
///
/// This struct consolidates all parameters for the create_macro endpoint to reduce
/// the argument count and satisfy clippy's too_many_arguments lint.
#[derive(Debug, Clone)]
pub struct CreateMacroRequest<'a> {
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

impl<'a> CreateMacroRequest<'a> {
    /// Create a new CreateMacroRequest with required fields.
    pub fn new(name: &'a str, definition: &'a str) -> Self {
        Self {
            name,
            definition,
            args: None,
            description: None,
            disabled: false,
            iseval: false,
            validation: None,
            errormsg: None,
        }
    }
}

/// Request parameters for updating an existing macro.
///
/// This struct consolidates all parameters for the update_macro endpoint to reduce
/// the argument count and satisfy clippy's too_many_arguments lint.
/// Only provided fields are updated; omitted fields (None) retain their current values.
#[derive(Debug, Clone, Default)]
pub struct UpdateMacroRequest<'a> {
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

impl<'a> UpdateMacroRequest<'a> {
    /// Create a new UpdateMacroRequest for the specified macro.
    pub fn new(name: &'a str) -> Self {
        Self {
            name,
            ..Default::default()
        }
    }
}

/// List all search macros.
#[allow(clippy::too_many_arguments)]
pub async fn list_macros(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<Vec<crate::models::Macro>> {
    debug!("Listing search macros");

    let url = format!("{}/services/admin/macros", base_url);

    let query_params: Vec<(String, String)> = vec![("output_mode".to_string(), "json".to_string())];

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&query_params);

    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/admin/macros",
        "GET",
        metrics,
        circuit_breaker,
    )
    .await?;

    let resp: MacroListResponse = response.json().await.map_err(|e| {
        ClientError::InvalidResponse(format!("Failed to parse macros response: {}", e))
    })?;

    Ok(resp
        .entry
        .into_iter()
        .map(|e| attach_entry_name(e.name, e.content))
        .collect())
}

/// Get a single macro by name.
///
/// # Arguments
/// * `client` - The reqwest client
/// * `base_url` - The Splunk base URL
/// * `auth_token` - Authentication token
/// * `name` - The name of the macro
/// * `max_retries` - Maximum number of retries for transient failures
/// * `metrics` - Optional metrics collector
///
/// # Returns
/// The `Macro` if found, or `ClientError::NotFound` if it doesn't exist.
#[allow(clippy::too_many_arguments)]
pub async fn get_macro(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    name: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<crate::models::Macro> {
    debug!("Getting macro: {}", name);

    let encoded_name = encode_path_segment(name);
    let url = format!("{}/services/admin/macros/{}", base_url, encoded_name);

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")]);

    let response = match send_request_with_retry(
        builder,
        max_retries,
        "/services/admin/macros/{name}",
        "GET",
        metrics,
        circuit_breaker,
    )
    .await
    {
        Ok(resp) => resp,
        Err(ClientError::ApiError { status: 404, .. }) => {
            return Err(ClientError::NotFound(format!("Macro '{}' not found", name)));
        }
        Err(e) => return Err(e),
    };

    let body: MacroListResponse = response.json().await.map_err(|e| {
        ClientError::InvalidResponse(format!("Failed to parse macro response: {}", e))
    })?;

    // Extract the first entry's content (Splunk returns single entry for single-resource GET)
    let entry = body
        .entry
        .into_iter()
        .next()
        .ok_or_else(|| ClientError::NotFound(format!("Macro '{}' not found", name)))?;

    Ok(attach_entry_name(entry.name, entry.content))
}

/// Create a new macro.
///
/// # Arguments
/// * `client` - The reqwest client
/// * `base_url` - The Splunk base URL
/// * `auth_token` - Authentication token
/// * `request` - Request parameters for creating the macro
/// * `max_retries` - Maximum number of retries
/// * `metrics` - Optional metrics collector
#[allow(clippy::too_many_arguments)]
pub async fn create_macro(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    request: &CreateMacroRequest<'_>,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<()> {
    debug!("Creating macro: {}", request.name);

    let url = format!("{}/services/admin/macros", base_url);

    let mut form_params: Vec<(&str, String)> = vec![];

    form_params_str! { form_params =>
        "name" => str Some(request.name),
        "definition" => str Some(request.definition),
        "args" => str request.args,
        "description" => str request.description,
        "disabled" => required_bool request.disabled,
        "iseval" => required_bool request.iseval,
        "validation" => str request.validation,
        "errormsg" => str request.errormsg,
    }

    let builder = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")])
        .form(&form_params);

    let _response = send_request_with_retry(
        builder,
        max_retries,
        "/services/admin/macros",
        "POST",
        metrics,
        circuit_breaker,
    )
    .await?;

    Ok(())
}

/// Update an existing macro.
///
/// Only provided fields are updated; omitted fields retain their current values.
///
/// # Arguments
/// * `client` - The reqwest client
/// * `base_url` - The Splunk base URL
/// * `auth_token` - Authentication token
/// * `request` - Request parameters for updating the macro
/// * `max_retries` - Maximum number of retries
/// * `metrics` - Optional metrics collector
///
/// # Returns
/// Ok(()) on success, or `ClientError::NotFound` if the macro doesn't exist.
#[allow(clippy::too_many_arguments)]
pub async fn update_macro(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    request: &UpdateMacroRequest<'_>,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<()> {
    debug!("Updating macro: {}", request.name);

    let encoded_name = encode_path_segment(request.name);
    let url = format!("{}/services/admin/macros/{}", base_url, encoded_name);

    let mut form_params: Vec<(&str, String)> = Vec::new();

    form_params_str! { form_params =>
        "definition" => str request.definition,
        "args" => str request.args,
        "description" => str request.description,
        "disabled" => bool request.disabled,
        "iseval" => bool request.iseval,
        "validation" => str request.validation,
        "errormsg" => str request.errormsg,
    }

    let builder = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")])
        .form(&form_params);

    match send_request_with_retry(
        builder,
        max_retries,
        "/services/admin/macros/{name}",
        "POST",
        metrics,
        circuit_breaker,
    )
    .await
    {
        Ok(_) => Ok(()),
        Err(ClientError::ApiError { status: 404, .. }) => Err(ClientError::NotFound(format!(
            "Macro '{}' not found",
            request.name
        ))),
        Err(e) => Err(e),
    }
}

/// Delete a macro.
///
/// # Arguments
/// * `client` - The reqwest client
/// * `base_url` - The Splunk base URL
/// * `auth_token` - Authentication token
/// * `name` - The name of the macro to delete
/// * `max_retries` - Maximum number of retries
/// * `metrics` - Optional metrics collector
///
/// # Returns
/// Ok(()) on success, or `ClientError::NotFound` if the macro doesn't exist.
#[allow(clippy::too_many_arguments)]
pub async fn delete_macro(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    name: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<()> {
    debug!("Deleting macro: {}", name);

    let encoded_name = encode_path_segment(name);
    let url = format!("{}/services/admin/macros/{}", base_url, encoded_name);

    let builder = client
        .delete(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")]);

    match send_request_with_retry(
        builder,
        max_retries,
        "/services/admin/macros/{name}",
        "DELETE",
        metrics,
        circuit_breaker,
    )
    .await
    {
        Ok(_) => Ok(()),
        Err(ClientError::ApiError { status: 404, .. }) => {
            Err(ClientError::NotFound(format!("Macro '{}' not found", name)))
        }
        Err(e) => Err(e),
    }
}
