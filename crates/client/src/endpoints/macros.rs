//! Search macro REST API endpoints.
//!
//! Responsibilities:
//! - Low-level HTTP calls to /services/admin/macros endpoints.
//! - Handle request serialization and response parsing.
//!
//! Non-responsibilities:
//! - Does not handle auth retry (see client module).
//! - Does not contain business logic.

use reqwest::Client;
use tracing::debug;

use crate::endpoints::send_request_with_retry;
use crate::error::{ClientError, Result};
use crate::metrics::MetricsCollector;
use crate::models::MacroListResponse;
use crate::name_merge::attach_entry_name;

/// List all search macros.
pub async fn list_macros(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
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
pub async fn get_macro(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    name: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<crate::models::Macro> {
    debug!("Getting macro: {}", name);

    let url = format!("{}/services/admin/macros/{}", base_url, name);

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
/// * `name` - The name of the macro
/// * `definition` - The SPL snippet or eval expression
/// * `args` - Optional comma-separated argument names
/// * `description` - Optional description
/// * `disabled` - Whether the macro is disabled
/// * `iseval` - Whether the macro is an eval expression
/// * `validation` - Optional validation expression
/// * `errormsg` - Optional error message for validation failure
/// * `max_retries` - Maximum number of retries
/// * `metrics` - Optional metrics collector
pub async fn create_macro(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    name: &str,
    definition: &str,
    args: Option<&str>,
    description: Option<&str>,
    disabled: bool,
    iseval: bool,
    validation: Option<&str>,
    errormsg: Option<&str>,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<()> {
    debug!("Creating macro: {}", name);

    let url = format!("{}/services/admin/macros", base_url);

    let mut form_params: Vec<(&str, String)> = vec![
        ("name", name.to_string()),
        ("definition", definition.to_string()),
    ];

    if let Some(a) = args {
        form_params.push(("args", a.to_string()));
    }
    if let Some(d) = description {
        form_params.push(("description", d.to_string()));
    }
    if disabled {
        form_params.push(("disabled", disabled.to_string()));
    }
    if iseval {
        form_params.push(("iseval", iseval.to_string()));
    }
    if let Some(v) = validation {
        form_params.push(("validation", v.to_string()));
    }
    if let Some(e) = errormsg {
        form_params.push(("errormsg", e.to_string()));
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
/// * `name` - The name of the macro to update
/// * `definition` - Optional new definition
/// * `args` - Optional new arguments
/// * `description` - Optional new description
/// * `disabled` - Optional enable/disable flag
/// * `iseval` - Optional eval expression flag
/// * `validation` - Optional new validation expression
/// * `errormsg` - Optional new error message
/// * `max_retries` - Maximum number of retries
/// * `metrics` - Optional metrics collector
///
/// # Returns
/// Ok(()) on success, or `ClientError::NotFound` if the macro doesn't exist.
pub async fn update_macro(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    name: &str,
    definition: Option<&str>,
    args: Option<&str>,
    description: Option<&str>,
    disabled: Option<bool>,
    iseval: Option<bool>,
    validation: Option<&str>,
    errormsg: Option<&str>,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<()> {
    debug!("Updating macro: {}", name);

    let url = format!("{}/services/admin/macros/{}", base_url, name);

    let mut form_params: Vec<(&str, String)> = Vec::new();

    if let Some(d) = definition {
        form_params.push(("definition", d.to_string()));
    }
    if let Some(a) = args {
        form_params.push(("args", a.to_string()));
    }
    if let Some(d) = description {
        form_params.push(("description", d.to_string()));
    }
    if let Some(disabled_flag) = disabled {
        form_params.push(("disabled", disabled_flag.to_string()));
    }
    if let Some(iseval_flag) = iseval {
        form_params.push(("iseval", iseval_flag.to_string()));
    }
    if let Some(v) = validation {
        form_params.push(("validation", v.to_string()));
    }
    if let Some(e) = errormsg {
        form_params.push(("errormsg", e.to_string()));
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
pub async fn delete_macro(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    name: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<()> {
    debug!("Deleting macro: {}", name);

    let url = format!("{}/services/admin/macros/{}", base_url, name);

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
