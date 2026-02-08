//! User models for Splunk user management API.
//!
//! This module contains types for listing and managing Splunk users.

use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Type of user authentication in Splunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum UserType {
    /// Native Splunk user.
    #[serde(rename = "Splunk")]
    Splunk,
    /// SSO authenticated user.
    #[serde(rename = "SSO")]
    Sso,
    /// LDAP authenticated user.
    #[serde(rename = "LDAP")]
    Ldap,
    /// SAML authenticated user.
    #[serde(rename = "SAML")]
    Saml,
    /// Unknown or unrecognized user type.
    #[serde(other)]
    #[default]
    Unknown,
}

impl fmt::Display for UserType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UserType::Splunk => write!(f, "Splunk"),
            UserType::Sso => write!(f, "SSO"),
            UserType::Ldap => write!(f, "LDAP"),
            UserType::Saml => write!(f, "SAML"),
            UserType::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Splunk user information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    #[serde(default)]
    pub name: String,
    pub realname: Option<String>,
    pub email: Option<String>,
    #[serde(rename = "type")]
    pub user_type: Option<UserType>,
    #[serde(rename = "defaultApp")]
    pub default_app: Option<String>,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(
        rename = "lastSuccessfulLogin",
        default,
        deserialize_with = "crate::serde_helpers::opt_usize_from_string_or_number"
    )]
    pub last_successful_login: Option<usize>, // Unix timestamp
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
        assert_eq!(user.user_type, Some(UserType::Splunk));
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

    #[test]
    fn test_deserialize_all_user_types() {
        // Test Splunk user type
        let json = r#"{"name": "user1", "type": "Splunk"}"#;
        let user: User = serde_json::from_str(json).unwrap();
        assert_eq!(user.user_type, Some(UserType::Splunk));

        // Test SSO user type
        let json = r#"{"name": "user2", "type": "SSO"}"#;
        let user: User = serde_json::from_str(json).unwrap();
        assert_eq!(user.user_type, Some(UserType::Sso));

        // Test LDAP user type
        let json = r#"{"name": "user3", "type": "LDAP"}"#;
        let user: User = serde_json::from_str(json).unwrap();
        assert_eq!(user.user_type, Some(UserType::Ldap));

        // Test SAML user type
        let json = r#"{"name": "user4", "type": "SAML"}"#;
        let user: User = serde_json::from_str(json).unwrap();
        assert_eq!(user.user_type, Some(UserType::Saml));
    }

    #[test]
    fn test_deserialize_unknown_user_type() {
        let json = r#"{"name": "user5", "type": "CustomAuth"}"#;
        let user: User = serde_json::from_str(json).unwrap();
        assert_eq!(user.user_type, Some(UserType::Unknown));
    }

    #[test]
    fn test_user_type_display() {
        assert_eq!(UserType::Splunk.to_string(), "Splunk");
        assert_eq!(UserType::Sso.to_string(), "SSO");
        assert_eq!(UserType::Ldap.to_string(), "LDAP");
        assert_eq!(UserType::Saml.to_string(), "SAML");
        assert_eq!(UserType::Unknown.to_string(), "Unknown");
    }

    #[test]
    fn test_user_type_default() {
        assert_eq!(UserType::default(), UserType::Unknown);
    }

    #[test]
    fn test_user_type_equality() {
        assert_eq!(UserType::Splunk, UserType::Splunk);
        assert_ne!(UserType::Splunk, UserType::Sso);
        assert_ne!(UserType::Unknown, UserType::Ldap);
    }

    #[test]
    fn test_user_type_clone() {
        let user_type = UserType::Saml;
        let cloned = user_type;
        assert_eq!(user_type, cloned);
    }

    #[test]
    fn test_user_type_serialize() {
        // Test serialization roundtrip
        let user_type = UserType::Ldap;
        let serialized = serde_json::to_string(&user_type).unwrap();
        assert_eq!(serialized, "\"LDAP\"");

        let deserialized: UserType = serde_json::from_str(&serialized).unwrap();
        assert_eq!(user_type, deserialized);
    }
}
