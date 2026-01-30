//! User models for Splunk user management API.
//!
//! This module contains types for listing and managing Splunk users.

use secrecy::SecretString;
use serde::{Deserialize, Serialize};

/// Splunk user information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    #[serde(default)]
    pub name: String,
    pub realname: Option<String>,
    pub email: Option<String>,
    #[serde(rename = "type")]
    pub user_type: Option<String>, // e.g., "Splunk", "SSO", etc.
    #[serde(rename = "defaultApp")]
    pub default_app: Option<String>,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(
        rename = "lastSuccessfulLogin",
        default,
        deserialize_with = "crate::serde_helpers::opt_u64_from_string_or_number"
    )]
    pub last_successful_login: Option<u64>, // Unix timestamp
}

/// User entry wrapper.
#[derive(Debug, Deserialize, Clone)]
pub struct UserEntry {
    pub name: String,
    pub content: User,
}

/// User list response.
#[derive(Debug, Deserialize, Clone)]
pub struct UserListResponse {
    pub entry: Vec<UserEntry>,
}

/// Parameters for creating a new user.
#[derive(Debug, Clone, Default)]
pub struct CreateUserParams {
    /// The username for the new user (required).
    pub name: String,
    /// The initial password for the user (required).
    pub password: SecretString,
    /// Roles to assign to the user (required, at least one).
    pub roles: Vec<String>,
    /// Real name of the user (optional).
    pub realname: Option<String>,
    /// Email address of the user (optional).
    pub email: Option<String>,
    /// Default app for the user (optional).
    pub default_app: Option<String>,
}

/// Parameters for modifying an existing user.
#[derive(Debug, Clone, Default)]
pub struct ModifyUserParams {
    /// New password for the user (optional).
    pub password: Option<SecretString>,
    /// Roles to assign to the user (replaces existing roles, optional).
    pub roles: Option<Vec<String>>,
    /// Real name of the user (optional).
    pub realname: Option<String>,
    /// Email address of the user (optional).
    pub email: Option<String>,
    /// Default app for the user (optional).
    pub default_app: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_user() {
        let json = r#"{
            "name": "admin",
            "realname": "Administrator",
            "email": "admin@example.com",
            "type": "Splunk",
            "defaultApp": "search",
            "roles": ["admin", "power"],
            "lastSuccessfulLogin": 1737712345
        }"#;
        let user: User = serde_json::from_str(json).unwrap();
        assert_eq!(user.name, "admin");
        assert_eq!(user.realname, Some("Administrator".to_string()));
        assert_eq!(user.email, Some("admin@example.com".to_string()));
        assert_eq!(user.user_type, Some("Splunk".to_string()));
        assert_eq!(user.default_app, Some("search".to_string()));
        assert_eq!(user.roles, vec!["admin", "power"]);
        assert_eq!(user.last_successful_login, Some(1737712345));
    }

    #[test]
    fn test_deserialize_user_with_optional_fields_missing() {
        let json = r#"{
            "name": "minimal_user",
            "roles": []
        }"#;
        let user: User = serde_json::from_str(json).unwrap();
        assert_eq!(user.name, "minimal_user");
        assert_eq!(user.realname, None);
        assert_eq!(user.email, None);
        assert_eq!(user.user_type, None);
        assert_eq!(user.default_app, None);
        assert!(user.roles.is_empty());
        assert_eq!(user.last_successful_login, None);
    }
}
