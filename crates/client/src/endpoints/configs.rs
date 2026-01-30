//! Configuration file management endpoints.
//!
//! This module provides low-level HTTP endpoint functions for interacting
//! with the Splunk configuration files REST API.
//!
//! # What this module handles:
//! - HTTP GET requests to list configuration stanzas
//! - HTTP GET requests to retrieve specific configuration stanzas
//! - Query parameter construction for pagination
//!
//! # What this module does NOT handle:
//! - Authentication retry logic (handled by [`crate::client`])
//! - High-level client operations (see [`crate::client::configs`])
//! - Response deserialization (delegated to models)

use reqwest::Client;

use crate::endpoints::send_request_with_retry;
use crate::error::Result;
use crate::metrics::MetricsCollector;
use crate::models::{ConfigFile, ConfigListResponse, ConfigStanza};
use crate::name_merge::attach_entry_name;

/// List configuration stanzas for a specific config file.
///
/// Retrieves a list of configuration stanzas from a specific config file
/// (e.g., props, transforms, inputs).
///
/// # Arguments
///
/// * `client` - The HTTP client to use for the request
/// * `base_url` - The base URL of the Splunk server
/// * `auth_token` - The authentication token for the request
/// * `config_file` - The config file name (e.g., "props", "transforms")
/// * `count` - Maximum number of results to return (default: 30)
/// * `offset` - Offset for pagination
/// * `max_retries` - Maximum number of retry attempts for failed requests
/// * `metrics` - Optional metrics collector for request tracking
///
/// # Returns
///
/// A `Result` containing a vector of `ConfigStanza` structs on success.
///
/// # Errors
///
/// Returns a `ClientError` if the request fails or the response cannot be parsed.
#[allow(clippy::too_many_arguments)]
pub async fn list_config_stanzas(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    config_file: &str,
    count: Option<u64>,
    offset: Option<u64>,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<Vec<ConfigStanza>> {
    let url = format!("{}/services/configs/conf-{}", base_url, config_file);

    let mut query_params: Vec<(String, String)> = vec![
        ("output_mode".to_string(), "json".to_string()),
        ("count".to_string(), count.unwrap_or(30).to_string()),
    ];

    if let Some(o) = offset {
        query_params.push(("offset".to_string(), o.to_string()));
    }

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&query_params);
    let response = send_request_with_retry(
        builder,
        max_retries,
        &format!("/services/configs/conf-{}", config_file),
        "GET",
        metrics,
    )
    .await?;

    let resp: ConfigListResponse = response.json().await?;

    Ok(resp
        .entry
        .into_iter()
        .map(|e| {
            let mut stanza = attach_entry_name(e.name, e.content);
            stanza.config_file = config_file.to_string();
            stanza
        })
        .collect())
}

/// Get a specific configuration stanza.
///
/// Retrieves a single configuration stanza by name from a specific config file.
///
/// # Arguments
///
/// * `client` - The HTTP client to use for the request
/// * `base_url` - The base URL of the Splunk server
/// * `auth_token` - The authentication token for the request
/// * `config_file` - The config file name (e.g., "props", "transforms")
/// * `stanza_name` - The name of the stanza to retrieve
/// * `max_retries` - Maximum number of retry attempts for failed requests
/// * `metrics` - Optional metrics collector for request tracking
///
/// # Returns
///
/// A `Result` containing a `ConfigStanza` struct on success.
///
/// # Errors
///
/// Returns a `ClientError` if the request fails or the response cannot be parsed.
#[allow(clippy::too_many_arguments)]
pub async fn get_config_stanza(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    config_file: &str,
    stanza_name: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<ConfigStanza> {
    // URL encode the stanza name as it may contain special characters
    let encoded_stanza = encode_stanza_name(stanza_name);
    let url = format!(
        "{}/services/configs/conf-{}/{}",
        base_url, config_file, encoded_stanza
    );

    let query_params: Vec<(String, String)> = vec![("output_mode".to_string(), "json".to_string())];

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&query_params);
    let response = send_request_with_retry(
        builder,
        max_retries,
        &format!("/services/configs/conf-{}/{{stanza}}", config_file),
        "GET",
        metrics,
    )
    .await?;

    let resp: ConfigListResponse = response.json().await?;

    // The API returns a list with a single entry for a specific stanza
    let stanza = resp
        .entry
        .into_iter()
        .next()
        .map(|e| {
            let mut s = attach_entry_name(e.name, e.content);
            s.config_file = config_file.to_string();
            s
        })
        .ok_or_else(|| {
            crate::error::ClientError::NotFound(format!(
                "config stanza '{}' in '{}'",
                stanza_name, config_file
            ))
        })?;

    Ok(stanza)
}

