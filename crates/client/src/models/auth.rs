//! Authentication models for Splunk login API.
//!
//! This module contains types for authentication responses.
//! Note: Most auth logic is handled in the `auth` module; this contains
//! only the API response types.

/// Authentication response from login.
///
/// Note: This type is only used in tests. Production auth handling
/// uses different mechanisms in the `auth` module.
#[derive(Debug, Clone, serde::Deserialize)]
#[cfg(test)]
pub struct AuthResponse {
    #[serde(rename = "sessionKey")]
    pub session_key: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_auth_response() {
        let json = r#"{"sessionKey": "test-session-key"}"#;
        let resp: AuthResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.session_key, "test-session-key");
    }
}
