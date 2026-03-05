//! License management endpoints.
//!
//! This module provides HTTP endpoint functions for Splunk license management:
//! - Reading license usage, pools, and stacks
//! - Installing license files
//! - Managing license pools (create, modify, delete)
//! - Activating/deactivating licenses
//!
//! # What this module handles:
//! - HTTP requests to /services/licenser/* endpoints
//! - Multipart file uploads for license installation
//! - Form-encoded POST requests for pool management
//!
//! # What this module does NOT handle:
//! - Business logic or retry logic (handled by client layer)
//! - License file validation (handled by Splunk server)

use reqwest::Client;

use crate::client::circuit_breaker::CircuitBreaker;
use crate::endpoints::encode_path_segment;
use crate::endpoints::{form_params_str, send_request_with_retry};
use crate::error::{ClientError, Result};
use crate::metrics::MetricsCollector;
use crate::models::{
    CreatePoolParams, LicenseActivationResult, LicenseInstallResult, LicensePool, LicenseStack,
    LicenseUsage, ModifyPoolParams, SplunkResponse,
};

/// Get license usage information.
#[allow(clippy::too_many_arguments)]
pub async fn get_license_usage(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<Vec<LicenseUsage>> {
    let url = format!("{}/services/licenser/usage", base_url);

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")]);
    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/licenser/usage",
        "GET",
        metrics,
        circuit_breaker,
    )
    .await?;

    let resp: SplunkResponse<LicenseUsage> = response.json().await?;

    Ok(resp
        .entry
        .into_iter()
        .map(|e| {
            let mut content = e.content;
            content.name = e.name;
            content
        })
        .collect())
}

/// List all license pools.
#[allow(clippy::too_many_arguments)]
pub async fn list_license_pools(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<Vec<LicensePool>> {
    let url = format!("{}/services/licenser/pools", base_url);

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")]);
    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/licenser/pools",
        "GET",
        metrics,
        circuit_breaker,
    )
    .await?;

    let resp: SplunkResponse<LicensePool> = response.json().await?;

    Ok(resp
        .entry
        .into_iter()
        .map(|e| {
            let mut content = e.content;
            content.name = e.name;
            content
        })
        .collect())
}

/// List all license stacks.
#[allow(clippy::too_many_arguments)]
pub async fn list_license_stacks(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<Vec<LicenseStack>> {
    let url = format!("{}/services/licenser/stacks", base_url);

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")]);
    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/licenser/stacks",
        "GET",
        metrics,
        circuit_breaker,
    )
    .await?;

    let resp: SplunkResponse<LicenseStack> = response.json().await?;

    Ok(resp
        .entry
        .into_iter()
        .map(|e| {
            let mut content = e.content;
            content.name = e.name;
            content
        })
        .collect())
}

/// List all installed licenses.
///
/// GET /services/licenser/licenses
#[allow(clippy::too_many_arguments)]
pub async fn list_installed_licenses(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<Vec<crate::models::InstalledLicense>> {
    let url = format!("{}/services/licenser/licenses", base_url);

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")]);
    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/licenser/licenses",
        "GET",
        metrics,
        circuit_breaker,
    )
    .await?;

    let resp: SplunkResponse<crate::models::InstalledLicense> = response.json().await?;

    Ok(resp
        .entry
        .into_iter()
        .map(|e| {
            let mut content = e.content;
            content.name = e.name;
            content
        })
        .collect())
}

/// Install a license file (.sla) via multipart upload.
///
/// POST /services/licenser/licenses
///
/// # Arguments
///
/// * `client` - The HTTP client
/// * `base_url` - The Splunk base URL
/// * `auth_token` - The authentication token
/// * `license_file_content` - Raw bytes of the license file
/// * `filename` - Original filename of the license file
/// * `max_retries` - Maximum number of retries for authentication failures
/// * `metrics` - Optional metrics collector
///
/// # Returns
///
/// Result indicating success or failure of the installation
#[allow(clippy::too_many_arguments)]
pub async fn install_license(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    license_file_content: Vec<u8>,
    filename: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<LicenseInstallResult> {
    let url = format!("{}/services/licenser/licenses", base_url);

    // Build multipart form for license file upload
    let form = reqwest::multipart::Form::new().part(
        "splunk_file",
        reqwest::multipart::Part::bytes(license_file_content)
            .file_name(filename.to_string())
            .mime_str("application/octet-stream")
            .map_err(|e| ClientError::InvalidRequest(format!("Invalid mime type: {}", e)))?,
    );

    let builder = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")])
        .multipart(form);

    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/licenser/licenses",
        "POST",
        metrics,
        circuit_breaker,
    )
    .await?;

    // Parse response to extract license name if available
    let resp: serde_json::Value = response.json().await?;

    // Try to extract the license name from the response
    let license_name = resp
        .get("entry")
        .and_then(|e| e.get(0))
        .and_then(|e| e.get("name"))
        .and_then(|n| n.as_str())
        .map(|s| s.to_string());

    // Check for errors in the response
    if let Some(messages) = resp.get("messages")
        && let Some(msg_array) = messages.as_array()
        && let Some(first_msg) = msg_array.first()
        && let Some(text) = first_msg.get("text").and_then(|t| t.as_str())
    {
        return Ok(LicenseInstallResult {
            success: false,
            message: text.to_string(),
            license_name: None,
        });
    }

    Ok(LicenseInstallResult {
        success: true,
        message: "License installed successfully".to_string(),
        license_name,
    })
}

