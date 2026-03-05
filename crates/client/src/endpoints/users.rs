//! User management endpoints.

use reqwest::{Client, Url};
use secrecy::ExposeSecret;

use crate::client::circuit_breaker::CircuitBreaker;
use crate::endpoints::encode_path_segment;
use crate::endpoints::send_request_with_retry;
use crate::error::{ClientError, Result};
use crate::form_params;
use crate::metrics::MetricsCollector;
use crate::models::{CreateUserParams, ModifyUserParams, User, UserListResponse};
use crate::name_merge::attach_entry_name;

/// List all users.
#[allow(clippy::too_many_arguments)]
pub async fn list_users(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    count: Option<usize>,
    offset: Option<usize>,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
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
        circuit_breaker,
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
#[allow(clippy::too_many_arguments)]
pub async fn create_user(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    params: &CreateUserParams,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<User> {
    let url = format!("{}/services/authentication/users", base_url);

    let mut form_params: Vec<(String, String)> =
        vec![("output_mode".to_string(), "json".to_string())];

    form_params! { form_params =>
        "name" => required_clone params.name,
        "password" => secret &params.password,
        "roles" => join params.roles,
        "realname" => ref params.realname,
        "email" => ref params.email,
        "defaultApp" => ref params.default_app,
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
        circuit_breaker,
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
#[allow(clippy::too_many_arguments)]
pub async fn modify_user(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    user_name: &str,
    params: &ModifyUserParams,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<User> {
    let encoded_user_name = encode_path_segment(user_name);
    let url = Url::parse(base_url)
        .map_err(|e| ClientError::InvalidUrl(format!("Invalid base URL: {}", e)))?
        .join(&format!(
            "/services/authentication/users/{}",
            encoded_user_name
        ))
        .map_err(|e| ClientError::InvalidUrl(format!("Invalid user name: {}", e)))?;

    let mut form_params: Vec<(String, String)> =
        vec![("output_mode".to_string(), "json".to_string())];

    form_params! { form_params =>
        "password" => secret_opt &params.password,
        "roles" => join_opt params.roles,
        "realname" => ref params.realname,
        "email" => ref params.email,
        "defaultApp" => ref params.default_app,
    }

    let builder = client
        .post(url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .form(&form_params);

    let response = send_request_with_retry(
        builder,
        max_retries,
        &format!("/services/authentication/users/{}", encoded_user_name),
        "POST",
        metrics,
        circuit_breaker,
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
#[allow(clippy::too_many_arguments)]
pub async fn delete_user(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    user_name: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<()> {
    let encoded_user_name = encode_path_segment(user_name);
    let url = Url::parse(base_url)
        .map_err(|e| ClientError::InvalidUrl(format!("Invalid base URL: {}", e)))?
        .join(&format!(
            "/services/authentication/users/{}",
            encoded_user_name
        ))
        .map_err(|e| ClientError::InvalidUrl(format!("Invalid user name: {}", e)))?;

    let builder = client
        .delete(url)
        .header("Authorization", format!("Bearer {}", auth_token));

    let _response = send_request_with_retry(
        builder,
        max_retries,
        &format!("/services/authentication/users/{}", encoded_user_name),
        "DELETE",
        metrics,
        circuit_breaker,
    )
    .await?;

    Ok(())
}
