//! User management endpoints.

use reqwest::{Client, Url};
use secrecy::ExposeSecret;

use crate::endpoints::send_request_with_retry;
use crate::error::{ClientError, Result};
use crate::metrics::MetricsCollector;
use crate::models::{CreateUserParams, ModifyUserParams, User, UserListResponse};
use crate::name_merge::attach_entry_name;

/// List all users.
pub async fn list_users(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    count: Option<usize>,
    offset: Option<usize>,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<Vec<User>> {
    let url = format!("{}/services/authentication/users", base_url);

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
        "/services/authentication/users",
        "GET",
        metrics,
    )
    .await?;

    let resp: UserListResponse = response.json().await?;

    Ok(resp
        .entry
        .into_iter()
        .map(|e| attach_entry_name(e.name, e.content))
        .collect())
}

/// Create a new user.
pub async fn create_user(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    params: &CreateUserParams,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<User> {
    let url = format!("{}/services/authentication/users", base_url);

    let mut form_params: Vec<(String, String)> = vec![
        ("name".to_string(), params.name.clone()),
        ("output_mode".to_string(), "json".to_string()),
    ];

    // Add password (required for create)
    form_params.push((
        "password".to_string(),
        params.password.expose_secret().to_string(),
    ));

    // Add roles (comma-separated, required - at least one)
    if !params.roles.is_empty() {
        form_params.push(("roles".to_string(), params.roles.join(",")));
    }

    if let Some(ref realname) = params.realname {
        form_params.push(("realname".to_string(), realname.clone()));
    }
    if let Some(ref email) = params.email {
        form_params.push(("email".to_string(), email.clone()));
    }
    if let Some(ref default_app) = params.default_app {
        form_params.push(("defaultApp".to_string(), default_app.clone()));
    }

    let builder = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .form(&form_params);

    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/authentication/users",
        "POST",
        metrics,
    )
    .await?;

    let resp: serde_json::Value = response.json().await?;

    // Extract entry from response
    let entry = resp.get("entry").and_then(|e| e.get(0)).ok_or_else(|| {
        ClientError::InvalidResponse("Missing entry in create user response".to_string())
    })?;

    let entry_name = entry
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or(&params.name)
        .to_string();

    let content = entry.get("content").ok_or_else(|| {
        ClientError::InvalidResponse("Missing entry content in create user response".to_string())
    })?;

    let user: User = serde_json::from_value(content.clone())
        .map_err(|e| ClientError::InvalidResponse(format!("Failed to parse user: {}", e)))?;

    Ok(attach_entry_name(entry_name, user))
}

/// Modify an existing user.
pub async fn modify_user(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    user_name: &str,
    params: &ModifyUserParams,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<User> {
    let url = Url::parse(base_url)
        .map_err(|e| ClientError::InvalidUrl(format!("Invalid base URL: {}", e)))?
        .join(&format!("/services/authentication/users/{}", user_name))
        .map_err(|e| ClientError::InvalidUrl(format!("Invalid user name: {}", e)))?;

    let mut form_params: Vec<(String, String)> =
        vec![("output_mode".to_string(), "json".to_string())];

    // Add password if provided
    if let Some(ref password) = params.password {
        form_params.push(("password".to_string(), password.expose_secret().to_string()));
    }

    // Add roles if provided (comma-separated, replaces existing)
    if let Some(ref roles) = params.roles {
        form_params.push(("roles".to_string(), roles.join(",")));
    }

    if let Some(ref realname) = params.realname {
        form_params.push(("realname".to_string(), realname.clone()));
    }
    if let Some(ref email) = params.email {
        form_params.push(("email".to_string(), email.clone()));
    }
    if let Some(ref default_app) = params.default_app {
        form_params.push(("defaultApp".to_string(), default_app.clone()));
    }

    let builder = client
        .post(url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .form(&form_params);

    let response = send_request_with_retry(
        builder,
        max_retries,
        &format!("/services/authentication/users/{}", user_name),
        "POST",
        metrics,
    )
    .await?;

    let resp: serde_json::Value = response.json().await?;

    // Extract entry from response
    let entry = resp.get("entry").and_then(|e| e.get(0)).ok_or_else(|| {
        ClientError::InvalidResponse(format!(
            "Missing entry in modify user response for '{}'",
            user_name
        ))
    })?;

    let entry_name = entry
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or(user_name)
        .to_string();

    let content = entry.get("content").ok_or_else(|| {
        ClientError::InvalidResponse(format!(
            "Missing entry content in modify user response for '{}'",
            user_name
        ))
    })?;

    let user: User = serde_json::from_value(content.clone())
        .map_err(|e| ClientError::InvalidResponse(format!("Failed to parse user: {}", e)))?;

    Ok(attach_entry_name(entry_name, user))
}

/// Delete a user by name.
pub async fn delete_user(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    user_name: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<()> {
    let url = Url::parse(base_url)
        .map_err(|e| ClientError::InvalidUrl(format!("Invalid base URL: {}", e)))?
        .join(&format!("/services/authentication/users/{}", user_name))
        .map_err(|e| ClientError::InvalidUrl(format!("Invalid user name: {}", e)))?;

    let builder = client
        .delete(url)
        .header("Authorization", format!("Bearer {}", auth_token));

    let _response = send_request_with_retry(
        builder,
        max_retries,
        &format!("/services/authentication/users/{}", user_name),
        "DELETE",
        metrics,
    )
    .await?;

    Ok(())
}