/// Create a new license pool.
///
/// POST /services/licenser/pools
#[allow(clippy::too_many_arguments)]
pub async fn create_license_pool(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    params: &CreatePoolParams,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<LicensePool> {
    let url = format!("{}/services/licenser/pools", base_url);

    let mut form_params: Vec<(&str, String)> = vec![];

    form_params_str! { form_params =>
        "name" => str Some(&params.name),
        "stack_id" => str Some(&params.stack_id),
        "quota" => params.quota_bytes,
        "description" => str params.description.as_deref(),
    }

    let builder = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")])
        .form(&form_params);

    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/licenser/pools",
        "POST",
        metrics,
        circuit_breaker,
    )
    .await?;

    let resp: SplunkResponse<LicensePool> = response.json().await?;

    resp.entry
        .into_iter()
        .next()
        .map(|e| {
            let mut content = e.content;
            content.name = e.name;
            content
        })
        .ok_or_else(|| ClientError::InvalidResponse("No pool created in response".to_string()))
}

/// Delete a license pool.
///
/// DELETE /services/licenser/pools/{name}
#[allow(clippy::too_many_arguments)]
pub async fn delete_license_pool(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    pool_name: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<()> {
    let encoded_pool_name = encode_path_segment(pool_name);
    let url = format!("{}/services/licenser/pools/{}", base_url, encoded_pool_name);

    let builder = client
        .delete(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")]);

    let _response = send_request_with_retry(
        builder,
        max_retries,
        "/services/licenser/pools/{pool_name}",
        "DELETE",
        metrics,
        circuit_breaker,
    )
    .await?;

    Ok(())
}

/// Modify an existing license pool.
///
/// POST /services/licenser/pools/{name}
#[allow(clippy::too_many_arguments)]
pub async fn modify_license_pool(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    pool_name: &str,
    params: &ModifyPoolParams,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<LicensePool> {
    let encoded_pool_name = encode_path_segment(pool_name);
    let url = format!("{}/services/licenser/pools/{}", base_url, encoded_pool_name);

    let mut form_params: Vec<(&str, String)> = Vec::new();

    form_params_str! { form_params =>
        "quota" => params.quota_bytes,
        "description" => str params.description.as_deref(),
    }

    let builder = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")])
        .form(&form_params);

    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/licenser/pools/{pool_name}",
        "POST",
        metrics,
        circuit_breaker,
    )
    .await?;

    let resp: SplunkResponse<LicensePool> = response.json().await?;

    resp.entry
        .into_iter()
        .next()
        .map(|e| {
            let mut content = e.content;
            content.name = e.name;
            content
        })
        .ok_or_else(|| ClientError::InvalidResponse("No pool modified in response".to_string()))
}

/// Activate a license.
///
/// POST /services/licenser/licenses/{name}/enable
#[allow(clippy::too_many_arguments)]
pub async fn activate_license(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    license_name: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<LicenseActivationResult> {
    let encoded_license_name = encode_path_segment(license_name);
    let url = format!(
        "{}/services/licenser/licenses/{}/enable",
        base_url, encoded_license_name
    );

    let builder = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")]);

    let _response = send_request_with_retry(
        builder,
        max_retries,
        "/services/licenser/licenses/{name}/enable",
        "POST",
        metrics,
        circuit_breaker,
    )
    .await?;

    Ok(LicenseActivationResult {
        success: true,
        message: format!("License '{}' activated successfully", license_name),
    })
}

/// Deactivate a license.
///
/// POST /services/licenser/licenses/{name}/disable
#[allow(clippy::too_many_arguments)]
pub async fn deactivate_license(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    license_name: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<LicenseActivationResult> {
    let encoded_license_name = encode_path_segment(license_name);
    let url = format!(
        "{}/services/licenser/licenses/{}/disable",
        base_url, encoded_license_name
    );

    let builder = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")]);

    let _response = send_request_with_retry(
        builder,
        max_retries,
        "/services/licenser/licenses/{name}/disable",
        "POST",
        metrics,
        circuit_breaker,
    )
    .await?;

    Ok(LicenseActivationResult {
        success: true,
        message: format!("License '{}' deactivated successfully", license_name),
    })
}