/// List available configuration files.
///
/// Retrieves a list of available configuration files from the Splunk server.
/// This returns the supported config files as defined in the models module.
///
/// # Arguments
///
/// * `client` - The HTTP client to use for the request
/// * `base_url` - The base URL of the Splunk server
/// * `auth_token` - The authentication token for the request
/// * `max_retries` - Maximum number of retry attempts for failed requests
/// * `metrics` - Optional metrics collector for request tracking
///
/// # Returns
///
/// A `Result` containing a vector of `ConfigFile` structs on success.
///
/// # Errors
///
/// Returns a `ClientError` if the request fails.
pub async fn list_config_files(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<Vec<ConfigFile>> {
    // The Splunk API doesn't have a direct endpoint to list all config files,
    // so we return the supported config files with their titles
    let _ = (client, base_url, auth_token, max_retries, metrics);

    // Return the list of supported config files
    let config_files: Vec<ConfigFile> = crate::models::SUPPORTED_CONFIG_FILES
        .iter()
        .map(|name| ConfigFile {
            name: name.to_string(),
            title: format!("{} Configuration", capitalize_first(name)),
            description: get_config_description(name),
        })
        .collect();

    Ok(config_files)
}

/// Capitalize the first letter of a string.
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// URL-encode a stanza name for use in API requests.
///
/// Stanza names may contain special characters like spaces, brackets, etc.
/// We use percent-encoding to ensure the URL is valid.
fn encode_stanza_name(name: &str) -> String {
    // Percent-encode special characters that may appear in stanza names
    name.chars()
        .map(|c| match c {
            ' ' => "%20".to_string(),
            '[' => "%5B".to_string(),
            ']' => "%5D".to_string(),
            ':' => "%3A".to_string(),
            '/' => "%2F".to_string(),
            '?' => "%3F".to_string(),
            '&' => "%26".to_string(),
            '=' => "%3D".to_string(),
            '%' => "%25".to_string(),
            '#' => "%23".to_string(),
            '@' => "%40".to_string(),
            '!' => "%21".to_string(),
            '$' => "%24".to_string(),
            '\'' => "%27".to_string(),
            '(' => "%28".to_string(),
            ')' => "%29".to_string(),
            '*' => "%2A".to_string(),
            '+' => "%2B".to_string(),
            ',' => "%2C".to_string(),
            ';' => "%3B".to_string(),
            '<' => "%3C".to_string(),
            '>' => "%3E".to_string(),
            '"' => "%22".to_string(),
            '{' => "%7B".to_string(),
            '}' => "%7D".to_string(),
            '|' => "%7C".to_string(),
            '\\' => "%5C".to_string(),
            '^' => "%5E".to_string(),
            '`' => "%60".to_string(),
            '~' => "%7E".to_string(),
            _ => c.to_string(),
        })
        .collect()
}

/// Get a description for a config file.
fn get_config_description(name: &str) -> Option<String> {
    match name {
        "props" => Some("Source type properties and field extractions".to_string()),
        "transforms" => Some("Field transformations and lookups".to_string()),
        "inputs" => Some("Data input configurations".to_string()),
        "outputs" => Some("Forwarder output configurations".to_string()),
        "server" => Some("Server-wide settings".to_string()),
        "indexes" => Some("Index definitions and settings".to_string()),
        "savedsearches" => Some("Saved search definitions".to_string()),
        "authentication" => Some("Authentication settings".to_string()),
        "authorize" => Some("Role-based access control".to_string()),
        "distsearch" => Some("Distributed search configuration".to_string()),
        "limits" => Some("Server limits and thresholds".to_string()),
        "web" => Some("Web interface settings".to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capitalize_first() {
        assert_eq!(capitalize_first("props"), "Props");
        assert_eq!(capitalize_first("transforms"), "Transforms");
        assert_eq!(capitalize_first(""), "");
        assert_eq!(capitalize_first("a"), "A");
    }

    #[test]
    fn test_get_config_description() {
        assert!(get_config_description("props").is_some());
        assert!(get_config_description("nonexistent").is_none());
        assert!(
            get_config_description("transforms")
                .unwrap()
                .contains("transformations")
        );
    }

    #[test]
    fn test_list_config_files_returns_supported_files() {
        // This is a simple smoke test - the actual async test would require a mock server
        let supported = crate::models::SUPPORTED_CONFIG_FILES;
        assert!(supported.contains(&"props"));
        assert!(supported.contains(&"transforms"));
        assert!(supported.contains(&"inputs"));
    }
}
