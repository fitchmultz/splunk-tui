//! Role management endpoints.

use reqwest::{Client, Url};

use crate::endpoints::form_params;
use crate::endpoints::send_request_with_retry;
use crate::error::{ClientError, Result};
use crate::metrics::MetricsCollector;
use crate::models::{CreateRoleParams, ModifyRoleParams, Role, RoleListResponse};
use crate::name_merge::attach_entry_name;

/// List all roles.
pub async fn list_roles(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    count: Option<usize>,
    offset: Option<usize>,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<Vec<Role>> {
    let url = format!("{}/services/authorization/roles", base_url);

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
        "/services/authorization/roles",
        "GET",
        metrics,
    )
    .await?;

    let resp: RoleListResponse = response.json().await?;

    Ok(resp
        .entry
        .into_iter()
        .map(|e| attach_entry_name(e.name, e.content))
        .collect())
}

/// Create a new role.
pub async fn create_role(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    params: &CreateRoleParams,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<Role> {
    let url = format!("{}/services/authorization/roles", base_url);

    let mut form_params: Vec<(String, String)> =
        vec![("output_mode".to_string(), "json".to_string())];

    form_params! { form_params =>
        "name" => required_clone params.name,
        "capabilities" => join params.capabilities,
        "searchIndexes" => join params.search_indexes,
        "searchFilter" => ref params.search_filter,
        "importedRoles" => join params.imported_roles,
        "defaultApp" => ref params.default_app,
    }

    let builder = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .form(&form_params);

    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/authorization/roles",
        "POST",
        metrics,
    )
    .await?;

    let resp: serde_json::Value = response.json().await?;

    // Extract entry from response
    let entry = resp.get("entry").and_then(|e| e.get(0)).ok_or_else(|| {
        ClientError::InvalidResponse("Missing entry in create role response".to_string())
    })?;

    let entry_name = entry
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or(&params.name)
        .to_string();

    let content = entry.get("content").ok_or_else(|| {
        ClientError::InvalidResponse("Missing entry content in create role response".to_string())
    })?;

    let role: Role = serde_json::from_value(content.clone())
        .map_err(|e| ClientError::InvalidResponse(format!("Failed to parse role: {}", e)))?;

    Ok(attach_entry_name(entry_name, role))
}

/// Modify an existing role.
pub async fn modify_role(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    role_name: &str,
    params: &ModifyRoleParams,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<Role> {
    let url = Url::parse(base_url)
        .map_err(|e| ClientError::InvalidUrl(format!("Invalid base URL: {}", e)))?
        .join(&format!("/services/authorization/roles/{}", role_name))
        .map_err(|e| ClientError::InvalidUrl(format!("Invalid role name: {}", e)))?;

    let mut form_params: Vec<(String, String)> =
        vec![("output_mode".to_string(), "json".to_string())];

    form_params! { form_params =>
        "capabilities" => join_opt params.capabilities,
        "searchIndexes" => join_opt params.search_indexes,
        "searchFilter" => ref params.search_filter,
        "importedRoles" => join_opt params.imported_roles,
        "defaultApp" => ref params.default_app,
    }

    let builder = client
        .post(url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .form(&form_params);

    let response = send_request_with_retry(
        builder,
        max_retries,
        &format!("/services/authorization/roles/{}", role_name),
        "POST",
        metrics,
    )
    .await?;

    let resp: serde_json::Value = response.json().await?;

    // Extract entry from response
    let entry = resp.get("entry").and_then(|e| e.get(0)).ok_or_else(|| {
        ClientError::InvalidResponse(format!(
            "Missing entry in modify role response for '{}'",
            role_name
        ))
    })?;

    let entry_name = entry
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or(role_name)
        .to_string();

    let content = entry.get("content").ok_or_else(|| {
        ClientError::InvalidResponse(format!(
            "Missing entry content in modify role response for '{}'",
            role_name
        ))
    })?;

    let role: Role = serde_json::from_value(content.clone())
        .map_err(|e| ClientError::InvalidResponse(format!("Failed to parse role: {}", e)))?;

    Ok(attach_entry_name(entry_name, role))
}

/// Delete a role by name.
pub async fn delete_role(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    role_name: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<()> {
    let url = Url::parse(base_url)
        .map_err(|e| ClientError::InvalidUrl(format!("Invalid base URL: {}", e)))?
        .join(&format!("/services/authorization/roles/{}", role_name))
        .map_err(|e| ClientError::InvalidUrl(format!("Invalid role name: {}", e)))?;

    let builder = client
        .delete(url)
        .header("Authorization", format!("Bearer {}", auth_token));

    let _response = send_request_with_retry(
        builder,
        max_retries,
        &format!("/services/authorization/roles/{}", role_name),
        "DELETE",
        metrics,
    )
    .await?;

    Ok(())
}
